use std::rc::Rc;

use gpui::{
    App, ElementId, Entity, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use super::button::{keystroke_of, shortcut_text};
use crate::{
    primitives::{FocusRing, Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
};

type Run = Rc<dyn Fn(&mut Window, &mut App)>;

/// One quick action: what it is, and its shortcut.
#[derive(Clone)]
pub struct QuickAction {
    icon: IconName,
    label: SharedString,
    note: Option<SharedString>,
    shortcut: Option<SharedString>,
    run: Run,
}

impl QuickAction {
    pub fn new(
        icon: IconName,
        label: impl Into<SharedString>,
        run: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            icon,
            label: label.into(),
            note: None,
            shortcut: None,
            run: Rc::new(run),
        }
    }

    pub fn note(mut self, note: impl Into<SharedString>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Panics on a bad keystroke.
    pub fn shortcut(mut self, keystroke: &str) -> Self {
        keystroke_of(keystroke);
        self.shortcut = Some(SharedString::from(keystroke.to_string()));
        self
    }
}

/// A list of actions. The pointer or the arrows choose; Enter or a click runs.
#[derive(IntoElement)]
pub struct QuickActions {
    id: ElementId,
    actions: Vec<QuickAction>,
}

impl QuickActions {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            actions: Vec::new(),
        }
    }

    pub fn action(mut self, action: QuickAction) -> Self {
        self.actions.push(action);
        self
    }
}

struct Pick {
    selected: usize,
    focus: gpui::FocusHandle,
}

fn run(action: &QuickAction, window: &mut Window, cx: &mut App) {
    log::info!("quick actions: {}", action.label);
    (action.run)(window, cx);
}

fn select(state: &Entity<Pick>, ix: usize, cx: &mut App) {
    if state.read(cx).selected != ix {
        state.update(cx, |pick, cx| {
            pick.selected = ix;
            cx.notify();
        });
    }
}

impl RenderOnce for QuickActions {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let count = self.actions.len();
        assert!(count > 0, "quick actions {:?} hold no action", self.id);
        let state = window.use_keyed_state(self.id.clone(), cx, |_, cx| Pick {
            selected: 0,
            focus: cx.focus_handle().tab_stop(true),
        });
        let (selected, focus) = {
            let pick = state.read(cx);
            (pick.selected.min(count - 1), pick.focus.clone())
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let rows: Vec<_> = self
            .actions
            .iter()
            .enumerate()
            .map(|(ix, action)| {
                let (hover, action_for_click) = (state.clone(), action.clone());
                div()
                    .id(("quick", ix))
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_2()
                    .py_1p5()
                    .rounded(theme.radius(Radius::Md))
                    .when(ix == selected, |row| row.bg(colors.hover))
                    .cursor_pointer()
                    .on_hover(move |hovered, _, cx| {
                        if *hovered {
                            select(&hover, ix, cx);
                        }
                    })
                    .on_click(move |_, window, cx| run(&action_for_click, window, cx))
                    .child(
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .justify_center()
                            .size(theme.control_height(ControlSize::Md))
                            .rounded(theme.radius(Radius::Md))
                            .bg(colors.sunken)
                            .border_1()
                            .border_color(colors.border)
                            .child(
                                Icon::new(action.icon)
                                    .size(IconSize::Sm)
                                    .color(colors.fg_muted),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(theme.text_size(TextSize::Base))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(colors.fg)
                                    .child(action.label.clone()),
                            )
                            .when_some(action.note.clone(), |text, note| {
                                text.child(
                                    div()
                                        .text_size(theme.text_size(TextSize::Xs))
                                        .text_color(colors.fg_subtle)
                                        .child(note),
                                )
                            }),
                    )
                    .when_some(action.shortcut.as_ref(), |row, stroke| {
                        row.child(
                            div()
                                .text_size(theme.text_size(TextSize::Xs))
                                .text_color(colors.fg_subtle)
                                .child(shortcut_text(stroke, theme.platform)),
                        )
                    })
            })
            .collect();
        let (keys, actions) = (state, self.actions);
        div()
            .id(self.id)
            .track_focus(&focus)
            .flex()
            .flex_col()
            .gap_0p5()
            .p_1()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .focus_ring(cx)
            .on_key_down(move |event, window, cx| {
                let current = keys.read(cx).selected.min(count - 1);
                match event.keystroke.key.as_str() {
                    "down" => select(&keys, (current + 1) % count, cx),
                    "up" => select(&keys, (current + count - 1) % count, cx),
                    "enter" => run(&actions[current], window, cx),
                    _ => return,
                }
                cx.stop_propagation();
            })
            .children(rows)
    }
}
