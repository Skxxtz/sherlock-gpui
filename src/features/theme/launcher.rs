use std::sync::Arc;

use gpui::App;

use crate::{
    app::theme::{ActiveTheme, ThemeData},
    define_inner_functions, ensure_func,
    launcher::{
        ExecEffect, LauncherConfig, LauncherProvider, LauncherType, variant_type::InnerFunction,
    },
    loader::utils::RawLauncher,
    sherlock_msg,
    ui::widgets::{RenderableChild, theme::ThemeWidget},
    utils::{
        errors::{
            SherlockMessage,
            types::{DirAction, FileAction, SherlockErrorType},
        },
        files::{expand_path, home_dir},
        format::make_title_case,
    },
};

define_inner_functions! {
    pub enum ThemePickerFunctions {
        Pick { theme: Arc<ThemeData> },
    }
}

/// The following arguments are available to users:
/// - `path`: The path to look for themes in
/// - TODO: `short_defaults`: Wheather default themes should be shown.
///
/// The following inner functions are available:
/// - `Pick`: Pick a theme, not user-facing yet
#[derive(Clone, Debug)]
pub struct ThemePicker {}

impl LauncherProvider for ThemePicker {
    fn try_parse(_raw: &RawLauncher) -> Result<LauncherType, SherlockMessage> {
        Ok(LauncherType::Theme(ThemePicker {}))
    }
    fn objects(
        &self,
        launcher: std::sync::Arc<LauncherConfig>,
        _ctx: &crate::loader::LoadContext,
        opts: std::sync::Arc<serde_json::Value>,
        messages: &mut Vec<SherlockMessage>,
        _cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, crate::utils::errors::SherlockMessage> {
        let path_str = opts
            .get("path")
            .and_then(|p| p.as_str())
            .unwrap_or("~/.config/sherlock/themes/");
        let home = home_dir()?;
        let path = expand_path(path_str, home);

        let builtin = [
            ("Default", ThemeData::dark()),
            ("Nord", ThemeData::nord()),
            ("Libre", ThemeData::libre()),
            ("Catppuccin Mocha", ThemeData::catppuccin_mocha()),
        ]
        .into_iter()
        .map(|(name, data)| {
            RenderableChild::new(
                launcher.clone(),
                ThemeWidget::new(name, Arc::new(data), true),
            )
        });

        let custom = if path.is_dir() {
            std::fs::read_dir(&path)
                .map_err(|e| {
                    sherlock_msg!(
                        Warning,
                        SherlockErrorType::DirError(DirAction::Read, path),
                        e
                    )
                })?
                .flatten()
                .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("toml"))
                .filter_map(|file| {
                    let content = std::fs::read_to_string(file.path())
                        .map_err(|e| {
                            messages.push(sherlock_msg!(
                                Warning,
                                SherlockErrorType::FileError(FileAction::Read, file.path()),
                                e
                            ));
                        })
                        .ok()?;
                    let data = toml::from_str::<ThemeData>(&content)
                        .map_err(|e| {
                            messages.push(sherlock_msg!(
                                Warning,
                                SherlockErrorType::DeserializationError(
                                    file.path().to_string_lossy().into()
                                ),
                                e
                            ))
                        })
                        .ok()?;

                    let mut name = file.path().file_stem()?.to_string_lossy().to_string();
                    make_title_case(&mut name);

                    Some(RenderableChild::new(
                        launcher.clone(),
                        ThemeWidget::new(name, Arc::new(data), false),
                    ))
                })
                .collect::<Vec<_>>()
        } else {
            vec![]
        };

        Ok(builtin.chain(custom).collect())
    }
    fn execute_function(
        &self,
        func: InnerFunction,
        _child: &RenderableChild,
        _variables: &[(gpui::SharedString, gpui::SharedString)],
        cx: &mut App,
    ) -> Result<ExecEffect, crate::utils::errors::SherlockMessage> {
        let func = ensure_func!(func, InnerFunction::Theme);

        match func {
            ThemePickerFunctions::Pick { theme } => {
                cx.set_global(ActiveTheme(theme));
            }
        }

        Ok(ExecEffect::None)
    }
}

// DOCS
#[cfg(feature = "docs")]
mod docs {
    use indoc::indoc;

    use super::ThemePicker;
    use crate::{
        display_name,
        docs::launcher::{Example, FieldDoc, InnerFunctionDoc, LauncherDoc, LauncherDocEntry},
        variant_name,
    };

    impl LauncherDoc for ThemePicker {
        fn doc() -> LauncherDocEntry {
            LauncherDocEntry {
                name: display_name!(ThemePicker),
                variant_name: variant_name!(Theme),
                description: "Preview and select Sherlock themes.",
                args: &[FieldDoc {
                    name: "path",
                    ty: "path",
                    required: false,
                    default: Some("~/.config/sherlock/themes/"),
                    description: "The path to the Sherlock themes directory.",
                }],
                inner_functions: &[InnerFunctionDoc {
                    name: "Pick",
                    identifier: "inner.pick",
                    description: "Apply a the selected theme as the active theme. (Not user-facing yet)",
                    user_facing: false, // TODO
                }],
                examples: &[Example {
                    description: "Basic process terminator",
                    json: indoc! {
                        r#"{
                        "name": "Theme Picker",
                        "type": "theme",
                        "alias": "themes",
                        "priority": 0,
                        "exit": false
                    }"#
                    },
                }],
                ..LauncherDocEntry::new()
            }
        }
    }
}
