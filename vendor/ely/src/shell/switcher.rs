use std::rc::Rc;

use gpui::{
    Animation, AnimationExt, AnyWindowHandle, App, ElementId, Entity, InteractiveElement,
    IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled, Window, div,
    prelude::*,
};

use super::WindowManager;
use crate::{
    motion,
    primitives::{Backdrop, FocusNext, FocusPrev, Icon, IconName, Place, give_back, take_focus},
    theme::{ActiveTheme, ControlSize, Elevation, IconSize, Radius, TextSize},
    typography::Caption,
};

#[derive(Default)]
struct Switch {
    selected: usize,
}

type Close = Rc<dyn Fn(&mut Window, &mut App)>;

/// Lists managed windows over a scrim. Arrows or Tab move, Enter switches.
#[derive(IntoElement)]
pub struct WindowSwitcher {
    id: ElementId,
    on_close: Close,
}

impl WindowSwitcher {
    pub fn new(
        id: impl Into<ElementId>,
        on_close: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            on_close: Rc::new(on_close),
        }
    }
}

fn step(state: &Entity<Switch>, by: isize, count: usize, cx: &mut App) {
    if count == 0 {
        return;
    }
    state.update(cx, |switch, cx| {
        switch.selected = (switch.selected as isize + by).rem_euclid(count as isize) as usize;
        cx.notify();
    });
}

impl RenderOnce for WindowSwitcher {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let windows = WindowManager::windows(cx);
        let count = windows.len();
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| Switch::default());
        let takeover = take_focus(self.id.clone(), window, cx);
        if count > 0 && state.read(cx).selected >= count {
            state.update(cx, |switch, _| switch.selected = count - 1);
        }
        let selected = state.read(cx).selected;
        let close: Close = {
            let (takeover, on_close) = (takeover.clone(), self.on_close);
            Rc::new(move |window, cx| {
                give_back(&takeover, window, cx);
                on_close(window, cx)
            })
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let rows: Vec<_> = windows
            .iter()
            .enumerate()
            .map(|(ix, (handle, title))| {
                let (handle, close) = (*handle, close.clone());
                div()
                    .id(("switch", ix))
                    .flex()
                    .items_center()
                    .gap_3()
                    .h(theme.control_height(ControlSize::Lg))
                    .px_3()
                    .rounded(theme.radius(Radius::Md))
                    .text_size(theme.text_size(TextSize::Base))
                    .when(ix == selected, |row| row.bg(colors.hover))
                    .cursor_pointer()
                    .on_click(move |_, window, cx| switch_to(handle, &close, window, cx))
                    .child(
                        Icon::new(IconName::AppWindow)
                            .size(IconSize::Sm)
                            .color(colors.fg_muted),
                    )
                    .child(title.clone())
            })
            .collect();
        let handles: Vec<AnyWindowHandle> = windows.iter().map(|(handle, _)| *handle).collect();
        let (next, prev, keys, chosen) =
            (state.clone(), state.clone(), state.clone(), state.clone());
        let (close_keys, close_enter, close_scrim) = (close.clone(), close.clone(), close);
        let focus = takeover.read(cx).focus.clone();
        let panel = div()
            .id("window-switcher")
            .track_focus(&focus)
            .w(theme.sheet_size())
            .p_2()
            .flex()
            .flex_col()
            .gap_0p5()
            .rounded(theme.radius(Radius::Xl))
            .bg(colors.overlay)
            .border_1()
            .border_color(colors.border)
            .shadow(theme.elevation(Elevation::Modal))
            .on_action(move |_: &FocusNext, _, cx| step(&next, 1, count, cx))
            .on_action(move |_: &FocusPrev, _, cx| step(&prev, -1, count, cx))
            .on_key_down(move |event, window, cx| {
                match event.keystroke.key.as_str() {
                    "down" => step(&keys, 1, count, cx),
                    "up" => step(&keys, -1, count, cx),
                    "escape" => close_keys(window, cx),
                    "enter" if !event.keystroke.modifiers.modified() => {}
                    _ => return,
                }
                cx.stop_propagation();
            })
            .on_key_up(move |event, window, cx| {
                let stroke = &event.keystroke;
                if stroke.key == "enter" && !stroke.modifiers.modified() {
                    cx.stop_propagation();
                    if let Some(handle) = handles.get(chosen.read(cx).selected) {
                        switch_to(*handle, &close_enter, window, cx);
                    }
                }
            })
            .children(rows)
            .when(count == 0, |panel| {
                panel.child(
                    div()
                        .p_3()
                        .child(Caption::new("No managed windows are open.")),
                )
            })
            .with_animation(
                "switcher-in",
                Animation::new(motion::duration(motion::BASE, cx))
                    .with_easing(motion::ease_out_cubic),
                |panel, t| panel.opacity(t).mt(motion::NUDGE * (1.0 - t)),
            );
        Backdrop::new(self.id)
            .place(Place::Center)
            .on_dismiss(move |window, cx| close_scrim(window, cx))
            .child(panel)
    }
}

fn switch_to(handle: AnyWindowHandle, close: &Close, window: &mut Window, cx: &mut App) {
    close(window, cx);
    if handle == window.window_handle() {
        window.activate_window();
        return;
    }
    if let Err(error) = WindowManager::focus(handle, cx) {
        log::error!("window switcher: {error:#}");
    }
}
