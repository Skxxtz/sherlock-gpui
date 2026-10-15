use gpui::SharedString;
use procfs::ProcResult;

use crate::loader::{IconType, find_desktop_icon, resolve_icon_path};

use super::ProcessData;

pub struct ProcessMeta {
    pub icon: Option<IconType>,
    pub exe: SharedString,
    pub memory_mb: f32,
    pub mem_peak: f32,
    pub threads: u32,
    pub pid: i32,
    pub ppid: i32,
}

impl ProcessMeta {
    pub fn new(value: &ProcessData) -> ProcResult<Self> {
        let process = procfs::process::Process::new(value.ppid)?;
        let status = process.status()?;
        let exe: SharedString = process.exe()?.to_string_lossy().to_string().into();

        let memory_mb = status.vmrss.unwrap_or(0) as f32 / 1024.0;
        let mem_peak = status.vmhwm.unwrap_or(0) as f32 / 1024.0;
        let threads = status.threads as u32;

        let icon_name = find_desktop_icon(exe.as_str());

        Ok(Self {
            pid: value.pid,
            ppid: value.ppid,
            memory_mb,
            mem_peak,
            threads,
            exe,
            icon: icon_name
                .as_ref()
                .and_then(|name| resolve_icon_path(name))
                .or(resolve_icon_path("sherlock-process")),
        })
    }
}
