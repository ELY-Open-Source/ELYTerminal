use anyhow::Context as _;
use gpui::{
    AnyView, AnyWindowHandle, App, AppContext, Bounds, Context, Entity, FocusHandle, Global,
    InteractiveElement, IntoElement, ParentElement, Pixels, Point, Render, SharedString, Size,
    Styled, TitlebarOptions, Window, WindowBounds, WindowHandle, WindowKind, WindowOptions, div,
    point, prelude::*,
};

use super::drag_region;
use crate::{
    buttons::IconButton,
    primitives::{FocusScope, IconName},
    theme::{ActiveTheme, ControlSize, TextSize},
};

fn titlebar(title: SharedString, cx: &App) -> TitlebarOptions {
    TitlebarOptions {
        title: Some(title),
        appears_transparent: true,
        traffic_light_position: Some(cx.theme().traffic_light_origin()),
    }
}

/// A window whose content is one component. Escape closes it.
pub struct Hosted<T> {
    pub content: T,
    focus: FocusHandle,
}

impl<T> Hosted<T> {
    pub(crate) fn new(content: T, focus: FocusHandle) -> Self {
        Self { content, focus }
    }
}

impl<T: IntoElement + Clone + 'static> Render for Hosted<T> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bar = drag_region("hosted-bar", window, cx);
        let theme = cx.theme();
        div()
            .id("hosted")
            .size_full()
            .on_key_down(|event, window, _| {
                if event.keystroke.key == "escape" {
                    window.remove_window();
                }
            })
            .child(
                FocusScope::new(&self.focus)
                    .root()
                    .size_full()
                    .flex()
                    .flex_col()
                    .bg(theme.colors.bg)
                    .text_color(theme.colors.fg)
                    .font_family(theme.font_family.clone())
                    .child(bar.flex_none().h(theme.titlebar_height()))
                    .child(self.content.clone()),
            )
    }
}

/// Opens `content` alone in a fixed-size window, centered.
pub fn open_hosted<T: IntoElement + Clone + 'static>(
    title: impl Into<SharedString>,
    size: Size<Pixels>,
    content: T,
    cx: &mut App,
) -> anyhow::Result<WindowHandle<Hosted<T>>> {
    let title = title.into();
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size, cx))),
        titlebar: Some(titlebar(title.clone(), cx)),
        is_resizable: false,
        is_minimizable: false,
        ..Default::default()
    };
    let handle = cx.open_window(options, |window, cx| {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        cx.new(|_| Hosted::new(content, focus))
    })?;
    log::info!("window: opened {title}");
    Ok(handle)
}

/// Windows opened through it, oldest first, with where each opened.
#[derive(Default)]
pub struct WindowManager {
    windows: Vec<(AnyWindowHandle, SharedString, Point<Pixels>)>,
}

impl Global for WindowManager {}

impl WindowManager {
    /// Opens a window one title bar below and right of the newest.
    pub fn open<V: Render + 'static>(
        title: impl Into<SharedString>,
        size: Size<Pixels>,
        cx: &mut App,
        build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
    ) -> anyhow::Result<WindowHandle<V>> {
        let title = title.into();
        Self::windows(cx);
        let theme = cx.theme();
        let step = theme.titlebar_height().to_pixels(theme.base_rem());
        let newest = cx
            .default_global::<WindowManager>()
            .windows
            .last()
            .map(|(_, _, at)| *at);
        let bounds = match newest {
            Some(at) => Bounds::new(at + point(step, step), size),
            None => Bounds::centered(None, size, cx),
        };
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(titlebar(title.clone(), cx)),
            ..Default::default()
        };
        let handle = cx.open_window(options, build)?;
        cx.default_global::<WindowManager>().windows.push((
            handle.into(),
            title.clone(),
            bounds.origin,
        ));
        log::info!("window manager: opened {title}");
        Ok(handle)
    }

    /// Managed windows still open, oldest first.
    pub fn windows(cx: &mut App) -> Vec<(AnyWindowHandle, SharedString)> {
        let open = cx.windows();
        let manager = cx.default_global::<WindowManager>();
        manager.windows.retain(|(handle, ..)| open.contains(handle));
        manager
            .windows
            .iter()
            .map(|(handle, title, _)| (*handle, title.clone()))
            .collect()
    }

    pub fn focus(handle: AnyWindowHandle, cx: &mut App) -> anyhow::Result<()> {
        log::info!("window manager: focus {:?}", handle.window_id());
        handle.update(cx, |_, window, _| window.activate_window())
    }
}

/// Screen corner for a mini window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// A small window above all others, around a host view.
pub struct MiniWindow {
    title: SharedString,
    view: AnyView,
    back: Option<AnyWindowHandle>,
}

impl MiniWindow {
    /// Pinned to a display corner or centered; expanding returns to the opener.
    pub fn open(
        title: impl Into<SharedString>,
        size: Size<Pixels>,
        corner: Option<Corner>,
        view: AnyView,
        cx: &mut App,
    ) -> anyhow::Result<WindowHandle<MiniWindow>> {
        let title = title.into();
        let back = cx.active_window();
        let area = cx
            .primary_display()
            .context("a mini window needs a display")?
            .bounds();
        let margin = cx.theme().window_margin();
        let (left, top) = (area.left() + margin, area.top() + margin * 2.0);
        let (right, bottom) = (
            area.right() - margin - size.width,
            area.bottom() - margin - size.height,
        );
        let bounds = match corner {
            Some(Corner::TopLeft) => Bounds::new(point(left, top), size),
            Some(Corner::TopRight) => Bounds::new(point(right, top), size),
            Some(Corner::BottomLeft) => Bounds::new(point(left, bottom), size),
            Some(Corner::BottomRight) => Bounds::new(point(right, bottom), size),
            None => Bounds::centered(None, size, cx),
        };
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: None,
            kind: WindowKind::PopUp,
            is_resizable: false,
            is_minimizable: false,
            ..Default::default()
        };
        let handle = cx.open_window(options, |_, cx| {
            cx.new(|_| MiniWindow {
                title: title.clone(),
                view,
                back,
            })
        })?;
        log::info!("mini window: opened {title} at {corner:?}");
        Ok(handle)
    }
}

impl Render for MiniWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let drag = drag_region("mini-bar", window, cx);
        let theme = cx.theme();
        let back = self.back;
        div()
            .id("mini-window")
            .group("mini-window")
            .relative()
            .size_full()
            .bg(theme.colors.bg)
            .text_color(theme.colors.fg)
            .font_family(theme.font_family.clone())
            .child(self.view.clone())
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .flex()
                    .items_center()
                    .gap_1()
                    .h(theme.control_height(ControlSize::Lg))
                    .px_2()
                    .bg(theme.colors.overlay.opacity(0.92))
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(theme.colors.fg_muted)
                    .invisible()
                    .group_hover("mini-window", |style| style.visible())
                    // Only the title drags: Windows takes the region as caption.
                    .child(
                        drag.flex_1()
                            .h_full()
                            .flex()
                            .items_center()
                            .child(self.title.clone()),
                    )
                    .when_some(back, |bar, back| {
                        bar.child(
                            IconButton::new("mini-expand", IconName::Maximize2)
                                .size(ControlSize::Sm)
                                .on_click(move |_, window, cx| {
                                    window.remove_window();
                                    let returned =
                                        back.update(cx, |_, window, _| window.activate_window());
                                    if let Err(error) = returned {
                                        log::warn!(
                                            "mini window: no window to return to: {error:#}"
                                        );
                                    }
                                }),
                        )
                    })
                    .child(
                        IconButton::new("mini-close", IconName::X)
                            .size(ControlSize::Sm)
                            .on_click(|_, window, _| window.remove_window()),
                    ),
            )
    }
}
