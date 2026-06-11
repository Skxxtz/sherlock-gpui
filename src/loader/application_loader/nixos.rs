use std::time::SystemTime;

/// This function checks the metadata of the NixOS-specific `/run/current-system` symlink.
/// The symlink is created on every NixOs rebuild. Therefore, this corresponds to the creation
/// timestamp of the current system.
pub fn system_creation_time() -> Option<SystemTime> {
    std::fs::symlink_metadata("/run/current-system")
        .ok()
        .and_then(|m| m.modified().ok())
}
