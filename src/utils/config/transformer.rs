mod counter_file;
mod fallback_migration;

use std::{fs, path::Path};

use crate::{
    sherlock_msg,
    utils::{
        config::{
            SherlockConfig,
            SherlockFlags,
            transformer::fallback_migration::LegacyRawLauncher,
        },
        errors::{
            SherlockMessage,
            types::{FileAction, SherlockErrorType},
        },
        paths,
    },
};

pub fn repair_config(flags: &mut SherlockFlags) {
    // parse configs
    let config = match flags.get_config() {
        Err(_) => {
            let mut defaults = SherlockConfig::default();
            defaults.apply_flags(flags);
            defaults
        }
        Ok((cfg, _)) => cfg,
    };

    // repair broken fallback.json
    let _ = migrate_fallback(&config.files.fallback);

    // repair broken counts file
    match paths::get_data_dir() {
        Ok(data_dir) => {
            let count_path = data_dir.join("counts.bin");
            println!("--- Migration Logs for {} ---", count_path.display());
            migrate_counts(&count_path);
        }
        Err(e) => {
            eprintln!("[Counter File]: Failed to get sherlock data dir: {:?}", e);
        }
    }
}

#[allow(dead_code)]
pub fn migrate_fallback<P: AsRef<Path>>(path: P) -> Result<(), SherlockMessage> {
    let path_ref = path.as_ref();
    let content = fs::read_to_string(path_ref).map_err(|e| {
        sherlock_msg!(
            Warning,
            SherlockErrorType::FileError(FileAction::Read, path_ref.to_path_buf()),
            e
        )
    })?;

    let legacy_configs: Vec<LegacyRawLauncher> = serde_json::from_str(&content).map_err(|e| {
        sherlock_msg!(
            Warning,
            SherlockErrorType::DeserializationError("Legacy Launcher".into()),
            format!("File is neither modern nor legacy format: {e}")
        )
    })?;

    let mut upgraded_launchers = Vec::new();
    let mut all_logs = Vec::new();

    for legacy in legacy_configs {
        match legacy.migrate() {
            Ok(result) => {
                upgraded_launchers.push(result.launcher);
                all_logs.extend(result.logs);
            }
            Err(e) => {
                all_logs.extend(e);
            }
        }
    }

    // 4. Print migration audit trail
    if !all_logs.is_empty() {
        println!("--- Migration Logs for {} ---", path_ref.display());
        for log in all_logs {
            println!("  • {}", log);
        }
    }

    // 5. Save the upgraded version back to the file
    let new_json = serde_json::to_string_pretty(&upgraded_launchers)
        .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::SerializationError, e))?;
    fs::write(path_ref, new_json).map_err(|e| {
        sherlock_msg!(
            Warning,
            SherlockErrorType::FileError(FileAction::Write, path_ref.to_path_buf()),
            e
        )
    })?;

    println!(
        "[{}] Successfully migrated to new format.",
        path_ref.display()
    );

    Ok(())
}

#[allow(dead_code)]
pub fn migrate_counts(path: &Path) {
    counter_file::transform_counter_file(path);
}
