use std::mem::discriminant;

use gpui::SharedString;
use serde::{Deserialize, Serialize};

use crate::utils::config::SherlockFlags;

#[derive(Deserialize, Serialize, Debug)]
pub enum ClientMessage {
    ConfigUpdate(Box<SherlockFlags>),
    Dmenu(Vec<SharedString>),
    Timer {
        duration: String,
        command: Option<SharedString>,
    },
    Open,
}

impl PartialEq for ClientMessage {
    fn eq(&self, other: &Self) -> bool {
        discriminant(self) == discriminant(other)
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ServerResponse {
    Print(String),
}

impl PartialEq for ServerResponse {
    fn eq(&self, other: &Self) -> bool {
        discriminant(self) == discriminant(other)
    }
}
