use std::time::UNIX_EPOCH;

use gpui::SharedString;

use crate::{ui::widgets::file::FileData, utils::filekinds::FileKind};

pub struct FileMeta {
    pub kind: FileKind,
    pub size: SharedString,
    pub modified: String,
    pub created: String,
    pub permissions: String,
    pub executable: bool,
    pub symlink_target: Option<String>,
}

impl FileMeta {
    pub fn new(file: &FileData) -> Option<Self> {
        let path = std::path::Path::new(file.loc.as_ref());
        let meta = std::fs::symlink_metadata(path).ok()?;
        let is_dir = meta.is_dir();
        let is_symlink = meta.file_type().is_symlink();

        let symlink_target = if is_symlink {
            std::fs::read_link(path)
                .ok()
                .map(|t| t.to_string_lossy().into_owned())
        } else {
            None
        };

        let extension = if is_symlink || is_dir {
            None
        } else {
            path.extension()
                .map(|e| e.to_string_lossy().to_uppercase())
                .map(|e| e.to_owned())
        };

        let kind = FileKind::from_extension(extension.as_deref(), is_dir, is_symlink);
        let size = if is_dir {
            std::fs::read_dir(path)
                .map(|e| format!("{} items", e.filter_map(|e| e.ok()).count()))
                .unwrap_or_else(|_| "—".into())
        } else {
            format_size(meta.len())
        };

        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| format_timestamp(d.as_secs()))
            .unwrap_or_else(|| "—".into());

        let created = meta
            .created()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| format_timestamp(d.as_secs()))
            .unwrap_or_else(|| "—".into());

        let (permissions, executable) = {
            use std::os::unix::fs::PermissionsExt;
            let mode = meta.permissions().mode();
            (format_permissions(mode), !is_dir && (mode & 0o111 != 0))
        };

        Some(Self {
            kind,
            size: size.into(),
            modified,
            created,
            permissions,
            executable,
            symlink_target,
        })
    }
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{bytes} B")
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}

fn format_timestamp(secs: u64) -> String {
    // Simple formatting without chrono — compute y/m/d from epoch seconds
    let days_since_epoch = secs / 86400;
    let seconds_in_day = secs % 86400;
    let hours = seconds_in_day / 3600;
    let minutes = (seconds_in_day % 3600) / 60;

    // Rata Die algorithm for Gregorian calendar
    let z = days_since_epoch as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{:02}.{:02}.{:04} {:02}:{:02}", d, m, y, hours, minutes)
}

fn format_permissions(mode: u32) -> String {
    let chars = [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ];
    let s: String = chars
        .iter()
        .map(|&(bit, ch)| if mode & bit != 0 { ch } else { '-' })
        .collect();
    format!("{s} ({:04o})", mode & 0o777)
}
