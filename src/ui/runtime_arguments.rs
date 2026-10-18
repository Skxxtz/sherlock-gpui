use gpui::SharedString;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, DeserializeOwned},
};

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
#[serde(remote = "Self")]
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

impl Serialize for RuntimeArgument {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if !s.is_human_readable() {
            return Self::serialize(self, s);
        }
        match self {
            Self::String(v) => v.serialize(s),
            Self::Password(v) => v.serialize(s),
            Self::Number(v) => v.serialize(s),
            Self::Path(v) => v.serialize(s),
            Self::Command(v) => v.serialize(s),
            Self::Choice(v) => v.serialize(s),
        }
    }
}

impl<'de> Deserialize<'de> for RuntimeArgument {
    /// Binary cache: tagged layout. JSON config: variant chosen by its key.
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        if !d.is_human_readable() {
            return Self::deserialize(d);
        }

        let value = serde_json::Value::deserialize(d)?;
        let has = |k: &str| value.get(k).is_some();
        let key = [
            "string_input",
            "password_input",
            "number_input",
            "path_input",
            "command_input",
            "choice",
        ]
        .into_iter()
        .find(|k| has(k));

        Ok(match key {
            Some("string_input") => Self::String(parse(value)?),
            Some("password_input") => Self::Password(parse(value)?),
            Some("number_input") => Self::Number(parse(value)?),
            Some("path_input") => Self::Path(parse(value)?),
            Some("command_input") => Self::Command(parse(value)?),
            Some("choice") => Self::Choice(parse(value)?),
            _ => {
                return Err(de::Error::custom(
                    "variable needs one of: string_input, password_input, number_input, \
                     path_input, command_input, choice",
                ));
            }
        })
    }
}

fn parse<T: DeserializeOwned, E: de::Error>(v: serde_json::Value) -> Result<T, E> {
    serde_json::from_value(v).map_err(E::custom)
}
