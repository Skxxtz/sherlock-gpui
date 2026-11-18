use std::panic::Location;

use super::{ErrorKind, Format, SherlockMessage};

macro_rules! from_error {
    ($($ty:ty => $kind:expr),+ $(,)?) => {$(
        impl From<$ty> for SherlockMessage {
            #[track_caller]
            fn from(e: $ty) -> Self {
                let kind: ErrorKind = $kind;
                SherlockMessage::at(
                    kind.level(),
                    kind,
                    e.to_string().into(),
                    Location::caller(),
                )
            }
        }
    )+};
}

from_error! {
    std::io::Error => ErrorKind::Io,
    toml::de::Error => ErrorKind::parse(Format::Toml, "input"),
    toml::ser::Error => ErrorKind::Serialize(Format::Toml),
    serde_json::Error => ErrorKind::parse(Format::Json, "input"),
    simd_json::Error => ErrorKind::parse(Format::Json, "input"),
}
