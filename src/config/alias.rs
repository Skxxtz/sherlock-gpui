use std::{collections::HashMap, fs::File, io::ErrorKind};

use serde::Deserialize;

use crate::{
    loader::utils::ApplicationAction,
    sherlock_msg,
    ui::runtime_arguments::RuntimeArgument,
    utils::errors::{
        SherlockMessage,
        types::{FileAction, SherlockErrorType},
    },
};

#[derive(Deserialize, Clone, Debug)]
pub struct SherlockAlias {
    pub name: Option<String>,
    pub icon: Option<String>,
    pub exec: Option<String>,
    pub keywords: Option<String>,
    pub actions: Option<Vec<ApplicationAction>>,
    pub add_actions: Option<Vec<ApplicationAction>>,
    pub variables: Option<Vec<RuntimeArgument>>,
}

#[derive(Clone, Copy, Debug)]
pub enum AliasField {
    Name,
    Icon,
    Exec,
    Keywords,
}

impl SherlockAlias {
    #[inline(always)]
    pub fn load(path: &std::path::Path) -> Result<HashMap<String, SherlockAlias>, SherlockMessage> {
        match File::open(path) {
            Ok(f) => simd_json::from_reader(f).map_err(|e| {
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::DeserializationError(path.to_string_lossy().into()),
                    e
                )
            }),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(HashMap::new()),
            Err(e) => Err(sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Read, path.to_path_buf()),
                e
            )),
        }
    }

    fn field(&self, field: AliasField) -> Option<&str> {
        match field {
            AliasField::Name => self.name.as_deref(),
            AliasField::Icon => self.icon.as_deref(),
            AliasField::Exec => self.exec.as_deref(),
            AliasField::Keywords => self.keywords.as_deref(),
        }
    }

    /// First alias whose `field` equals `value`. Returns its key and data.
    pub fn search<'a>(
        aliases: &'a HashMap<String, SherlockAlias>,
        field: AliasField,
        value: &str,
    ) -> Option<(&'a str, &'a SherlockAlias)> {
        aliases
            .iter()
            .find(|(_, a)| a.field(field) == Some(value))
            .map(|(k, a)| (k.as_str(), a))
    }

    pub fn find_app_by_name(
        path: &std::path::Path,
        name: String,
    ) -> Result<String, SherlockMessage> {
        let aliases = Self::load(path)?;
        Ok(Self::search(&aliases, AliasField::Name, &name)
            .and_then(|(_, data)| data.name.clone())
            .unwrap_or(name))
    }
}
