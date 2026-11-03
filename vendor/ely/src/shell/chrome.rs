use std::rc::Rc;

use gpui::{
    AnyElement, App, Div, ElementId, FontWeight, InteractiveElement, IntoElement, MouseButton,
    ParentElement, RenderOnce, SharedString, Stateful, StatefulInteractiveElement, Styled, Window,
    WindowControlArea, div, prelude::*,
};
use smallvec::SmallVec;

use crate::{
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, Platform, TextSize},
    typography::Ellipsis,
};

/// Drags the window, and zooms it on double-click.
pub fn drag_region(id: impl Into<ElementId>, window: &mut Window, cx: &mut App) -> Stateful<Div> {
    let id = id.into();
    let pressed = window.use_keyed_state(id.clone(), cx, |_, _| false);
    let (down, moved, up, out) = (pressed.clone(), pressed.clone(), pressed.clone(), pressed);
    div()
        .id(id)
        .window_control_area(WindowControlArea::Drag)
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            // A button inside claims its own press.
            if !window.default_prevented() {
                down.update(cx, |pressed, _| *pressed = true)
            }
        })
        .on_mouse_move(move |_, window, cx| {
            if *moved.read(cx) {
                moved.update(cx, |pressed, _| *pressed = false);
                window.start_window_move();
            }
        })
        .on_mouse_up(MouseButton::Left, move |_, _, cx| {
            up.update(cx, |pressed, _| *pressed = false)
        })
        .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
            out.update(cx, |pressed, _| *pressed = false)
        })
        .on_click(|event, window, _| {
            if !event.standard_click() || event.click_count() != 2 {
                return;
            }
            if cfg!(target_os = "macos") {
                window.titlebar_double_click();
            } else if cfg!(target_os = "linux") {
                window.zoom_window();
            }
        })
}

#[derive(Clone, Copy, Debug)]
enum Control {
    Minimize,
    Maximize,
    Close,
}

pub(super) type OnClose = Rc<dyn Fn(&mut Window, &mut App)>;

fn run(control: Control, on_close: Option<&OnClose>, window: &mut Window, cx: &mut App) {
    log::info!("window controls: {control:?}");
    match (control, on_close) {
        (Control::Minimize, _) => window.minimize_window(),
        (Control::Maximize, _) => window.zoom_window(),
        (Control::Close, Some(close)) => close(window, cx),
        (Control::Close, None) => window.remove_window(),
    }
}

/// Minimize, maximize and close, drawn in one system's manner.
#[derive(IntoElement)]
pub struct WindowControls {
    id: ElementId,
    platform: Option<Platform>,
    on_close: Option<OnClose>,
}

impl WindowControls {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            platform: None,
            on_close: None,
        }
    }

    /// Draws this platform's buttons instead of the theme's.
    pub fn platform(mut self, platform: Platform) -> Self {
        self.platform = Some(platform);
        self
    }

    /// Replaces closing the window, e.g. to confirm first.
    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for WindowControls {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let controls = match self.platform.unwrap_or(cx.theme().platform) {
            Platform::Mac => traffic_lights(self.id, self.on_close, window, cx),
            Platform::Windows => caption_buttons(self.id, self.on_close, window, cx),
            Platform::Linux => round_buttons(self.id, self.on_close, window, cx),
        };
        // Windows takes a drag region under controls as caption.
        controls.occlude()
    }
}

fn traffic_lights(
    id: ElementId,
    on_close: Option<OnClose>,
    window: &mut Window,
    cx: &mut App,
) -> Stateful<Div> {
    let theme = cx.theme();
    let colors = &theme.colors;
    let active = window.is_window_active();
    let lights = [
        (Control::Close, colors.danger, "×"),
        (Control::Minimize, colors.warning, "−"),
        (Control::Maximize, colors.success, "+"),
    ];
    let group = SharedString::from(format!("traffic-{id}"));
    div()
        .id(id)
        .group(group.clone())
        .flex()
        .items_center()
        .gap_2()
        .children(lights.map(|(control, tint, glyph)| {
            let on_close = on_close.clone();
            div()
                .id(SharedString::from(format!("{control:?}")))
                .size(theme.traffic_light())
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(if active { tint } else { colors.border_strong })
                .border_1()
                .border_color(colors.shadow.opacity(0.12))
                .text_size(theme.text_size(TextSize::Xs))
                .line_height(theme.traffic_light())
                .font_weight(FontWeight::BOLD)
                .text_color(gpui::transparent_black())
                .group_hover(group.clone(), |style| {
                    style.text_color(colors.shadow.opacity(0.6))
                })
                .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                .on_click(move |_, window, cx| run(control, on_close.as_ref(), window, cx))
                .child(glyph)
        }))
}

fn caption_buttons(
    id: ElementId,
    on_close: Option<OnClose>,
    window: &mut Window,
    cx: &mut App,
) -> Stateful<Div> {
    let theme = cx.theme();
    let colors = &theme.colors;
    let maximize = if window.is_maximized() {
        IconName::Copy
    } else {
        IconName::Square
    };
    let buttons = [
        (Control::Minimize, IconName::Minus, WindowControlArea::Min),
        (Control::Maximize, maximize, WindowControlArea::Max),
        (Control::Close, IconName::X, WindowControlArea::Close),
    ];
    let height = theme.caption_button_height(window.rem_size());
    div()
        .id(id)
        .flex()
        .flex_none()
        .children(buttons.map(|(control, icon, area)| {
            let on_close = on_close.clone();
            let group = SharedString::from(format!("caption-{control:?}"));
            let (hover, glyph) = match control {
                Control::Close => (colors.danger, colors.on_accent),
                _ => (colors.hover, colors.fg),
            };
            div()
                .id(group.clone())
                .group(group.clone())
                .flex()
                .items_center()
                .justify_center()
                .w(theme.caption_button_width())
                .h(height)
                .debug_selector(move || format!("caption-{control:?}"))
                .window_control_area(area)
                .hover(|style| style.bg(hover))
                .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                .on_click(move |_, window, cx| run(control, on_close.as_ref(), window, cx))
                .child(
                    Icon::new(icon)
                        .size(IconSize::Sm)
                        .color(colors.fg_muted)
                        .group_hover_color(group, glyph),
                )
        }))
}

fn round_buttons(
    id: ElementId,
    on_close: Option<OnClose>,
    window: &mut Window,
    cx: &mut App,
) -> Stateful<Div> {
    let theme = cx.theme();
    let colors = &theme.colors;
    let offered = window.window_controls();
    let buttons = [
        (Control::Minimize, IconName::Minus, offered.minimize),
        (Control::Maximize, IconName::Square, offered.maximize),
        (Control::Close, IconName::X, true),
    ];
    div().id(id).flex().items_center().gap_2().children(
        buttons
            .into_iter()
            .filter(|(_, _, shown)| *shown)
            .map(|(control, icon, _)| {
                let on_close = on_close.clone();
                div()
                    .id(SharedString::from(format!("{control:?}")))
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(theme.control_height(ControlSize::Sm))
                    .rounded_full()
                    .bg(colors.hover)
                    .hover(|style| style.bg(colors.active))
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .on_click(move |_, window, cx| run(control, on_close.as_ref(), window, cx))
                    .child(Icon::new(icon).size(IconSize::Xs).color(colors.fg))
            }),
    )
}

/// A slot's controls, occluding the drag region beneath.
fn slot(items: SmallVec<[AnyElement; 2]>) -> Option<Div> {
    (!items.is_empty()).then(|| {
        div()
            .flex()
            .items_center()
            .gap_2()
            .occlude()
            .children(items)
    })
}

/// Whether the system draws the window controls, and whose manner they take.
pub(super) fn controls_style(platform: Option<Platform>, cx: &App) -> (bool, Platform) {
    let system = platform.is_none() && cfg!(target_os = "macos");
    match system {
        true => (true, Platform::Mac),
        false => (false, platform.unwrap_or(cx.theme().platform)),
    }
}

/// The live close handler, read by the window's close request.
struct CloseRoute {
    handler: Option<OnClose>,
    registered: bool,
}

/// Sends system closes to `handler` while the bar lives and holds one.
pub(super) fn route_close(
    id: &ElementId,
    handler: Option<OnClose>,
    window: &mut Window,
    cx: &mut App,
) {
    let route = window.use_keyed_state(id.clone(), cx, |_, _| CloseRoute {
        handler: None,
        registered: false,
    });
    let register = handler.is_some() && !route.read(cx).registered;
    route.update(cx, |route, _| {
        route.handler = handler;
        route.registered |= register;
    });
    if register {
        let live = route.downgrade();
        window.on_window_should_close(cx, move |window, cx| {
            let handler = live
                .upgrade()
                .and_then(|route| route.read(cx).handler.clone());
            let Some(close) = handler else {
                return true;
            };
            log::info!("title bar: system close routed to on_close");
            close(window, cx);
            false
        });
    }
}

/// Window title bar. Its empty space drags the window.
#[derive(IntoElement)]
pub struct TitleBar {
    id: ElementId,
    title: Option<SharedString>,
    platform: Option<Platform>,
    on_close: Option<OnClose>,
    leading: SmallVec<[AnyElement; 2]>,
    actions: SmallVec<[AnyElement; 2]>,
}

impl TitleBar {
    /// On macOS the system draws the traffic lights; the bar leaves them room.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            title: None,
            platform: None,
            on_close: None,
            leading: SmallVec::new(),
            actions: SmallVec::new(),
        }
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Draws this platform's controls instead of the system's.
    pub fn platform(mut self, platform: Platform) -> Self {
        self.platform = Some(platform);
        self
    }

    /// Runs instead of closing; without a forced platform, system closes run it too.
    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }

    /// Right-aligned content before the window controls.
    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }
}

impl ParentElement for TitleBar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.leading.extend(elements);
    }
}

impl RenderOnce for TitleBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if self.platform.is_none() {
            route_close(&self.id, self.on_close.clone(), window, cx);
        }
        let bar = drag_region(self.id, window, cx);
        let (system, style) = controls_style(self.platform, cx);
        let theme = cx.theme();
        let mut trailing = (!system).then_some(WindowControls {
            id: "window-controls".into(),
            platform: Some(style),
            on_close: self.on_close,
        });
        let leading = if style == Platform::Mac {
            trailing.take()
        } else {
            None
        };
        let title = self.title.map(|title| {
            div()
                .min_w_0()
                .text_size(theme.text_size(TextSize::Sm))
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme.colors.fg_muted)
                .debug_selector(|| "titlebar-title".into())
                .child(Ellipsis::new(title))
        });
        let centered = style != Platform::Windows;
        let inset = theme.traffic_light_inset();
        let fullscreen = window.is_fullscreen();
        let bar = bar
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .gap_2()
            .h(theme.titlebar_height())
            .bg(theme.colors.bg)
            .border_b_1()
            .border_color(theme.colors.border);
        let bar = bar.when(!centered, |bar| bar.pl_3());
        let start = div().flex().child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .when(centered, |start| match system && !fullscreen {
                    true => start.pl(inset),
                    false => start.pl_3(),
                })
                .children(leading)
                .children(slot(self.leading)),
        );
        let end = div().flex().justify_end().child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .when(centered, |end| end.pr_2())
                .children(slot(self.actions))
                .children(trailing.map(|controls| {
                    div()
                        .flex_none()
                        .debug_selector(|| "titlebar-controls".into())
                        .child(controls)
                })),
        );
        let middle = match title {
            Some(title) if centered => title,
            Some(title) => title.flex_1(),
            None => div().flex_1(),
        };
        match centered {
            true => bar.child(start.flex_1()).child(middle).child(end.flex_1()),
            false => bar
                .child(start.flex_none())
                .child(middle)
                .child(end.flex_none()),
        }
    }
}
