use std::{iter::Peekable, path::PathBuf, slice::Iter};

use gpui::SharedString;

use crate::{
    loader::flag_loader::{DebugAction, actions::StartupAction, utils::ParseError},
    utils::{config::SherlockFlags, networking::ClientMessage},
};

use super::utils::FlagSection;

type FlagParserFn<'a> = fn(
    &[String],
    &mut Peekable<Iter<'_, String>>,
    &mut SherlockFlags,
    &mut Option<StartupAction>,
) -> Result<(), ParseError<'a>>;

pub struct FlagSpec<'a> {
    pub long: &'static str,
    pub short: Option<&'static str>,
    pub section: FlagSection,
    pub help: &'static str,
    pub parse: FlagParserFn<'a>,
}

pub const FLAGS: &[FlagSpec] = &[
    // Basics
    FlagSpec {
        long: "--help",
        short: Some("-h"),
        section: FlagSection::Basics,
        help: "Show this help message.",
        parse: |_args, _iter, _flags, startup| {
            if startup.is_none() {
                *startup = Some(DebugAction::Help.into());
            }
            Ok(())
        },
    },
    FlagSpec {
        long: "--version",
        short: Some("-v"),
        section: FlagSection::Basics,
        help: "Print the version.",
        parse: |_args, _iter, _flags, startup| {
            if startup.is_none() {
                *startup = Some(DebugAction::Version.into());
            }
            Ok(())
        },
    },
    FlagSpec {
        long: "init",
        short: None,
        section: FlagSection::Basics,
        help: "Write default configs to path (default: ~/.config/sherlock/).",
        parse: |args, iter, _flags, startup| {
            if startup.is_none() {
                let path = iter
                    .next()
                    .filter(|a| !a.starts_with('-'))
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("~/.config/sherlock/"));

                let extension = args
                    .windows(2)
                    .find(|w| w[0] == "--file-type" || w[0] == "-f")
                    .map(|w| w[1].clone())
                    .unwrap_or_else(|| "toml".into());

                *startup = Some(DebugAction::Init { path, extension }.into());
            }
            Ok(())
        },
    },
    FlagSpec {
        long: "repair",
        short: None,
        section: FlagSection::Basics,
        help: "Repair config files.",
        parse: |_args, _iter, _flags, startup| {
            if startup.is_none() {
                *startup = Some(DebugAction::Repair.into());
            }
            Ok(())
        },
    },
    FlagSpec {
        long: "clear-cache",
        short: None,
        section: FlagSection::Basics,
        help: "Clear Sherlock's cache.",
        parse: |_args, _iter, _flags, startup| {
            if startup.is_none() {
                *startup = Some(DebugAction::ClearCache.into());
            }
            Ok(())
        },
    },
    FlagSpec {
        long: "--generate-docs",
        short: None,
        section: FlagSection::None,
        help: "Generate docs (dev only).",
        parse: |_args, _iter, _flags, startup| {
            if startup.is_none()
                && std::env::var("SHERLOCK_DEV").is_ok_and(|var| var.eq_ignore_ascii_case("true"))
            {
                *startup = Some(DebugAction::GenerateDocs.into());
            }
            Ok(())
        },
    },
    // Files
    FlagSpec {
        long: "--config-dir",
        short: None,
        section: FlagSection::Files,
        help: "Directory Sherlock looks for configuration in.",
        parse: |_args, _iter, _flags, startup| {
            if startup.is_none()
                && std::env::var("SHERLOCK_DEV").is_ok_and(|var| var.eq_ignore_ascii_case("true"))
            {
                *startup = Some(DebugAction::Version.into());
            }
            Ok(())
        },
    },
    FlagSpec {
        long: "--config",
        short: None,
        section: FlagSection::Files,
        help: "Configuration file to load.",
        parse: |_args, iter, flags, _startup| {
            flags.config = iter.next().map(PathBuf::from);
            Ok(())
        },
    },
    FlagSpec {
        long: "--fallback",
        short: None,
        section: FlagSection::Files,
        help: "Fallback file to load.",
        parse: |_args, iter, flags, _startup| {
            flags.fallback = iter.next().map(PathBuf::from);
            Ok(())
        },
    },
    FlagSpec {
        long: "--ignore",
        short: None,
        section: FlagSection::Files,
        help: "Sherlock ignore file.",
        parse: |_args, iter, flags, _startup| {
            flags.ignore = iter.next().map(PathBuf::from);
            Ok(())
        },
    },
    FlagSpec {
        long: "--alias",
        short: None,
        section: FlagSection::Files,
        help: "Sherlock alias file (.json).",
        parse: |_args, iter, flags, _startup| {
            flags.alias = iter.next().map(PathBuf::from);
            Ok(())
        },
    },
    FlagSpec {
        long: "--cache",
        short: None,
        section: FlagSection::Files,
        help: "Sherlock cache file (.json).",
        parse: |_args, iter, flags, _startup| {
            flags.cache = iter.next().map(PathBuf::from);
            Ok(())
        },
    },
    // Behaviour
    FlagSpec {
        long: "--placeholder",
        short: Some("-p"),
        section: FlagSection::Behavior,
        help: "Overwrite placeholder text of the search bar.",
        parse: |_args, iter, flags, _startup| {
            flags.placeholder = iter.next().map(Into::into);
            Ok(())
        },
    },
    FlagSpec {
        long: "--sub-menu",
        short: Some("-sm"),
        section: FlagSection::Behavior,
        help: "Start with an alias active (e.g. 'pm' for power menu).",
        parse: |_args, iter, flags, _startup| {
            flags.sub_menu = iter.next().map(Into::into);
            Ok(())
        },
    },
    FlagSpec {
        long: "--multi",
        short: None,
        section: FlagSection::Behavior,
        help: "Enable multi-select mode.",
        parse: |_args, _iter, flags, _startup| {
            flags.multi = true;
            Ok(())
        },
    },
    FlagSpec {
        long: "--photo",
        short: None,
        section: FlagSection::Behavior,
        help: "Disable close-on-focus-loss.",
        parse: |_args, _iter, flags, _startup| {
            flags.photo_mode = true;
            Ok(())
        },
    },
    FlagSpec {
        long: "--wait",
        short: Some("-w"),
        section: FlagSection::Behavior,
        help: "Wait mode.",
        parse: |_args, _iter, flags, _startup| {
            flags.wait = true;
            Ok(())
        },
    },
    FlagSpec {
        long: "--center",
        short: None,
        section: FlagSection::Behavior,
        help: "Center raw display.",
        parse: |_args, _iter, flags, _startup| {
            flags.center_raw = true;
            Ok(())
        },
    },
    FlagSpec {
        long: "--display-raw",
        short: None,
        section: FlagSection::Behavior,
        help: "Use singular tile for piped content.",
        parse: |_args, _iter, flags, _startup| {
            flags.display_raw = true;
            Ok(())
        },
    },
    // Sherlock Functions
    FlagSpec {
        long: "new-timer",
        short: None,
        section: FlagSection::Functions,
        help: "Starts a new timer if Sherlock's config contains a timer launcher.",
        parse: |_args, iter, _flags, startup| {
            if startup.is_none() {
                let duration: String = iter
                    .next()
                    .map(|s| s.to_owned())
                    .ok_or(ParseError::MissingValue("duration"))?;

                let command: Option<SharedString> = iter.next().map(Into::into);

                *startup = Some(ClientMessage::Timer { duration, command }.into())
            }

            Ok(())
        },
    },
    // Pipe Mode
    FlagSpec {
        long: "--method",
        short: None,
        section: FlagSection::Pipe,
        help: "What to do with the selected data row.",
        parse: |_args, iter, flags, _startup| {
            flags.method = iter.next().map(Into::into);
            Ok(())
        },
    },
    FlagSpec {
        long: "--field",
        short: None,
        section: FlagSection::Pipe,
        help: "Which field to print on return press.",
        parse: |_args, iter, flags, _startup| {
            flags.field = iter.next().map(Into::into);
            Ok(())
        },
    },
    FlagSpec {
        long: "--input",
        short: None,
        section: FlagSection::Pipe,
        help: "Input mode.",
        parse: |_args, iter, flags, _startup| {
            flags.input = iter.next().and_then(|v| v.parse().ok());
            Ok(())
        },
    },
];
