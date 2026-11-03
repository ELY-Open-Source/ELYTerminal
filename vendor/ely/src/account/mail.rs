use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, Styled, Window, div,
};

use super::login::{Run, field};
use crate::{
    buttons::{Button, ButtonVariant},
    forms::{EmailInput, Enter, FormField, is_email},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize, TextSize},
    typography::Link,
};

type OnText = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// The words that set one email step apart from another.
struct Words {
    intro: &'static str,
    send: &'static str,
    sent: &'static str,
    note: fn(&str) -> String,
}

/// A way out: its label and what it runs.
type Way = Option<(&'static str, Run)>;

/// One email step: its words, where it stands, and its ways out. `back` shows in both steps, `other` once sent.
struct Step {
    id: ElementId,
    words: Words,
    sent: bool,
    busy: bool,
    on_send: OnText,
    back: Way,
    other: Way,
}

/// An address to send a link to, then where the link went with a way to send it again.
fn email_step(step: Step, window: &mut Window, cx: &mut App) -> AnyElement {
    let Step {
        id,
        words,
        sent,
        busy,
        on_send,
        back,
        other,
    } = step;
    let email = field(&id, "email", "name@example.com", false, window, cx);
    let address: SharedString = email.read(cx).text().trim().to_string().into();
    let ready = is_email(&address) && !busy;
    let send: Run = {
        let address = address.clone();
        Rc::new(move |window, cx| {
            log::info!("email step: send");
            on_send(&address, window, cx);
        })
    };
    let theme = cx.theme();
    let muted = |text: SharedString| {
        div().flex().child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(theme.colors.fg_muted)
                .child(text),
        )
    };
    let back = back.map(|(label, run)| {
        Link::new((id.clone(), "back"), label, move |_, window, cx| {
            run(window, cx)
        })
    });
    if sent {
        let other = other.map(|(label, run)| {
            Button::new((id.clone(), "other"), label)
                .variant(ButtonVariant::Ghost)
                .on_click(move |_, window, cx| run(window, cx))
        });
        return div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_3()
                    .child(
                        Icon::new(IconName::CircleCheck)
                            .size(IconSize::Md)
                            .color(theme.colors.success),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(div().font_weight(FontWeight::MEDIUM).child(words.sent))
                            .child(muted((words.note)(&address).into())),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        Button::new((id.clone(), "resend"), "Send again")
                            .loading(busy)
                            .disabled(busy)
                            .on_click(move |_, window, cx| send(window, cx)),
                    )
                    .children(other),
            )
            .children(back)
            .into_any_element();
    }
    let enter = send.clone();
    div()
        .flex()
        .flex_col()
        .gap_4()
        .capture_action(move |_: &Enter, window, cx| {
            if ready {
                cx.stop_propagation();
                enter(window, cx);
            }
        })
        .child(muted(words.intro.into()))
        .child(FormField::new((id.clone(), "email-field"), "Email").child(EmailInput::new(&email)))
        .child(
            Button::new((id.clone(), "send"), words.send)
                .variant(ButtonVariant::Primary)
                .full_width()
                .loading(busy)
                .disabled(!ready)
                .on_click(move |_, window, cx| send(window, cx)),
        )
        .children(back)
        .into_any_element()
}

/// Sign in by a link sent to an address: the address, then where the link went, with a way to send it again or to use another address.
#[derive(IntoElement)]
pub struct MagicLinkForm {
    id: ElementId,
    sent: bool,
    busy: bool,
    on_send: OnText,
    on_other: Option<Run>,
}

impl MagicLinkForm {
    pub fn new(
        id: impl Into<ElementId>,
        on_send: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            sent: false,
            busy: false,
            on_send: Rc::new(on_send),
            on_other: None,
        }
    }

    /// Once the owner has sent the link.
    pub fn sent(mut self, sent: bool) -> Self {
        self.sent = sent;
        self
    }

    /// While the owner sends.
    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    /// Shows "Use another email" once sent; the owner clears `sent`.
    pub fn on_other(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_other = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for MagicLinkForm {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let words = Words {
            intro: "We will email you a link that signs you in. No password needed.",
            send: "Email me a link",
            sent: "Check your email",
            note: |address| {
                format!("A sign-in link is on its way to {address}. Open it on this device.")
            },
        };
        let other = self.on_other.map(|run| ("Use another email", run));
        let step = Step {
            id: self.id,
            words,
            sent: self.sent,
            busy: self.busy,
            on_send: self.on_send,
            back: None,
            other,
        };
        email_step(step, window, cx)
    }
}

/// A forgotten password: the account's address, then word that a reset link is on its way, with a way to send it again and a way back to sign in.
#[derive(IntoElement)]
pub struct ForgotPassword {
    id: ElementId,
    sent: bool,
    busy: bool,
    on_send: OnText,
    on_back: Option<Run>,
}

impl ForgotPassword {
    pub fn new(
        id: impl Into<ElementId>,
        on_send: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            sent: false,
            busy: false,
            on_send: Rc::new(on_send),
            on_back: None,
        }
    }

    /// Once the owner has sent the link.
    pub fn sent(mut self, sent: bool) -> Self {
        self.sent = sent;
        self
    }

    /// While the owner sends.
    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    /// Shows "Back to sign in".
    pub fn on_back(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_back = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ForgotPassword {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let words = Words {
            intro: "Enter the email you signed up with and we will send a link to set a new password.",
            send: "Send reset link",
            sent: "Check your email",
            note: |address| format!("If an account uses {address}, a reset link is on its way."),
        };
        let back = self.on_back.map(|run| ("Back to sign in", run));
        let step = Step {
            id: self.id,
            words,
            sent: self.sent,
            busy: self.busy,
            on_send: self.on_send,
            back,
            other: None,
        };
        email_step(step, window, cx)
    }
}
