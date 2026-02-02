use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

use gpui::{IntoElement, ParentElement, SharedString, Styled, div, px, rgb};

use crate::{launcher::children::RenderableChildImpl, utils::intent::Intent};

#[derive(Clone)]
pub struct CalcData {
    capabilities: HashSet<String>,
    result: Arc<RwLock<Option<(SharedString, SharedString)>>>,
}

impl CalcData {
    pub fn new(capabilities: HashSet<String>) -> Self {
        Self {
            capabilities,
            result: Arc::new(RwLock::new(None)),
        }
    }
    pub fn based_show(&self, keyword: &str) -> bool {
        if keyword.trim().is_empty() {
            return false;
        }

        let mut result = None;

        if self.capabilities.contains("calc.math") {
            let trimmed_keyword = keyword.trim();
            if let Ok(r) = meval::eval_str(trimmed_keyword) {
                let r = r.to_string();
                if &r != trimmed_keyword {
                    result = Some((r.clone(), format!("= {}", r)));
                }
            }
        }

        {
            let intent = Intent::parse(keyword);
            let r = match intent {
                Intent::ColorConvert { .. } => intent.execute(),
                Intent::Conversion { .. } => intent.execute(),
                _ => None,
            };

            if let Some(r) = r {
                result = Some((r.clone(), r));
            }
        }

        let show = result.is_some();
        if let Ok(mut writer) = self.result.write() {
            *writer = result.map(|(o, r)| (SharedString::from(o), SharedString::from(r)));
        }
        show
    }
}

impl<'a> RenderableChildImpl<'a> for CalcData {
    fn search(&'a self, _launcher: &std::sync::Arc<crate::launcher::Launcher>) -> &'a str {
        ""
    }
    fn execute(
        &self,
        _launcher: &std::sync::Arc<crate::launcher::Launcher>,
        _keyword: &str,
        _variables: &[(SharedString, SharedString)],
    ) -> Result<bool, crate::utils::errors::SherlockError> {
        Ok(false)
    }
    fn priority(&self, launcher: &std::sync::Arc<crate::launcher::Launcher>) -> f32 {
        launcher.priority as f32
    }
    fn render(
        &self,
        _launcher: &std::sync::Arc<crate::launcher::Launcher>,
        is_selected: bool,
    ) -> gpui::AnyElement {
        let result = {
            let guard = self.result.read().unwrap();
            let Some((_, res)) = guard.as_ref() else {
                return div().into_any_element();
            };
            res.clone()
        };

        div()
            .px_4()
            .py_7()
            .size_full()
            .flex()
            .gap_5()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_size(px(24.0))
                    .text_color(if is_selected {
                        rgb(0xDDD5D0)
                    } else {
                        rgb(0x6E6E6E)
                    })
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .child(result),
            )
            .into_any_element()
    }
}
