use std::sync::Arc;

use gpui::SharedString;
use serde::{Deserialize, Serialize};

use crate::ui::{choice::ChoiceOption, utils::input_rule::CharSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RuntimeArgument {
    String(StringVar),
    Password(PasswordVar),
    Number(NumberVar),
    Path(PathVar),
    Command(CommandVar),
    Choice(ChoiceVar),
}

impl RuntimeArgument {
    pub fn placeholder(&self) -> SharedString {
        match self {
            Self::String(v) => v.string_input.clone(),
            Self::Password(v) => v.password_input.clone(),
            Self::Number(v) => v.number_input.clone(),
            Self::Path(v) => v.path_input.clone(),
            Self::Command(v) => v.command_input.clone(),
            Self::Choice(v) => v.choice.name.clone(),
        }
    }
    pub fn default_value(&self) -> Option<SharedString> {
        match self {
            Self::String(v) => v.default.clone(),
            Self::Password(v) => v.default.clone(),
            Self::Number(v) => v.default.map(|n| n.to_string().into()),
            Self::Path(v) => v.default.clone(),
            Self::Command(v) => v.default.clone(),
            Self::Choice(v) => v.choice.default.clone(),
        }
    }
    pub fn validation(&self) -> Option<&CharSet> {
        if let Self::String(v) = self {
            v.validation.as_ref()
        } else {
            None
        }
    }
}

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StringVar {
    pub string_input: SharedString,
    pub default: Option<SharedString>,
    pub validation: Option<CharSet>,
}

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PasswordVar {
    pub password_input: SharedString,
    pub default: Option<SharedString>,
}

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NumberVar {
    pub number_input: SharedString,
    pub default: Option<f64>,
    pub validation: Option<CharSet>,
}

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PathVar {
    pub path_input: SharedString,
    pub default: Option<SharedString>,
    #[serde(skip)]
    pub index: usize,
}

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandVar {
    pub command_input: SharedString,
    pub default: Option<SharedString>,
    #[serde(skip)]
    pub index: usize,
    #[serde(skip)]
    pub is_scoped: bool,
}

impl From<SharedString> for CommandVar {
    fn from(value: SharedString) -> Self {
        Self {
            command_input: value,
            ..Default::default()
        }
    }
}

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChoiceVar {
    pub choice: ChoiceVarBody,
}

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChoiceVarBody {
    pub name: SharedString,
    pub default: Option<SharedString>,
    pub choices: Arc<[ChoiceOption]>,
}
