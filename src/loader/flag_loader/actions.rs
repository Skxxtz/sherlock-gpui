use std::{env, fs, path::Path};

use crate::{
    launcher::plugin_launcher::api::LuaApiDocumentation,
    loader::flag_loader::{DebugAction, flags::FLAGS, utils::FlagSection},
    sherlock_msg,
    tokio_utils::SizedMessageObj,
    utils::{
        config::SherlockConfig,
        errors::{SherlockMessage, types::SherlockErrorType},
        networking::ClientMessage,
        paths::get_cache_dir,
    },
};

#[derive(PartialEq)]
pub enum StartupAction {
    Debug(DebugAction),
    Server { msg: ClientMessage, exit: bool },
}

impl From<DebugAction> for StartupAction {
    fn from(value: DebugAction) -> Self {
        Self::Debug(value)
    }
}
impl From<ClientMessage> for StartupAction {
    fn from(value: ClientMessage) -> Self {
        Self::Server {
            msg: value,
            exit: true,
        }
    }
}

impl TryFrom<&StartupAction> for SizedMessageObj {
    type Error = SherlockMessage;
    fn try_from(value: &StartupAction) -> Result<Self, Self::Error> {
        match value {
            StartupAction::Debug(_) => Err(sherlock_msg!(
                Error,
                SherlockErrorType::Unreachable,
                "Tried to use `StartupAction::Debug` as a `SizedMessageObj`"
            )),
            StartupAction::Server { msg, .. } => SizedMessageObj::from_struct(msg),
        }
    }
}

#[allow(unused)]
impl StartupAction {
    pub fn exit(&self) -> bool {
        match self {
            Self::Server { exit, .. } => *exit,
            _ => true,
        }
    }
    pub fn with_exit(mut self, exit: bool) -> Self {
        if let Self::Server {
            exit: ref mut internal_exit,
            ..
        } = self
        {
            *internal_exit = exit;
        }
        self
    }
}

pub(super) fn init_config(path: &Path, extension: &str) {
    if let Err(e) = SherlockConfig::to_file(path, extension) {
        eprintln!("{:?}", e)
    }
}

pub(super) fn plugin_init() {
    let dir = match env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("error: could not determine current directory: {e}");
            return;
        }
    };

    let cache_root = match get_cache_dir() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {:?}", e);
            return;
        }
    };
    let meta_dir = cache_root.join("meta").join("sherlock");
    if let Err(e) = fs::create_dir_all(&meta_dir) {
        eprintln!("error: failed to create {}: {e}", meta_dir.display());
        return;
    }

    // purely generated -- always overwrite, no need to check existence
    let init_path = meta_dir.join("init.lua");
    let api_stub = LuaApiDocumentation::generate_lua_stub();
    if let Err(e) = fs::write(&init_path, api_stub) {
        eprintln!("error: failed to write {}: {e}", init_path.display());
        return;
    }
    println!("wrote {}", init_path.display());

    let ui_path = meta_dir.join("ui.lua");
    let ui_source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/launcher/plugin_launcher/api/assets/ui.lua"
    ));
    if let Err(e) = fs::write(&ui_path, ui_source) {
        eprintln!("error: failed to write {}: {e}", ui_path.display());
        return;
    }
    println!("wrote {}", ui_path.display());

    let luarc_path = dir.join(".luarc.json");
    let library_dir = meta_dir.parent().unwrap();

    if luarc_path.exists() {
        let wired_up = fs::read_to_string(&luarc_path)
            .map(|contents| contents.contains(library_dir.to_string_lossy().as_ref()))
            .unwrap_or(false);
        if wired_up {
            println!(
                "{} already references {}",
                luarc_path.display(),
                library_dir.display()
            );
        } else {
            println!(
                "skipped {} (already exists) — add \"{}\" to workspace.library manually",
                luarc_path.display(),
                library_dir.display()
            );
        }
    } else {
        let luarc = serde_json::json!({
            "workspace.library": [library_dir.to_string_lossy()],
            "diagnostics.globals": ["sherlock"]
        });
        let luarc_str = match serde_json::to_string_pretty(&luarc) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: failed to serialize .luarc.json: {e}");
                return;
            }
        };
        if let Err(e) = fs::write(&luarc_path, luarc_str) {
            eprintln!("error: failed to write {}: {e}", luarc_path.display());
            return;
        }
        println!("wrote {}", luarc_path.display());
    }
}

pub(super) fn print_version() {
    let version = env!("CARGO_PKG_VERSION");
    println!("Sherlock v{}", version);
    println!("Developed by Skxxtz and Sherlock's awesome community.");
}

pub(super) fn flag_documentation() {
    let longest = FLAGS
        .iter()
        .map(|f| f.long.len() + f.short.map_or(0, |s| s.len() + 2))
        .max()
        .unwrap_or(20)
        + 4;

    let mut current_section = FlagSection::None;
    for spec in FLAGS {
        if spec.section == FlagSection::None {
            continue;
        }

        if spec.section != current_section {
            current_section = spec.section;
            println!("\n{current_section}:");
        }
        let flag_str = match spec.short {
            Some(s) => format!("{}, {}", s, spec.long),
            None => spec.long.to_string(),
        };
        println!("  {:<width$} {}", flag_str, spec.help, width = longest);
    }

    println!(
        "\n\nFor more help:\nhttps://github.com/Skxxtz/sherlock/blob/documentation/docs/flags.md\n"
    );
}
