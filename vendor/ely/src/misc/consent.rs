use std::{cell::Cell, rc::Rc};

use gpui::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
    prelude::*,
};

use crate::{
    buttons::{Button, ButtonVariant},
    forms::{Checkbox, Run},
    overlays::Dialog,
    theme::{ActiveTheme, TextSize},
};

/// Terms to read and agree to, over the page: a box says the viewer agrees, and Accept asks for the box until it is ticked. Each way out answers once: Accept accepts, and Decline, Escape or a press on the scrim declines. Render it while open.
#[derive(IntoElement)]
pub struct ConsentDialog {
    id: ElementId,
    title: SharedString,
    terms: SharedString,
    agreement: SharedString,
    on_accept: Run,
    on_decline: Run,
}

impl ConsentDialog {
    pub fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        terms: impl Into<SharedString>,
        on_accept: impl Fn(&mut Window, &mut App) + 'static,
        on_decline: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            terms: terms.into(),
            agreement: "I have read and agree to these terms.".into(),
            on_accept: Rc::new(on_accept),
            on_decline: Rc::new(on_decline),
        }
    }

    /// What the box says the viewer agrees to.
    pub fn agreement(mut self, text: impl Into<SharedString>) -> Self {
        self.agreement = text.into();
        self
    }
}

impl RenderOnce for ConsentDialog {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let state = window.use_keyed_state((id.clone(), "agreed"), cx, |_, _| (false, false));
        let (agreed, nudged) = *state.read(cx);
        let theme = cx.theme();
        let colors = &theme.colors;
        let (ticked, asked) = (state.clone(), state);
        let (decline, accept) = (self.on_decline, self.on_accept);
        let (declined, accepted) = (id.clone(), id.clone());
        let answered = Rc::new(Cell::new(false));
        let answer = answered.clone();
        Dialog::new(id.clone(), self.title, move |window, cx| {
            if !answered.get() {
                log::info!("consent dialog {declined:?}: declined");
                decline(window, cx)
            }
        })
        .child(
            div().flex().child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(colors.fg)
                    .child(self.terms),
            ),
        )
        .child(
            Checkbox::new((id.clone(), "agree"), agreed)
                .label(self.agreement)
                .on_change(move |on, _, cx| {
                    ticked.update(cx, |state, cx| {
                        *state = (on, false);
                        cx.notify();
                    })
                }),
        )
        .when(nudged, |dialog| {
            dialog.child(
                div()
                    .debug_selector(|| "consent-tick-first".into())
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(colors.danger)
                    .child("Tick the box to agree first."),
            )
        })
        .action({
            let id = id.clone();
            move |close| {
                Button::new((id, "decline"), "Decline")
                    .variant(ButtonVariant::Ghost)
                    .on_click(move |_, window, cx| close(window, cx))
            }
        })
        .action(move |close| {
            Button::new((id, "accept"), "Accept")
                .primary()
                .on_click(move |_, window, cx| {
                    if !agreed {
                        asked.update(cx, |state, cx| {
                            state.1 = true;
                            cx.notify();
                        });
                        return;
                    }
                    log::info!("consent dialog {accepted:?}: accepted");
                    answer.set(true);
                    close(window, cx);
                    accept(window, cx);
                })
        })
    }
}
