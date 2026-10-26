use std::sync::Arc;

use tokio::sync::mpsc::Receiver;

use crate::ui::views::process::{backends::internal::InternalBackend, utils::ResultHeap};

pub mod internal;

macro_rules! define_backend {
    ( enum $name:ident { $( $variant:ident( $inner:ty ) ),* $(,)? }) => {
        #[derive(Clone, Debug)]
        pub enum $name {
            $($variant($inner),)*
        }

        impl ProcessBackend {
            pub fn search(
                &self,
                query: Arc<str>,
                heap: &mut ResultHeap,
                cancel_rx: Receiver<()>,
            ) -> bool {
                match self {
                    $(
                        Self::$variant(inner) => <$inner as ProcessSearchProvider>::search(inner, query, heap, cancel_rx),
                    )*
                }
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                match self {
                    $(
                        Self::$variant(_) => serializer.serialize_str(&stringify!($variant).to_lowercase()),
                    )*
                }
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let s = String::deserialize(deserializer)?.to_lowercase();
                match s.as_str() {
                    $(
                        s if s == stringify!($variant).to_lowercase() => {
                            Ok(Self::$variant(<$inner>::default()))
                        }
                    )*
                    _ => Err(serde::de::Error::unknown_variant(&s, &[ $( stringify!($variant) ),* ])),
                }
            }
        }
    }
}

define_backend! {
    enum ProcessBackend {
        Internal(InternalBackend)
    }
}

impl Default for ProcessBackend {
    fn default() -> Self {
        Self::Internal(Default::default())
    }
}

#[allow(dead_code)]
pub trait ProcessSearchProvider {
    fn name(&self) -> &'static str;
    fn search(&self, query: Arc<str>, heap: &mut ResultHeap, cancel_rx: Receiver<()>) -> bool;
}
