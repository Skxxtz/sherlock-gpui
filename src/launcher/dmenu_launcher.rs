use std::sync::Arc;

use crate::{
    launcher::{LauncherConfig, LauncherProvider, LauncherType},
    loader::utils::RawLauncher,
    ui::widgets::RenderableChild,
    utils::errors::SherlockMessage,
};

/// No user-side arguments
#[derive(Clone, Debug, Default)]
pub struct DmenuLauncher {}

impl LauncherProvider for DmenuLauncher {
    fn try_parse(_raw: &RawLauncher) -> Result<LauncherType, SherlockMessage> {
        Ok(LauncherType::Dmenu(Self {}))
    }

    fn objects(
        &self,
        _launcher: Arc<LauncherConfig>,
        _ctx: &crate::loader::LoadContext,
        _opts: std::sync::Arc<serde_json::Value>,
        _messages: &mut Vec<SherlockMessage>,
        _cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, SherlockMessage> {
        // Should never be called! This is only from piped input.
        unimplemented!()
    }
}

// DOCS
#[cfg(feature = "docs")]
mod docs {
    use super::DmenuLauncher;
    use crate::{
        display_name,
        docs::launcher::{LauncherDoc, LauncherDocEntry},
        variant_name,
    };

    impl LauncherDoc for DmenuLauncher {
        fn doc() -> LauncherDocEntry {
            LauncherDocEntry::new_hidden(
                display_name!(DmenuLauncher),
                variant_name!(Dmenu),
                "The launcher to handle Dmenu-style piping",
            )
        }
    }
}
