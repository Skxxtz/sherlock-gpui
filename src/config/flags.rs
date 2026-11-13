use std::{fs::read_to_string, path::PathBuf};

use serde::{Deserialize, Serialize};

use super::{ConfigSourceFiles, SherlockConfig, types::take_warnings};
use crate::{
    sherlock_msg,
    utils::{
        errors::{
            SherlockMessage,
            types::{FileAction, SherlockErrorType},
        },
        files::{expand_path, home_dir},
        paths,
    },
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SherlockFlags {
    pub config_dir: Option<PathBuf>,
    pub config: Option<PathBuf>,
    pub fallback: Option<PathBuf>,
    pub style: Option<PathBuf>,
    pub ignore: Option<PathBuf>,
    pub alias: Option<PathBuf>,
    pub display_raw: bool,
    pub center_raw: bool,
    pub cache: Option<PathBuf>,
    pub method: Option<String>,
    pub field: Option<String>,
    pub sub_menu: Option<String>,
    pub multi: bool,
    pub photo_mode: bool,
    pub input: Option<bool>,
    pub placeholder: Option<String>,
    pub wait: bool,
}

impl SherlockFlags {
    pub fn get_config(
        &mut self,
    ) -> Result<(SherlockConfig, Vec<SherlockMessage>), SherlockMessage> {
        // Get location of config file
        let config_dir = self.config_dir.take().unwrap_or(paths::get_config_dir()?);
        let home = home_dir()?;
        let mut path = match &self.config {
            Some(path) => expand_path(path, home),
            _ => config_dir.join("config.toml"),
        };

        // logic to either use json or toml
        let filetype = if let Some(ext) = path.extension() {
            let mut ext_str = ext.to_string_lossy().to_string();
            if !path.exists() {
                match ext_str.as_str() {
                    "json" => {
                        path.set_extension("toml");
                        ext_str = "toml".into();
                    }
                    "toml" => {
                        path.set_extension("json");
                        ext_str = "json".into();
                    }
                    _ => {
                        return Err(sherlock_msg!(
                            Warning,
                            SherlockErrorType::FileError(FileAction::Parse, path.clone()),
                            format!("unsupported format: '{}'", ext_str)
                        ));
                    }
                }
            }
            ext_str
        } else {
            return Err(sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Parse, path.clone()),
                "file has no extension and cannot be parsed"
            ));
        };

        match std::fs::read_to_string(&path) {
            Ok(mut config_str) => {
                let mut unknown: Vec<String> = Vec::new();
                take_warnings(); // stale ones from an earlier read
                let config_res: Result<SherlockConfig, SherlockMessage> = match filetype.as_str() {
                    "json" => {
                        let mut bytes = config_str.into_bytes();
                        let err = |e: &dyn std::fmt::Display| {
                            sherlock_msg!(
                                Warning,
                                SherlockErrorType::DeserializationError("Json Config".into()),
                                e.to_string()
                            )
                        };
                        simd_json::Deserializer::from_slice(&mut bytes)
                            .map_err(|e| err(&e))
                            .and_then(|mut de| {
                                serde_ignored::deserialize(&mut de, |p| unknown.push(p.to_string()))
                                    .map_err(|e| err(&e))
                            })
                    }
                    "toml" => {
                        // Setup to parse nested configs
                        if let Ok(sources) = toml::de::from_str::<ConfigSourceFiles>(&config_str)
                            && !sources.source.is_empty()
                        {
                            sources
                                .source
                                .into_iter()
                                .map(|s| {
                                    if s.file.starts_with("~/") {
                                        expand_path(s.file, home)
                                    } else {
                                        s.file
                                    }
                                })
                                .filter(|f| f.is_file())
                                .filter_map(|f| read_to_string(&f).ok())
                                .for_each(|content| {
                                    config_str.push('\n');
                                    config_str.push_str(&content);
                                });
                        }
                        let err = |e: toml::de::Error| {
                            sherlock_msg!(
                                Warning,
                                SherlockErrorType::DeserializationError("Toml Config".into()),
                                e
                            )
                        };
                        toml::Deserializer::parse(&config_str)
                            .map_err(err)
                            .and_then(|de| {
                                serde_ignored::deserialize(de, |p| unknown.push(p.to_string()))
                                    .map_err(err)
                            })
                    }
                    _ => {
                        let ext = path
                            .extension()
                            .map(|s| s.to_string_lossy())
                            .unwrap_or_else(|| "none".into());

                        return Err(sherlock_msg!(
                            Warning,
                            SherlockErrorType::FileError(FileAction::Parse, path.clone()),
                            format!("the file extension '{}' is not a supported format", ext)
                        ));
                    }
                };
                // problems that didn't stop the config from loading
                let mut warnings = take_warnings();
                unknown.retain(|key| key != "source" && !key.starts_with("source."));
                warnings.extend(
                    unknown
                        .into_iter()
                        .map(|key| format!("unknown key `{key}`, ignored")),
                );

                match config_res {
                    Ok(mut config) => {
                        config.apply_flags(self);
                        config.initialized = true;
                        warnings.extend(config.validate());
                        let messages = warnings
                            .into_iter()
                            .map(|w| {
                                sherlock_msg!(
                                    Warning,
                                    SherlockErrorType::ConfigError(path.display().to_string()),
                                    w
                                )
                            })
                            .collect();
                        Ok((config, messages))
                    }
                    Err(e) => {
                        let mut config = SherlockConfig::default();
                        config.apply_flags(self);
                        Ok((config, vec![e]))
                    }
                }
            }
            Err(e) => {
                let mut config = SherlockConfig::default();
                config.apply_flags(self);
                let e = sherlock_msg!(
                    Warning,
                    SherlockErrorType::FileError(FileAction::Read, path),
                    e
                );
                Ok((config, vec![e]))
            }
        }
    }
}
