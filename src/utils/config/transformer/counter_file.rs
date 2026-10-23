use std::collections::HashMap;
use std::path::Path;

use crate::{
    loader::application_loader::ApplicationLoader,
    ui::query_memory::{PersistentQueryMemory, today},
    utils::{cache::BinaryCache, item_key::ItemKey, paths::query_memory_path},
};

/// Ports the legacy `counts.bin` (exec → launch count) into the query memory's
/// global popularity. Apps only: their old key was the desktop file's `Exec`
/// (after aliases), which maps to the new `ItemKey` via the desktop file.
///
/// Reads both historic formats (`u16` and the older `u32` values). On success
/// the file is renamed to `counts.bin.migrated`, so this runs once.
pub fn transform_counter_file(path: &Path) {
    if !path.exists() {
        return; // fresh install or already migrated: nothing to do, no need to parse apps
    }

    // Parse desktop files exactly like the launcher does, aliases applied,
    // so `exec` is the same string the old counter used as its key.
    let apps = match ApplicationLoader::load_applications(false, None) {
        Ok(apps) => apps,
        Err(e) => {
            eprintln!(
                "[Counter File]: Failed to load applications. Caused by: {:?}",
                e.traceback
            );
            return;
        }
    };
    let exec_to_key: HashMap<&str, ItemKey> = apps
        .iter()
        .filter_map(|a| {
            Some((
                a.payload.exec.as_str(),
                ItemKey::app(&a.payload.desktop_file)?,
            ))
        })
        .collect();

    let Ok(mem_path) = query_memory_path() else {
        eprintln!("[Counter File]: Could not resolve query memory path, skipping");
        return;
    };

    if let Some(ported) = port_counts(path, PersistentQueryMemory::load(mem_path), &exec_to_key) {
        eprintln!(
            "[Counter File]: Ported {} of {} entries",
            ported.matched, ported.total
        );
    }
}

/// Result of a successful port.
#[derive(Debug, PartialEq, Eq)]
struct Ported {
    matched: usize,
    total: usize,
}

/// IO glue: read the old file, import, save, retire the old file.
/// `None` means nothing was ported and `counts_path` is left untouched.
fn port_counts(
    counts_path: &Path,
    mut mem: PersistentQueryMemory,
    exec_to_key: &HashMap<&str, ItemKey>,
) -> Option<Ported> {
    let bytes = match std::fs::read(counts_path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            eprintln!("[Counter File]: Failed to read counter file: {e}");
            return None;
        }
    };
    let Some(legacy) = decode_counter_file(&bytes) else {
        eprintln!("[Counter File]: Failed to parse counter file, leaving it in place");
        return None;
    };

    let matched = import_counts(&mut mem, &legacy, exec_to_key, today());

    // persist first; only then retire the old file
    if let Err(e) = mem.save_atomic() {
        eprintln!(
            "[Counter File]: Failed to write query memory. Caused by: {:?}",
            e.traceback
        );
        return None;
    }
    if let Err(e) = std::fs::rename(counts_path, counts_path.with_extension("bin.migrated")) {
        eprintln!("[Counter File]: Ported, but could not rename old file: {e}");
    }
    Some(Ported {
        matched,
        total: legacy.len(),
    })
}

/// Decodes `counts.bin` bytes in either historic format.
fn decode_counter_file(bytes: &[u8]) -> Option<HashMap<String, u32>> {
    if let Ok(map) = BinaryCache::decode::<HashMap<String, u16>>(bytes) {
        return Some(map.into_iter().map(|(k, v)| (k, v as u32)).collect());
    }
    BinaryCache::decode::<HashMap<String, u32>>(bytes).ok()
}

/// Imports every legacy count whose exec maps to a known app (no IO).
/// Returns how many entries matched.
fn import_counts(
    mem: &mut PersistentQueryMemory,
    legacy: &HashMap<String, u32>,
    exec_to_key: &HashMap<&str, ItemKey>,
    today: u32,
) -> usize {
    let mut matched = 0;
    for (exec, count) in legacy {
        if let Some(key) = exec_to_key.get(exec.as_str()) {
            mem.import(key, *count as f32, today);
            matched += 1;
        }
    }
    matched
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    fn app(id: &str) -> ItemKey {
        ItemKey::app(Path::new(&format!("/usr/share/applications/{id}"))).unwrap()
    }

    fn legacy<V: Serialize + Copy>(entries: &[(&str, V)]) -> Vec<u8> {
        let map: HashMap<String, V> = entries.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        BinaryCache::encode(&map).unwrap()
    }

    fn mapping() -> HashMap<&'static str, ItemKey> {
        HashMap::from([
            ("firefox %u", app("firefox.desktop")),
            ("kitty", app("kitty.desktop")),
        ])
    }

    #[track_caller]
    fn approx(a: f32, b: f32) {
        assert!((a - b).abs() < 1e-5, "{a} != {b}");
    }

    #[test]
    fn decodes_u16_format() {
        let bytes = legacy(&[("kitty", 7u16)]);
        assert_eq!(
            decode_counter_file(&bytes),
            Some(HashMap::from([("kitty".into(), 7)]))
        );
    }

    #[test]
    fn decodes_legacy_u32_format_without_truncating() {
        let bytes = legacy(&[("kitty", 70_000u32)]); // > u16::MAX
        assert_eq!(
            decode_counter_file(&bytes),
            Some(HashMap::from([("kitty".into(), 70_000)]))
        );
    }

    #[test]
    fn garbage_decodes_as_none() {
        assert_eq!(decode_counter_file(b"not a counter file"), None);
    }

    #[test]
    fn imports_known_apps_and_skips_the_rest() {
        let mut mem = PersistentQueryMemory::in_memory();
        let legacy = HashMap::from([
            ("firefox %u".into(), 12),
            ("kitty".into(), 3),
            ("websearch-google".into(), 9), // not an app: skipped
        ]);

        assert_eq!(import_counts(&mut mem, &legacy, &mapping(), 0), 2);
        approx(mem.inner().popularity_of(&app("firefox.desktop"), 0), 12.0);
        approx(mem.inner().popularity_of(&app("kitty.desktop"), 0), 3.0);
    }

    #[test]
    fn adds_to_existing_memory() {
        let mut mem = PersistentQueryMemory::in_memory();
        mem.inner_mut().record("kit", &app("kitty.desktop"), 0);
        let legacy = HashMap::from([("kitty".into(), 3)]);

        import_counts(&mut mem, &legacy, &mapping(), 0);
        approx(mem.inner().popularity_of(&app("kitty.desktop"), 0), 4.0); // 1 pick + 3 imported
    }

    #[test]
    fn zero_counts_are_ignored() {
        let mut mem = PersistentQueryMemory::in_memory();
        let legacy = HashMap::from([("kitty".into(), 0)]);

        assert_eq!(import_counts(&mut mem, &legacy, &mapping(), 0), 1); // matched …
        assert!(mem.inner().is_empty()); // … but `import` skips zero, nothing stored
    }

    #[test]
    fn imported_counts_decay_from_today() {
        let mut mem = PersistentQueryMemory::in_memory();
        let legacy = HashMap::from([("kitty".into(), 4)]);

        import_counts(&mut mem, &legacy, &mapping(), 100);
        approx(mem.inner().popularity_of(&app("kitty.desktop"), 100), 4.0);
        approx(mem.inner().popularity_of(&app("kitty.desktop"), 130), 2.0); // one half-life later
    }
}
