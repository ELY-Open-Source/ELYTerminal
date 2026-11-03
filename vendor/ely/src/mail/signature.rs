use std::rc::Rc;

use gpui::{
    App, Context, Div, ElementId, Entity, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Subscription, Window, div, prelude::*,
};

use crate::{
    forms::{Input, InputEvent, OnValue, TextInput},
    theme::{ActiveTheme, TextSize},
};

/// A signature as it sits under a message: a short rule, then its lines in the muted tone.
pub(super) fn signature_block(text: &str, cx: &App) -> Div {
    let theme = cx.theme();
    let colors = &theme.colors;
    div()
        .flex()
        .flex_col()
        .gap_1()
        .text_size(theme.text_size(TextSize::Sm))
        .text_color(colors.fg_muted)
        .child(
            div()
                .w_6()
                .mb_1()
                .border_t_1()
                .border_color(colors.border_strong),
        )
        .children(text.lines().map(|line| div().child(line.to_string())))
}

/// The field and what it last told the owner, so an echo is known from a new signature.
struct Signed {
    input: Entity<TextInput>,
    seed: SharedString,
    sent: Option<SharedString>,
    on_change: Option<OnValue>,
    _changes: Subscription,
}

/// Writes a signature: its lines in a field, and under it how it will sit below a message. The owner gets the words as they change; a new signature from the owner starts the field over.
#[derive(IntoElement)]
pub struct SignatureEditor {
    id: ElementId,
    text: SharedString,
    on_change: Option<OnValue>,
}

impl SignatureEditor {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            on_change: None,
        }
    }

    /// Gets the words as they change.
    pub fn on_change(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SignatureEditor {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let seed = self.text.clone();
        let signed = window.use_keyed_state(
            (self.id.clone(), "signed"),
            cx,
            |window, cx: &mut Context<Signed>| {
                let input = cx.new(|cx| {
                    let mut input = TextInput::new(window, cx)
                        .multi_line(3, 6)
                        .placeholder("Your name, your role, a way to reach you");
                    input.set_text(seed.to_string(), cx);
                    input
                });
                let changes =
                    cx.subscribe_in(&input, window, |signed, input, event, window, cx| {
                        if *event != InputEvent::Changed {
                            return;
                        }
                        let words = SharedString::from(input.read(cx).text().to_string());
                        if words == signed.seed {
                            return;
                        }
                        log::info!("signature editor: {} characters", words.len());
                        signed.sent = Some(words.clone());
                        if let Some(on_change) = signed.on_change.clone() {
                            on_change(&words, window, cx);
                        }
                        cx.notify();
                    });
                Signed {
                    input,
                    seed: seed.clone(),
                    sent: None,
                    on_change: None,
                    _changes: changes,
                }
            },
        );
        let text = self.text.clone();
        signed.update(cx, |signed, cx| {
            signed.on_change = self.on_change.clone();
            if signed.seed == text {
                return;
            }
            signed.seed = text.clone();
            if signed.sent.as_ref() == Some(&text) {
                return;
            }
            log::info!("signature editor: started over");
            signed
                .input
                .update(cx, |input, cx| input.set_text(text.to_string(), cx));
        });
        let input = signed.read(cx).input.clone();
        let words = input.read(cx).text().to_string();
        let theme = cx.theme();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(Input::new(&input))
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(theme.colors.fg_subtle)
                    .child("Under each message"),
            )
            .child(signature_block(&words, cx))
    }
}
