use std::sync::Arc;

use gpui::Image;
use zbus::zvariant::{DeserializeDict, Type};

#[derive(Clone, Default)]
pub struct MprisState {
    pub player: String,
    pub raw: Option<MprisData>,
    pub image: Option<Arc<Image>>,
}

#[derive(DeserializeDict, Type, Debug, Clone, Default, PartialEq)]
#[zvariant(signature = "a{sv}")]
#[allow(unused)]
pub struct MprisData {
    #[zvariant(rename = "PlaybackStatus")]
    pub playback_status: String,

    #[zvariant(rename = "Metadata")]
    pub metadata: MetaData,
}

#[derive(DeserializeDict, Type, Debug, Clone, Default, PartialEq)]
#[zvariant(signature = "a{sv}")]
#[allow(unused)]
pub struct MetaData {
    #[zvariant(rename = "xesam:title")]
    pub title: Option<String>,

    #[zvariant(rename = "xesam:album")]
    pub album: Option<String>,

    #[zvariant(rename = "xesam:artist")]
    pub artists: Option<Vec<String>>,

    #[zvariant(rename = "xesam:url")]
    pub url: Option<String>,

    #[zvariant(rename = "mpris:artUrl")]
    pub art: Option<String>,
}
