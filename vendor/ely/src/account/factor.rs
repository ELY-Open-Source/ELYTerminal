use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
};

use super::login::Run;
use crate::{
    feedback::InlineMessage,
    forms::PinInput,
    primitives::Severity,
    theme::{ActiveTheme, TextSize},
    typography::Link,
};

type OnText = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// Digits in a code from an authenticator app.
const DIGITS: usize = 6;

/// The code from an authenticator app, in six boxes. A full code goes to the owner at once; after one fails the boxes clear for the next, and a recovery code can stand in.
#[derive(IntoElement)]
pub struct TwoFactorInput {
    id: ElementId,
    failed: usize,
    busy: bool,
    on_code: OnText,
    on_recovery: Option<Run>,
}

impl TwoFactorInput {
    pub fn new(
        id: impl Into<ElementId>,
        on_code: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            failed: 0,
            busy: false,
            on_code: Rc::new(on_code),
            on_recovery: None,
        }
    }

    /// How many codes have failed; each new failure clears the boxes and says so.
    pub fn failed(mut self, failed: usize) -> Self {
        self.failed = failed;
        self
    }

    /// While the owner checks a code.
    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    /// Shows "Use a recovery code".
    pub fn on_recovery(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_recovery = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for TwoFactorInput {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let on_code = self.on_code;
        let theme = cx.theme();
        let note = match (self.busy, self.failed) {
            (true, _) => Some(
                div()
                    .text_color(theme.colors.fg_muted)
                    .child("Checking the code…")
                    .into_any_element(),
            ),
            (false, 0) => None,
            (false, _) => Some(
                InlineMessage::new(
                    Severity::Danger,
                    "That code did not work. Try the one showing now.",
                )
                .into_any_element(),
            ),
        };
        div()
            .flex()
            .flex_col()
            .gap_3()
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div().flex().child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_color(theme.colors.fg_muted)
                        .child("Enter the six-digit code from your authenticator app."),
                ),
            )
            .child(
                PinInput::new((id.clone(), "code"), DIGITS)
                    .attempt(self.failed)
                    .on_complete(move |code, window, cx| {
                        log::info!("two-factor input: code entered");
                        on_code(&SharedString::from(code.to_string()), window, cx);
                    }),
            )
            .children(note)
            .children(self.on_recovery.map(|run| {
                div().child(Link::new(
                    (id.clone(), "recovery"),
                    "Use a recovery code",
                    move |_, window, cx| run(window, cx),
                ))
            }))
    }
}
