use std::{iter::Peekable, slice::Iter};

use super::{
    DebugAction,
    actions::{StartupAction, flag_documentation, init_config, print_version},
    flags::FLAGS,
    utils::{FlagSection, ParseError},
};
#[cfg(feature = "docs")]
use crate::docs::SherlockDocumentation;
use crate::{
    launcher::debug_launcher::DebugFunctions,
    loader::flag_loader::actions::plugin_init,
    utils::config::{SherlockFlags, repair_config},
};

pub struct ParsedArgs {
    pub flags: SherlockFlags,
    pub startup: Option<StartupAction>,
}
impl ParsedArgs {
    /// Executes debug functions (e.g print flag help or init configs).
    /// # Returns:
    /// - `true` if the program should exit after the execution
    /// - `false` if the program can continue running after this execution
    pub fn execute_startup(&mut self) -> bool {
        if !self
            .startup
            .as_ref()
            .is_some_and(|a| matches!(a, StartupAction::Debug(_)))
        {
            return false;
        }

        let Some(StartupAction::Debug(action)) = self.startup.take() else {
            return false;
        };

        match action {
            DebugAction::Help => flag_documentation(),
            DebugAction::Version => print_version(),
            DebugAction::Repair => repair_config(&mut self.flags),
            DebugAction::ClearCache => {
                if let Err(e) = DebugFunctions::clear_cache() {
                    eprintln!("{:?}", e);
                }
            }
            DebugAction::Init { path, extension } => init_config(&path, &extension),
            DebugAction::PluginInit => plugin_init(),
            #[cfg(feature = "docs")]
            DebugAction::GenerateDocs => SherlockDocumentation::generate(),
        }

        true
    }
}

pub struct ArgParser {
    dev_mode: bool,
}

impl ArgParser {
    pub fn from_env() -> Option<ParsedArgs> {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let dev_mode = std::env::var("SHERLOCK_DEV").is_ok();
        Self { dev_mode }.parse(&args)
    }

    fn parse(&self, args: &[String]) -> Option<ParsedArgs> {
        let mut startup: Option<StartupAction> = None;
        let mut flags = SherlockFlags::default();
        let mut iter: Peekable<Iter<'_, String>> = args.iter().peekable();

        while let Some(arg) = iter.next() {
            let Some(spec) = FLAGS
                .iter()
                .find(|s| arg == s.long || s.short.is_some_and(|sh| arg == sh))
            else {
                eprintln!("{}", ParseError::UnknownFlag(arg));
                return None;
            };

            if matches!(spec.section, FlagSection::None) && !self.dev_mode {
                eprintln!("{}", ParseError::UnknownFlag(arg));
                return None;
            }

            if let Err(e) = (spec.parse)(args, &mut iter, &mut flags, &mut startup) {
                eprintln!("{e}");
            }
        }

        Some(ParsedArgs { flags, startup })
    }
}
