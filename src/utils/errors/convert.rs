use std::panic::Location;

use super::{ErrorKind, SherlockMessage, SherlockMessageLevel};

macro_rules! from_error {
    ($($ty:ty => $kind:expr),+ $(,)?) => {$(
        impl From<$ty> for SherlockMessage {
            #[track_caller]
            fn from(e: $ty) -> Self {
                SherlockMessage::at(
                    SherlockMessageLevel::Warning,
                    $kind,
                    e.to_string().into(),
                    Location::caller(),
                )
            }
        }
    )+};
}

from_error! {
    std::io::Error => ErrorKind::Io,
    toml::de::Error => ErrorKind::Parse("toml".into()),
    toml::ser::Error => ErrorKind::Serialize,
    serde_json::Error => ErrorKind::Parse("json".into()),
    simd_json::Error => ErrorKind::Parse("json".into()),
}
