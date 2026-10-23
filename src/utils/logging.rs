use std::io::Write;
use std::sync::LazyLock;
use std::{fs, fs::OpenOptions, sync::Mutex};

use chrono::Local;

use crate::utils::{errors::SherlockMessage, paths};

// Must not log on failure: `sherlock_msg!` calls `write_log`, which would
// re-enter this LazyLock while it's initializing and deadlock (e.g. in the
// Nix build sandbox, where the cache dir can't be created).
static LOG_FILE: LazyLock<Option<Mutex<std::fs::File>>> = LazyLock::new(|| {
    // Not paths::get_cache_dir(): it logs on error too.
    let cache_dir = paths::get_xdg_dirs().get_cache_home()?;
    fs::create_dir_all(&cache_dir).ok()?;
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(cache_dir.join("sherlock.log"))
        .ok()?;
    Some(Mutex::new(file))
});

pub fn write_log<T: AsRef<str>>(message: T, file: &str, line: u32) -> Result<(), SherlockMessage> {
    let message = message.as_ref();
    let now = Local::now().format("%Y-%m-%d %H:%M:%S");
    // No log file available: drop the message rather than erroring, since
    // building an error here would recurse back into write_log.
    let Some(log_file) = LOG_FILE.as_ref() else {
        return Ok(());
    };
    let mut log_file = log_file.lock().expect("Failed to lock LOG_FILE..");

    message
        .split("\n")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .for_each(|msg| {
            writeln!(log_file, "[{}] {}:{} - {}", now, file, line, msg)
                .expect("Failed to write to log file");
        });

    Ok(())
}

#[macro_export]
macro_rules! sher_log {
    // With message only — uses current file!() and line!()
    ($message:expr) => {
        $crate::utils::logging::write_log($message, file!(), line!())
    };

    // With message, file and line explicitly passed
    ($message:expr, $file:expr, $line:expr) => {
        $crate::utils::logging::write_log($message, $file, $line)
    };
}
