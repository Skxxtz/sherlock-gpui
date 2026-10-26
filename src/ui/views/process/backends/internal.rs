use std::{
    collections::{HashMap, hash_map::Entry},
    sync::Arc,
};

use procfs::process::all_processes;
use rayon::iter::{ParallelBridge, ParallelIterator};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::ui::views::process::{
    backends::ProcessSearchProvider,
    utils::{PathSearchUtils, ProcessResult, ResultHeap},
};
use crate::utils::string::CiUtils;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct InternalBackend;

impl ProcessSearchProvider for InternalBackend {
    fn name(&self) -> &'static str {
        "internal"
    }
    fn search(
        &self,
        query: Arc<str>,
        heap: &mut ResultHeap,
        mut cancel_rx: mpsc::Receiver<()>,
    ) -> bool {
        let Ok(all_procs) = all_processes() else {
            return true;
        };

        let procs = all_procs
            .par_bridge()
            .filter_map(Result::ok)
            .filter(|p| p.uid().is_ok_and(|uid| uid > 0))
            .filter_map(|p| {
                let stat = p.stat().ok()?;
                let exe = p.exe().ok()?;
                let exe_str = exe.to_str()?;
                let name = exe.file_name()?.to_str().map(str::to_string)?;
                if !is_app_process(exe_str, &name) {
                    return None;
                }
                Some((p.pid, Some((name, exe_str.to_string())), stat))
            })
            .collect::<Vec<_>>();

        let mut process_names: HashMap<i32, (String, String)> = procs
            .iter()
            .filter_map(|(pid, info, _)| info.clone().map(|n| (*pid, n)))
            .collect();

        let mut tmp: HashMap<i32, i32> = HashMap::new();

        for (pid, _, stat) in procs.into_iter().rev() {
            if cancel_rx.try_recv().is_ok() || cancel_rx.is_closed() {
                return false;
            }

            let entry = if stat.ppid == 1 {
                let named_id = tmp.get(&pid).copied().unwrap_or(pid);
                process_names
                    .remove(&named_id)
                    .map(|info| ((pid, named_id), info))
            } else if stat.tty_nr != 0 {
                if let Some(r) = tmp.remove(&pid) {
                    tmp.insert(stat.ppid, r);
                } else {
                    tmp.entry(stat.ppid).or_insert(pid);
                }
                process_names
                    .remove(&pid)
                    .map(|(name, exe)| ((pid, pid), (name, exe)))
            } else if let Entry::Vacant(e) = tmp.entry(stat.ppid) {
                e.insert(pid);
                None
            } else {
                None
            };

            let Some(((ppid, pid), (name, _exe))) = entry else {
                continue;
            };

            let name_bytes = name.as_bytes();
            if !CiUtils::bytes_contain_ci(name_bytes, query.as_bytes()) {
                continue;
            }
            let score = PathSearchUtils::score_ci(name_bytes, &query);
            heap.push(ProcessResult {
                name: name.into(),
                pid,
                ppid,
                score,
            });
        }

        true
    }
}

fn is_app_process(exe: &str, name: &str) -> bool {
    // Reject kernel threads and pseudo-processes
    if exe.is_empty() {
        return false;
    }

    // Reject system library executables
    const SYSTEM_PREFIXES: &[&str] = &["/usr/lib", "/usr/libexec", "/lib", "/usr/sbin", "/sbin"];
    if SYSTEM_PREFIXES.iter().any(|p| exe.starts_with(p)) {
        return false;
    }

    // Reject known noisy system process names
    const SYSTEM_NAMES: &[&str] = &[
        "dbus-daemon",
        "dbus-broker",
        "systemd",
        "sd-pam",
        "pipewire",
        "pipewire-pulse",
        "wireplumber",
        "at-spi",
        "gvfsd",
        "xdg-",
        "gdm",
        "lightdm",
        "sh",
        "bash",
        "zsh",
        "fish",
        "python",
        "python3",
        "ruby",
        "perl",
    ];
    if SYSTEM_NAMES.iter().any(|s| name.starts_with(s)) {
        return false;
    }

    true
}
