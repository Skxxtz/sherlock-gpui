use std::{rc::Rc, sync::Arc};

use gpui::{
    AnyElement, App, AppContext, Image, ImageSource, IntoElement, ParentElement, Styled, div, img,
    prelude::FluentBuilder, px,
};

use crate::{
    app::theme::ThemeData,
    launcher::{
        LauncherConfig,
        audio_launcher::{AudioLauncherFunctions, MusicPlayerFunctions, utils::MprisState},
        utils::{binds::Bind, exec_mode::ExecMode},
        variant_type::InnerFunction,
    },
    loader::utils::Priority,
    ui::{
        utils::{
            async_update::{AsyncUpdate, AsyncUpdateEntity, Fetchable},
            selection::Selection,
        },
        widgets::RenderableChildImpl,
    },
    utils::errors::SherlockMessage,
};

#[derive(Clone)]
pub struct MusicPlayerWidget {
    pub entity: AsyncUpdateEntity<MprisState>,
}
impl MusicPlayerWidget {
    pub fn new(cx: &mut impl AppContext) -> Self {
        Self {
            entity: AsyncUpdateEntity::<MprisState>::new(cx),
        }
    }
}

impl Fetchable for MprisState {
    type Error = SherlockMessage;
    async fn fetch(
        _launcher: &Arc<LauncherConfig>,
        old: Option<Rc<Self>>,
    ) -> Result<Option<Rc<Self>>, Self::Error> {
        let launcher = AudioLauncherFunctions::new()?;
        let player = match launcher.get_current_player() {
            Some(p) => p,
            None => return Ok(None),
        };

        let raw = launcher.get_metadata(&player);

        if let Some(old_ref) = old.as_ref()
            && old_ref.raw.as_ref() == raw.as_ref()
        {
            return Ok(old);
        }

        let mut image = None;
        if let Some(metadata) = raw.as_ref()
            && let Some((img_data, _)) = metadata.get_image().await
        {
            image = Some(img_data);
        }

        Ok(Some(Rc::new(MprisState { raw, image, player })))
    }
}

impl<'a> RenderableChildImpl<'a> for MusicPlayerWidget {
    fn render(
        &self,
        _launcher: &Arc<LauncherConfig>,
        selection: Selection,
        _query: &str,
        theme: Arc<ThemeData>,
        cx: &mut App,
    ) -> AnyElement {
        let Ok(Some(state)) = self.entity.read(cx).as_ref() else {
            return div().into_any_element();
        };

        div()
            .px_4()
            .py_2()
            .w_full()
            .flex()
            .gap_5()
            .items_center()
            .border_1()
            .rounded_md()
            .when(!selection.is_selected, |this| {
                this.border_color(theme.border_idle)
            })
            .child(if let Some(icon) = &state.image {
                img(ImageSource::Image(Arc::clone(icon)))
                    .size(px(64.))
                    .rounded_md()
            } else {
                img(ImageSource::Image(Arc::new(Image::empty()))).size(px(24.))
            })
            .child(
                div()
                    .text_color(theme.secondary_text)
                    .when(selection.is_selected, |this| {
                        this.text_color(theme.primary_text)
                    })
                    .flex_col()
                    .justify_between()
                    .items_center()
                    .when_some(
                        state.raw.as_ref().and_then(|s| s.metadata.title.as_ref()),
                        |this, name| {
                            this.child(
                                div()
                                    .text_sm()
                                    .font_family(theme.font_family.clone())
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .child(div().child(name.to_string())),
                            )
                        },
                    )
                    .when_some(
                        state.raw.as_ref().and_then(|s| {
                            s.metadata
                                .artists
                                .as_ref()
                                .filter(|artists| artists.iter().any(|a| !a.is_empty()))
                        }),
                        |this, st| this.text_xs().child(st.join(", ").to_string()),
                    ),
            )
            .into_any_element()
    }
    #[inline(always)]
    fn build_exec(&self, launcher: &Arc<LauncherConfig>, _cx: &mut App) -> Option<ExecMode> {
        Some(ExecMode::Inner {
            func: InnerFunction::MusicPlayer(MusicPlayerFunctions::TogglePlayback),
            exit: launcher.exit,
        })
    }
    #[inline(always)]
    fn get_content(&self, _launcher: &Arc<LauncherConfig>, cx: &mut App) -> Option<String> {
        let Ok(Some(inner)) = self.entity.read(cx) else {
            return None;
        };
        inner.raw.as_ref().and_then(|m| m.metadata.title.clone())
    }
    #[inline(always)]
    fn priority(&self, launcher: &Arc<LauncherConfig>) -> Priority {
        Priority::new_with_launcher(launcher, 0)
    }
    #[inline(always)]
    fn search(&'a self, _launcher: &Arc<LauncherConfig>) -> &'a str {
        ""
    }
    #[inline(always)]
    fn based_show<C: AppContext>(&self, _keyword: &str, cx: &mut C) -> Option<bool> {
        if self.entity.is_valid(cx) {
            None
        } else {
            Some(false)
        }
    }
    #[inline(always)]
    fn binds(&self, launcher: &Arc<LauncherConfig>, _cx: &mut App) -> Option<Arc<Vec<Bind>>> {
        launcher.launcher_type.binds()
    }
    #[inline(always)]
    fn update_async<C: AppContext>(&self, launcher: Arc<LauncherConfig>, cx: &mut C) {
        self.entity.update_async(launcher, cx);
    }
}
