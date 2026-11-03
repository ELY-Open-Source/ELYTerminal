use gpui::{
    Animation, AnimationExt, App, AppContext, Bounds, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, WindowBounds, WindowHandle, WindowOptions, div, prelude::*,
    relative,
};

use super::{Hosted, open_hosted};
use crate::{
    motion::{self, ProgressBar},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
    typography::{Caption, CopyableText, ExternalLink, Paragraph, Title},
};

fn mark(name: &SharedString, icon: Option<IconName>, cx: &App) -> impl IntoElement + use<> {
    let theme = cx.theme();
    let initial: SharedString = name.chars().take(1).collect::<String>().into();
    div()
        .flex()
        .items_center()
        .justify_center()
        .size(theme.control_height(ControlSize::Lg) * 2.0)
        .rounded(theme.radius(Radius::Xl))
        .bg(theme.colors.sunken)
        .border_1()
        .border_color(theme.colors.border)
        .text_size(theme.text_size(TextSize::Xxl))
        .text_color(theme.colors.fg)
        .map(|tile| match icon {
            Some(icon) => tile.child(Icon::new(icon).size(IconSize::Xxl)),
            None => tile.child(initial),
        })
}

/// Name, version, credits: the About window's content.
#[derive(IntoElement, Clone)]
pub struct AboutDialog {
    name: SharedString,
    version: SharedString,
    icon: Option<IconName>,
    description: Option<SharedString>,
    links: Vec<(SharedString, SharedString)>,
    copyright: Option<SharedString>,
}

impl AboutDialog {
    pub fn new(name: impl Into<SharedString>, version: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            icon: None,
            description: None,
            links: Vec::new(),
            copyright: None,
        }
    }

    /// App glyph; the name's initial stands in without one.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn description(mut self, text: impl Into<SharedString>) -> Self {
        self.description = Some(text.into());
        self
    }

    pub fn link(mut self, label: impl Into<SharedString>, url: impl Into<SharedString>) -> Self {
        self.links.push((label.into(), url.into()));
        self
    }

    pub fn copyright(mut self, text: impl Into<SharedString>) -> Self {
        self.copyright = Some(text.into());
        self
    }

    /// Shows it in its own window; Escape closes it.
    pub fn open(self, cx: &mut App) -> anyhow::Result<WindowHandle<Hosted<AboutDialog>>> {
        let size = cx.theme().about_window();
        open_hosted(format!("About {}", self.name), size, self, cx)
    }
}

impl RenderOnce for AboutDialog {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_2()
            .px_8()
            .pt_4()
            .pb_8()
            .child(mark(&self.name, self.icon, cx))
            .child(div().pt_3().child(Title::new(self.name.clone())))
            .child(CopyableText::new(
                "about-version",
                format!("Version {}", self.version),
            ))
            .when_some(self.description, |about, text| {
                about.child(div().pt_2().text_center().child(Paragraph::new(text)))
            })
            .when(!self.links.is_empty(), |about| {
                about.child(
                    div()
                        .flex()
                        .gap_4()
                        .pt_2()
                        .children(
                            self.links
                                .into_iter()
                                .enumerate()
                                .map(|(ix, (label, url))| {
                                    ExternalLink::new(("about-link", ix), label, url)
                                }),
                        ),
                )
            })
            .when_some(self.copyright, |about, text| {
                about.child(div().pt_4().child(Caption::new(text)))
            })
    }
}

/// First screen while an app loads: mark, name, progress.
#[derive(IntoElement, Clone)]
pub struct SplashScreen {
    name: SharedString,
    icon: Option<IconName>,
    tagline: Option<SharedString>,
    status: Option<SharedString>,
    progress: Option<f32>,
}

impl SplashScreen {
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            icon: None,
            tagline: None,
            status: None,
            progress: None,
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn tagline(mut self, text: impl Into<SharedString>) -> Self {
        self.tagline = Some(text.into());
        self
    }

    pub fn status(mut self, text: impl Into<SharedString>) -> Self {
        self.status = Some(text.into());
        self
    }

    /// 0 to 1; `None` shows a moving bar.
    pub fn progress(mut self, progress: Option<f32>) -> Self {
        if let Some(value) = progress {
            assert!(
                (0.0..=1.0).contains(&value),
                "splash progress {value} is not 0..=1"
            );
        }
        self.progress = progress;
        self
    }

    /// Shows it borderless and centered. Close it with `window.remove_window()`.
    pub fn open(self, cx: &mut App) -> anyhow::Result<WindowHandle<Hosted<SplashScreen>>> {
        let size = cx.theme().splash_window();
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size, cx))),
            titlebar: None,
            is_movable: false,
            is_resizable: false,
            is_minimizable: false,
            ..Default::default()
        };
        let handle = cx.open_window(options, |window, cx| {
            let focus = cx.focus_handle();
            window.focus(&focus, cx);
            cx.new(|_| Hosted::new(self, focus))
        })?;
        log::info!("splash: shown");
        Ok(handle)
    }
}

impl RenderOnce for SplashScreen {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let enter = motion::duration(motion::SLOW, cx);
        let track = match self.progress {
            Some(value) => ProgressBar::new("splash-progress", value).into_any_element(),
            None => ProgressBar::indeterminate("splash-progress").into_any_element(),
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .px_10()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .child(mark(&self.name, self.icon, cx))
                    .child(div().pt_2().child(Title::new(self.name.clone())))
                    .when_some(self.tagline, |block, text| block.child(Caption::new(text)))
                    .with_animation(
                        "splash-in",
                        Animation::new(enter).with_easing(motion::ease_out_cubic),
                        |block, t| block.opacity(t).mt(motion::NUDGE * 2.0 * (1.0 - t)),
                    ),
            )
            .child(
                div()
                    .w(relative(0.6))
                    .pt_8()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .child(div().w_full().child(track))
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg_subtle)
                            .child(self.status.unwrap_or_default()),
                    ),
            )
    }
}
