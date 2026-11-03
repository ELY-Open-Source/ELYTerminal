use std::rc::Rc;

use gpui::{
    Animation, AnimationExt, App, ElementId, FontWeight, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::*,
};

use crate::{
    motion,
    primitives::{Backdrop, FocusScope, Place, give_back, take_focus},
    theme::{ActiveTheme, ControlSize, Elevation, Radius, TextSize},
};

type Run = Rc<dyn Fn(&mut Window, &mut App)>;

/// A short list of choices rising from the bottom, with its own cancel.
#[derive(IntoElement)]
pub struct ActionSheet {
    id: ElementId,
    title: Option<SharedString>,
    message: Option<SharedString>,
    actions: Vec<(SharedString, bool, Run)>,
    on_close: Run,
}

impl ActionSheet {
    pub fn new(
        id: impl Into<ElementId>,
        on_close: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            title: None,
            message: None,
            actions: Vec::new(),
            on_close: Rc::new(on_close),
        }
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn message(mut self, message: impl Into<SharedString>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Runs `handler`, then closes.
    pub fn action(
        mut self,
        label: impl Into<SharedString>,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.actions.push((label.into(), false, Rc::new(handler)));
        self
    }

    /// An action that destroys something, set in the danger tone.
    pub fn destructive(
        mut self,
        label: impl Into<SharedString>,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.actions.push((label.into(), true, Rc::new(handler)));
        self
    }
}

impl RenderOnce for ActionSheet {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let takeover = take_focus(self.id.clone(), window, cx);
        let close: Run = {
            let (takeover, on_close) = (takeover.clone(), self.on_close);
            Rc::new(move |window, cx| {
                give_back(&takeover, window, cx);
                on_close(window, cx)
            })
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let card = || {
            div()
                .flex()
                .flex_col()
                .overflow_hidden()
                .rounded(theme.radius(Radius::Xl))
                .bg(colors.overlay)
                .border_1()
                .border_color(colors.border)
                .shadow(theme.elevation(Elevation::Modal))
        };
        let row = |id: ElementId, label: SharedString, fg, strong: bool| {
            div()
                .id(id)
                .flex()
                .items_center()
                .justify_center()
                .h(theme.control_height(ControlSize::Lg)
                    + theme.control_height(ControlSize::Sm) / 2.0)
                .border_t_1()
                .border_color(colors.border)
                .text_size(theme.text_size(TextSize::Md))
                .font_weight(if strong {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::NORMAL
                })
                .text_color(fg)
                .cursor_pointer()
                .tab_index(0)
                .focus(|style| style.bg(colors.active))
                .hover(|style| style.bg(colors.hover))
                .child(label)
        };
        let heading = (self.title.is_some() || self.message.is_some()).then(|| {
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap_1()
                .px_6()
                .py_4()
                .text_center()
                .when_some(self.title, |head, title| {
                    head.child(
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.fg_muted)
                            .child(title),
                    )
                })
                .when_some(self.message, |head, message| {
                    head.child(
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(colors.fg_subtle)
                            .child(message),
                    )
                })
        });
        let rows: Vec<_> = self
            .actions
            .into_iter()
            .enumerate()
            .map(|(ix, (label, danger, run))| {
                let close = close.clone();
                let fg = if danger { colors.danger } else { colors.fg };
                row(("sheet-action", ix).into(), label.clone(), fg, false).on_click(
                    move |_, window, cx| {
                        log::info!("action sheet: {label}");
                        run(window, cx);
                        close(window, cx);
                    },
                )
            })
            .collect();
        let (cancel, escape, dismiss) = (close.clone(), close.clone(), close);
        let focus = takeover.read(cx).focus.clone();
        let panel = div()
            .id("action-sheet")
            .w(theme.sheet_size())
            .pb_4()
            .on_key_down(move |event, window, cx| {
                if event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    escape(window, cx);
                }
            })
            .child(
                FocusScope::new(&focus)
                    .trap()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(card().map(|card| match heading {
                        Some(heading) => card.child(heading).children(rows),
                        None => card.children(rows),
                    }))
                    .child(
                        card().child(
                            row("sheet-cancel".into(), "Cancel".into(), colors.fg, true)
                                .border_t_0()
                                .on_click(move |_, window, cx| cancel(window, cx)),
                        ),
                    ),
            )
            .with_animation(
                "action-sheet-in",
                Animation::new(motion::duration(motion::BASE, cx))
                    .with_easing(motion::ease_out_cubic),
                |panel, t| panel.opacity(t).mb(-(motion::NUDGE * 6.0 * (1.0 - t))),
            );
        Backdrop::new(self.id)
            .place(Place::Bottom)
            .on_dismiss(move |window, cx| dismiss(window, cx))
            .child(div().w_full().flex().justify_center().child(panel))
    }
}
