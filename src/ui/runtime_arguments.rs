use gpui::SharedString;
use serde::{Deserialize, Serialize};

use crate::ui::{
    runtime_arguments::{
        choice::ChoiceVar, command::CommandVar, number::NumberVar, password::PasswordVar,
        path::PathVar, string::StringVar,
    },
    utils::input_rule::CharSet,
};

pub mod choice;
pub mod command;
pub mod number;
pub mod password;
pub mod path;
pub mod string;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
