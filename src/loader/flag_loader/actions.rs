use std::{env, fs, path::Path};

use indoc::indoc;

use crate::{
    launcher::plugin_launcher::api::LuaApiDocumentation,
    loader::flag_loader::{DebugAction, flags::FLAGS, utils::FlagSection},
    sherlock_msg,
    tokio_utils::SizedMessageObj,
    utils::{
        config::SherlockConfig,
        errors::{SherlockMessage, types::SherlockErrorType},
        networking::ClientMessage,
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

    let api_path = dir.join("sherlock-api.lua");
    let luarc_path = dir.join(".luarc.json");

    let api = LuaApiDocumentation::generate_lua_stub();
    let ui_source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/launcher/plugin_launcher/api/assets/ui.lua"
    ));
    let combined = format!("{api}\n{ui_source}");
    if let Err(e) = fs::write(&api_path, combined) {
        eprintln!("error: failed to write {}: {e}", api_path.display());
        return;
    }
    println!("wrote {}", api_path.display());

    if luarc_path.exists() {
        println!(
            "skipped {} (already exists) — add \"./sherlock-api.lua\" to workspace.library manually if needed",
            luarc_path.display()
        );
    } else {
        let luarc = indoc! {r#"
            {
              "workspace.library": ["./sherlock-api.lua"]
            }"#};
        if let Err(e) = fs::write(&luarc_path, luarc) {
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
