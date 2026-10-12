use crate::{
    app::theme::{ActiveTheme, ThemeData},
    launcher::{LauncherConfig, utils::exec_mode::ExecMode},
    loader::{IconType, resolve_icon_path, utils::Priority},
    ui::{
        launcher::views::NavigationViewType, traits::RenderableChildImpl,
        utils::selection::Selection,
    },
    utils::{
        filekinds::{FileCategory, FileKind},
        preview_lru::PreviewCache,
    },
};
use gpui::{
    AnyElement, App, Image, ImageSource, IntoElement, ParentElement, RenderImage, RenderOnce,
    SharedString, Styled, StyledImage, div, img, prelude::FluentBuilder, px,
};
use sherlock_derive::Hideable;
use std::{sync::Arc, time::UNIX_EPOCH};

enum Preview {
    Image(Arc<RenderImage>),
    Text(SharedString),
}

#[derive(Clone, Default, Debug, Hideable)]
pub struct FileData {
    loc: SharedString,
    name: SharedString,
    icon: Option<IconType>,
}

impl FileData {
    pub fn new(loc: Arc<str>) -> Self {
        let name: Arc<str> = loc
            .trim_end_matches('/')
            .rsplit_once('/')
            .map(|(_, name)| name)
            .unwrap_or(&loc)
            .into();
        Self {
            loc: loc.clone().into(),
            name: name.into(),
            icon: None,
        }
    }

    pub fn with_icon_name(mut self, icon_name: &str) -> Self {
        self.icon = resolve_icon_path(icon_name);
        self
    }

    fn fetch_meta(&self) -> Option<FileMeta> {
        let path = std::path::Path::new(self.loc.as_ref());
        let meta = std::fs::symlink_metadata(path).ok()?;
        let is_dir = meta.is_dir();
        let is_symlink = meta.file_type().is_symlink();

        let symlink_target = if is_symlink {
            std::fs::read_link(path)
                .ok()
                .map(|t| t.to_string_lossy().into_owned())
        } else {
            None
        };

        let extension = if is_symlink || is_dir {
            None
        } else {
            path.extension()
                .map(|e| e.to_string_lossy().to_uppercase())
                .map(|e| e.to_owned())
        };

        let kind = FileKind::from_extension(extension.as_deref(), is_dir, is_symlink);
        let size = if is_dir {
            std::fs::read_dir(path)
                .map(|e| format!("{} items", e.filter_map(|e| e.ok()).count()))
                .unwrap_or_else(|_| "—".into())
        } else {
            format_size(meta.len())
        };

        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| format_timestamp(d.as_secs()))
            .unwrap_or_else(|| "—".into());

        let created = meta
            .created()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| format_timestamp(d.as_secs()))
            .unwrap_or_else(|| "—".into());

        let (permissions, executable) = {
            use std::os::unix::fs::PermissionsExt;
            let mode = meta.permissions().mode();
            (format_permissions(mode), !is_dir && (mode & 0o111 != 0))
        };

        Some(FileMeta {
            kind,
            size: size.into(),
            modified,
            created,
            permissions,
            executable,
            symlink_target,
        })
    }
}

struct FileMeta {
    kind: FileKind,
    size: SharedString,
    modified: String,
    created: String,
    permissions: String,
    executable: bool,
    symlink_target: Option<String>,
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{bytes} B")
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}

fn format_timestamp(secs: u64) -> String {
    // Simple formatting without chrono — compute y/m/d from epoch seconds
    let days_since_epoch = secs / 86400;
    let seconds_in_day = secs % 86400;
    let hours = seconds_in_day / 3600;
    let minutes = (seconds_in_day % 3600) / 60;

    // Rata Die algorithm for Gregorian calendar
    let z = days_since_epoch as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{:02}.{:02}.{:04} {:02}:{:02}", d, m, y, hours, minutes)
}

fn format_permissions(mode: u32) -> String {
    let chars = [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ];
    let s: String = chars
        .iter()
        .map(|&(bit, ch)| if mode & bit != 0 { ch } else { '-' })
        .collect();
    format!("{s} ({:04o})", mode & 0o777)
}

impl<'a> RenderableChildImpl<'a> for FileData {
    fn render(
        &self,
        _launcher: &Arc<LauncherConfig>,
        selection: Selection,
        _query: &str,
        theme: Arc<ThemeData>,
        _cx: &mut App,
    ) -> AnyElement {
        div()
            .px_4()
            .py_2()
            .w_full()
            .flex()
            .gap_5()
            .items_center()
            .child(if let Some(icon) = self.icon.as_ref() {
                if let Some(svg) = icon.svg() {
                    svg.size(px(24.))
                        .text_color(theme.primary_text)
                        .into_any_element()
                } else {
                    img(icon.clone())
                        .size(px(24.))
                        .flex_shrink_0()
                        .into_any_element()
                }
            } else {
                img(ImageSource::Image(Arc::new(Image::empty())))
                    .size(px(24.))
                    .flex_shrink_0()
                    .into_any_element()
            })
            .child(
                div()
                    .flex_col()
                    .justify_between()
                    .items_center()
                    .min_w_0()
                    .w_full()
                    .child(
                        div()
                            .font_family(theme.font_family.clone())
                            .text_sm()
                            .w_full()
                            .text_color(theme.secondary_text)
                            .when(selection.is_selected, |this| {
                                this.text_color(theme.primary_text)
                            })
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .child(self.name.clone()),
                    )
                    .child(
                        div()
                            .font_family(theme.font_family.clone())
                            .text_xs()
                            .w_full()
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .text_color(theme.secondary_text)
                            .child(self.loc.clone()),
                    ),
            )
            .into_any_element()
    }

    #[inline(always)]
    fn sidebar(&self, _launcher: &Arc<LauncherConfig>, _cx: &mut App) -> Option<AnyElement> {
        let meta = self.fetch_meta()?;
        Some(
            FileSidebar {
                meta,
                icon: self.icon.clone(),
                name: self.name.clone(),
                loc: self.loc.clone(),
            }
            .into_any_element(),
        )
    }

    #[inline(always)]
    fn has_sidebar(&self) -> bool {
        true
    }

    #[inline(always)]
    fn build_exec(&self, launcher: &Arc<LauncherConfig>, _cx: &mut App) -> Option<ExecMode> {
        if self.loc.ends_with('/') {
            return Some(ExecMode::CreateView {
                mode: NavigationViewType::Files {
                    dir: Some(self.loc.clone()),
                },
                launcher: Arc::clone(launcher),
            });
        }
        None
    }

    #[inline(always)]
    fn get_content(&self, _launcher: &Arc<LauncherConfig>, _cx: &mut App) -> Option<String> {
        None
    }

    #[inline(always)]
    fn priority(&self, launcher: &Arc<LauncherConfig>) -> Priority {
        Priority::new_with_launcher(launcher, 0)
    }

    #[inline(always)]
    fn search(&'a self, _launcher: &Arc<LauncherConfig>) -> &'a str {
        &self.loc
    }
}

#[derive(IntoElement)]
struct FileSidebar {
    meta: FileMeta,
    loc: SharedString,
    name: SharedString,
    icon: Option<IconType>,
}
impl RenderOnce for FileSidebar {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let theme = cx.global::<ActiveTheme>().0.clone();

        let kind = self.meta.kind;
        let preview = if kind.has_image_preview() {
            crate::utils::preview::request_preview(self.loc.clone(), cx).map(Preview::Image)
        } else if kind.is_text() || kind.category() == FileCategory::Other {
            read_text_preview(self.loc.as_str(), 4096, 20)
                .map(SharedString::from)
                .map(Preview::Text)
        } else {
            None
        };
        cx.default_global::<PreviewCache>().flush(window);

        // Compact label/value row
        let row = |label: &'static str, value: SharedString| {
            div()
                .flex()
                .items_start()
                .justify_between()
                .gap_4()
                .py(px(4.))
                .child(
                    div()
                        .text_xs()
                        .font_family(theme.font_family.clone())
                        .text_color(theme.secondary_text)
                        .flex_shrink_0()
                        .child(label),
                )
                .child(
                    div()
                        .text_xs()
                        .font_family(theme.font_family.clone())
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.primary_text)
                        .text_right()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(value),
                )
        };

        let section_label = |text: &'static str| {
            div()
                .text_xs()
                .font_family(theme.font_family.clone())
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.secondary_text)
                .pt_3()
                .pb_1()
                .child(SharedString::from(text))
        };

        let separator = || div().h(px(1.)).w_full().bg(theme.border_selected).my_1();

        div()
            .min_h_full()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .pb(px(12.))
                    .child(
                        // Icon box
                        div()
                            .size(px(48.))
                            .rounded_lg()
                            .bg(theme.bg_muted)
                            .flex()
                            .items_center()
                            .justify_center()
                            .flex_shrink_0()
                            .child(if let Some(icon) = &self.icon {
                                if let Some(svg) = icon.svg() {
                                    svg.size(px(32.))
                                        .text_color(theme.tertiary_text)
                                        .into_any_element()
                                } else {
                                    img(icon.clone()).size(px(32.)).into_any_element()
                                }
                            } else {
                                div().into_any_element()
                            }),
                    )
                    .child(
                        div()
                            .flex_col()
                            .gap_1()
                            .min_w_0()
                            .child(
                                div()
                                    .text_sm()
                                    .font_family(theme.font_family.clone())
                                    .px(px(3.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.primary_text)
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .child(self.name.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .px(px(6.))
                                            .py(px(1.))
                                            .rounded(px(4.))
                                            .bg(theme.bg_muted)
                                            .text_xs()
                                            .font_family(theme.font_family.clone())
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(theme.secondary_text)
                                            .child(self.meta.kind.as_str()),
                                    )
                                    .when(self.meta.executable, |this| {
                                        this.child(
                                            div()
                                                .px(px(6.))
                                                .py(px(1.))
                                                .rounded(px(4.))
                                                .bg(theme.bg_muted)
                                                .text_xs()
                                                .font_family(theme.font_family.clone())
                                                .font_weight(gpui::FontWeight::MEDIUM)
                                                .text_color(theme.secondary_text)
                                                .child(SharedString::from("Executable")),
                                        )
                                    }),
                            ),
                    ),
            )
            .child(
                div()
                    .mt_auto()
                    .pt(px(5.))
                    .text_xs()
                    .text_color(theme.secondary_text)
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .child(self.loc.clone()),
            )
            .child(separator())
            .child(
                div()
                    .flex_col()
                    .child(section_label("Info"))
                    .child(row("Size", self.meta.size.clone()))
                    .child(row("Kind", self.meta.kind.as_str().into()))
                    .when_some(self.meta.symlink_target, |this, target| {
                        this.child(row("Points to", target.into()))
                    }),
            )
            .child(separator())
            .child(
                div()
                    .flex_col()
                    .child(section_label("Dates"))
                    .child(row("Modified", self.meta.modified.into()))
                    .child(row("Created", self.meta.created.into())),
            )
            .child(separator())
            .child(
                div()
                    .flex_col()
                    .child(section_label("Permissions"))
                    .child(row("Mode", self.meta.permissions.into())),
            )
            .when_some(preview, |this, val| {
                this.child(separator())
                    .child(section_label("Preview"))
                    .child(match val {
                        Preview::Image(image) => div()
                            .w_full()
                            .mt(px(3.))
                            .mb(px(12.))
                            .rounded_md()
                            .overflow_hidden()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                img(image)
                                    .w_full()
                                    .h(px(200.))
                                    .object_fit(gpui::ObjectFit::Contain),
                            ),
                        Preview::Text(text) => div()
                            .w_full()
                            .mt(px(3.))
                            .mb(px(12.))
                            .p_2()
                            .max_h(px(200.))
                            .overflow_hidden()
                            .rounded_md()
                            .bg(theme.bg_muted)
                            .font_family("monospace")
                            .text_size(px(11.))
                            .text_color(theme.secondary_text)
                            .children(text.lines().map(|line| {
                                div()
                                    .whitespace_nowrap()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .child(
                                        SharedString::from(if line.is_empty() {
                                            " "
                                        } else {
                                            line
                                        })
                                        .to_string(),
                                    )
                            })),
                    })
            })
    }
}

/// First lines of a text file, or None if it looks binary.
fn read_text_preview<P: AsRef<std::path::Path>>(
    path: P,
    max_bytes: usize,
    max_lines: usize,
) -> Option<String> {
    use std::io::Read;

    let path = path.as_ref();

    let mut buf = Vec::with_capacity(max_bytes);
    std::fs::File::open(path)
        .ok()?
        .take(max_bytes as u64)
        .read_to_end(&mut buf)
        .ok()?;

    if buf.contains(&0) {
        return None; // NUL byte → binary
    }
    // Cut off a UTF-8 character that may have been split at the 4 KB boundary.
    let text = match std::str::from_utf8(&buf) {
        Ok(s) => s,
        Err(e) if e.error_len().is_none() => std::str::from_utf8(&buf[..e.valid_up_to()]).ok()?,
        Err(_) => return None, // not UTF-8
    };

    let preview: Vec<String> = text
        .lines()
        .take(max_lines)
        .map(|l| l.replace('\t', "    ").trim_end().to_owned())
        .collect();
    (!preview.iter().all(|l| l.is_empty())).then(|| preview.join("\n"))
}
