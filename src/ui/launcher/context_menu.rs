use std::{borrow::Cow, fmt::Debug, sync::Arc};

use gpui::{
    Context, ImageSource, InteractiveElement, IntoElement, ParentElement, SharedString, Styled,
    div, img, prelude::FluentBuilder, px, relative,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    app::theme::ThemeData,
    features::emoji::{ALL_SKIN_TONES, SkinTone},
    loader::{
        IconType, resolve_icon_path,
        utils::{ApplicationAction, ApplicationActionSerde},
    },
    ui::{
        launcher::{
            LauncherView,
            context_menu::{action_keys::ContextActionBinds, quick_actions::QuickAction},
        },
        widgets::emoji::{EmojiAction, get_emoji, get_selected_skin_tones},
    },
};

pub mod action_keys;
pub mod quick_actions;
pub mod wrapper;

#[derive(Debug, PartialEq)]
pub enum ContextMenuAction {
    App(ApplicationAction),
    QuickAction(QuickAction),
    Fn(DynamicFunctionAction),
    Emoji(EmojiAction),
}
impl From<ApplicationAction> for Arc<ContextMenuAction> {
    fn from(value: ApplicationAction) -> Self {
        Arc::new(ContextMenuAction::App(value))
    }
}
impl From<ApplicationActionSerde> for Arc<ContextMenuAction> {
    fn from(value: ApplicationActionSerde) -> Self {
        Arc::new(ContextMenuAction::App(value.into()))
    }
}
impl Serialize for ContextMenuAction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ContextMenuAction::App(app) => serializer.serialize_some(app),

            _ => serializer.serialize_none(),
        }
    }
}
impl<'de> Deserialize<'de> for ContextMenuAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<ApplicationAction>::deserialize(deserializer)?;

        match opt {
            Some(app_action) => Ok(ContextMenuAction::App(app_action)),
            None => Err(serde::de::Error::custom(
                "Found None where App was expected",
            )),
        }
    }
}

impl ContextMenuAction {
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::App(act) => Some(&act.name),
            Self::Fn(act) => Some(&act.name),
            Self::QuickAction(q) => Some(q.name()),
            Self::Emoji(_) => None,
        }
    }
    pub fn render_row(
        &self,
        is_flashing: bool,
        is_selected: bool,
        theme: Arc<ThemeData>,
        key: Option<ContextActionBinds>,
    ) -> impl IntoElement {
        let (name, icon): (SharedString, Option<Cow<IconType>>) = match self {
            Self::Fn(this) => (this.name.clone(), this.icon.as_ref().map(Cow::Borrowed)),
            Self::App(this) => (this.name.clone(), this.icon.as_ref().map(Cow::Borrowed)),
            Self::QuickAction(this) => (
                this.to_string().into(),
                match this {
                    QuickAction::Hide => resolve_icon_path("sherlock-hide").map(Cow::Owned),
                    QuickAction::Info => resolve_icon_path("sherlock-info").map(Cow::Owned),
                },
            ),
            _ => return div(),
        };

        div()
            .group("")
            .rounded_sm()
            .relative()
            .flex_1()
            .flex()
            .gap(px(10.))
            .py(px(8.))
            .px(px(10.))
            .cursor_pointer()
            .text_color(if is_selected {
                theme.primary_text
            } else {
                theme.secondary_text
            })
            .text_size(px(13.))
            .font_family(theme.font_family.clone())
            .line_height(relative(1.0))
            .items_center()
            .bg(if is_selected && is_flashing {
                theme.bg_idle
            } else if is_flashing || is_selected {
                theme.bg_selected
            } else {
                theme.bg_idle
            })
            .hover(|s| {
                if is_selected {
                    s
                } else {
                    s.bg(theme.bg_selected)
                }
            })
            .child(if let Some(icon) = icon {
                if let Some(svg) = icon.svg() {
                    svg.size(px(16.))
                        .text_color(theme.secondary_text)
                        .into_any_element()
                } else {
                    img(icon.into_owned()).size(px(16.)).into_any_element()
                }
            } else {
                img(ImageSource::Image(Arc::new(gpui::Image::empty())))
                    .size(px(16.))
                    .into_any_element()
            })
            .child(
                div()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(name),
            )
            .when_some(key, |row, key| row.child(key.hint(&theme)))
    }
    pub fn render_emoji_col(
        &self,
        row_is_selected: bool,
        theme: Arc<ThemeData>,
    ) -> impl IntoElement {
        let Self::Emoji(this) = self else {
            return div();
        };

        let Some(emoji_entry) = this.entry() else {
            return div();
        };
        let mut tones = get_selected_skin_tones();
        let col_idx = this.get_index() as usize;

        let outer = div()
            .group("skin-tone-container")
            .flex()
            .flex_row()
            .w_full()
            .justify_between()
            .items_center()
            .rounded_sm()
            .relative()
            .gap(px(10.))
            .p(px(4.))
            .cursor_pointer()
            .text_color(if row_is_selected {
                theme.primary_text
            } else {
                theme.secondary_text
            })
            .text_size(px(13.))
            .line_height(relative(1.0))
            .items_center()
            .bg(if row_is_selected {
                theme.bg_selected
            } else {
                theme.bg_idle
            })
            .hover(|s| {
                if row_is_selected {
                    s
                } else {
                    if row_is_selected {
                        s
                    } else {
                        s.bg(theme.bg_selected)
                    }
                }
            });

        // when one part is simpsons, we need to force the other one to be simpsons too. when we
        // leave simpsons, we have to force the other one to be non-simpsons too.
        if tones.iter().any(|t| matches!(t, SkinTone::Simpsons)) {
            return outer.children(ALL_SKIN_TONES.iter().enumerate().map(|(i, tone)| {
                div()
                    .flex_1()
                    .flex()
                    .justify_center()
                    .items_center()
                    .rounded_sm()
                    .p(px(8.))
                    .when(col_idx == i, |this| this.bg(theme.border_selected))
                    .child(
                        div()
                            .flex()
                            .justify_center()
                            .items_center()
                            .w(px(24.))
                            .child(get_emoji(emoji_entry, &[*tone, *tone]).as_str().to_string()),
                    )
            }));
        }

        outer.children(ALL_SKIN_TONES.iter().enumerate().map(|(i, tone)| {
            tones[this.for_tone as usize] = *tone;
            div()
                .flex_1()
                .flex()
                .justify_center()
                .items_center()
                .rounded_sm()
                .p(px(8.))
                .when(col_idx == i, |this| this.bg(theme.border_selected))
                .child(
                    div()
                        .flex()
                        .justify_center()
                        .items_center()
                        .w(px(24.))
                        .child({
                            if i == 0 {
                                get_emoji(emoji_entry, &[SkinTone::Simpsons, SkinTone::Simpsons])
                                    .as_str()
                                    .to_string()
                            } else {
                                get_emoji(emoji_entry, &tones).as_str().to_string()
                            }
                        }),
                )
        }))
    }
}

pub type ContextFunction = dyn Fn(&mut Context<LauncherView>) + Send + Sync + 'static;

pub struct DynamicFunctionAction {
    pub name: SharedString,
    pub icon: Option<IconType>,
    pub exit: bool,
    pub func: Option<Box<ContextFunction>>,
}
impl Debug for DynamicFunctionAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name.as_str())
    }
}
impl PartialEq for DynamicFunctionAction {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.icon == other.icon && self.exit == other.exit
    }
}

impl DynamicFunctionAction {
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            icon: None,
            exit: true,
            func: None,
        }
    }

    pub fn on_exec<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut Context<LauncherView>) + Send + Sync + 'static,
    {
        self.func = Some(Box::new(f));
        self
    }

    pub fn icon(mut self, icon: IconType) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn icon_name(mut self, icon_name: &str) -> Self {
        self.icon = resolve_icon_path(icon_name);
        self
    }

    pub fn exit(mut self, should_exit: bool) -> Self {
        self.exit = should_exit;
        self
    }
}

#[cfg(test)]
mod tests {
    use std::{path::Path, sync::Arc};

    use super::*;
    use crate::ui::widgets::list_item::{ItemMeta, ListItem, app::AppPayload};

    #[test]
    fn test_context_menu_action_round_trip() {
        let app_action = ContextMenuAction::App(ApplicationAction::new("test", "test"));

        let serialized = serde_json::to_string(&app_action).expect("Failed to serialize");
        let deserialized: ContextMenuAction =
            serde_json::from_str(&serialized).expect("Failed to deserialize");

        assert_eq!(app_action, deserialized);
    }

    #[test]
    fn test_app_item_round_trip() {
        let action = Arc::new(ContextMenuAction::App(ApplicationAction::new(
            "test", "test",
        )));
        let item = ListItem {
            meta: ItemMeta {
                name: "Firefox".into(),
                search: "firefox;browser".into(),
                icon: None,
                actions: Arc::new([action]),
                vars: Arc::new([]),
            },
            payload: AppPayload {
                exec: "firefox %u".into(),
                terminal: false,
                desktop_file: Arc::from(Path::new("/usr/share/applications/firefox.desktop")),
                original_name: "Firefox".into(),
            },
        };

        let serialized = serde_json::to_string(&item).expect("Failed to serialize");
        let back: ListItem<AppPayload> =
            serde_json::from_str(&serialized).expect("Failed to deserialize");

        assert_eq!(item.meta.name, back.meta.name);
        assert_eq!(item.meta.search, back.meta.search);
        assert_eq!(item.meta.actions, back.meta.actions);
        assert_eq!(item.payload.exec, back.payload.exec);
        assert_eq!(item.payload.terminal, back.payload.terminal);
        assert_eq!(item.payload.desktop_file, back.payload.desktop_file);
        assert_eq!(item.payload.original_name, back.payload.original_name);
    }
}
