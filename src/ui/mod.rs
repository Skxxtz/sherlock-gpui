use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Hash, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum UIFunction {
    Exit,

    ItemDown,
    ItemUp,
    ItemLeft,
    ItemRight,

    ArgNext,
    ArgPrev,

    Exec,
    ExecInplace,

    MultiSelect,

    ToggleContext,
    CloseContext,

    ClearBar,
    Backspace,

    ErrorPage,

    Shortcut,
}
