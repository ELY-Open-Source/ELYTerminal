use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div,
};

use super::login::{Run, aside, field};
use crate::{
    buttons::{Button, ButtonVariant},
    feedback::InlineMessage,
    forms::{Checkbox, EmailInput, Enter, FormField, Input, PasswordInput, is_email},
    primitives::{Icon, IconName, Severity},
    theme::{ActiveTheme, IconSize, TextSize},
};

/// What a sign-up hands its owner.
#[derive(Clone, Debug, PartialEq)]
pub struct Signup {
    pub name: SharedString,
    pub email: SharedString,
    pub password: SharedString,
}

/// The rules a new password keeps, each with whether `password` keeps it.
pub fn password_rules(password: &str) -> [(&'static str, bool); 3] {
    [
        ("At least 8 characters", password.chars().count() >= 8),
        ("A letter", password.chars().any(char::is_alphabetic)),
        ("A number", password.chars().any(|ch| ch.is_ascii_digit())),
    ]
}

type OnSignup = Rc<dyn Fn(&Signup, &mut Window, &mut App)>;

/// A name, an email and a new password with the rules it keeps, ticked as they hold, and terms to accept when given. Create account rests until all of it holds; Enter in a field sends it.
#[derive(IntoElement)]
pub struct SignupForm {
    id: ElementId,
    terms: Option<SharedString>,
    error: Option<SharedString>,
    busy: bool,
    on_submit: OnSignup,
    on_sign_in: Option<Run>,
}

impl SignupForm {
    pub fn new(
        id: impl Into<ElementId>,
        on_submit: impl Fn(&Signup, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            terms: None,
            error: None,
            busy: false,
            on_submit: Rc::new(on_submit),
            on_sign_in: None,
        }
    }

    /// A box the person ticks to accept, such as "I agree to the terms of service".
    pub fn terms(mut self, label: impl Into<SharedString>) -> Self {
        self.terms = Some(label.into());
        self
    }

    /// Why the last try failed, shown above Create account.
    pub fn error(mut self, error: impl Into<SharedString>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// While the owner works: Create account spins and rests.
    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    /// Shows "Have an account? Sign in".
    pub fn on_sign_in(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_sign_in = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SignupForm {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let name = field(&id, "name", "Your name", false, window, cx);
        let email = field(&id, "email", "name@example.com", false, window, cx);
        let password = field(&id, "password", "A new password", true, window, cx);
        let accepted = window.use_keyed_state((id.clone(), "terms"), cx, |_, _| false);
        let signup = Signup {
            name: name.read(cx).text().trim().to_string().into(),
            email: email.read(cx).text().trim().to_string().into(),
            password: password.read(cx).text().to_string().into(),
        };
        let rules = password_rules(&signup.password);
        let agreed = *accepted.read(cx);
        let ready = !signup.name.is_empty()
            && is_email(&signup.email)
            && rules.iter().all(|(_, kept)| *kept)
            && (self.terms.is_none() || agreed)
            && !self.busy;
        let on_submit = self.on_submit;
        let submit: Run = Rc::new(move |window, cx| {
            log::info!("sign-up form: create account");
            on_submit(&signup, window, cx);
        });
        let enter = submit.clone();
        let theme = cx.theme();
        let rule = |(rule, kept): (&'static str, bool)| {
            let color = if kept {
                theme.colors.success
            } else {
                theme.colors.fg_muted
            };
            div()
                .flex()
                .items_center()
                .gap_2()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(color)
                .child(
                    Icon::new(if kept {
                        IconName::CircleCheck
                    } else {
                        IconName::Circle
                    })
                    .size(IconSize::Sm)
                    .color(color),
                )
                .child(rule)
        };
        let rules = div().flex().flex_col().gap_1().children(rules.map(rule));
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
            .child(FormField::new((id.clone(), "name-field"), "Name").child(Input::new(&name)))
            .child(
                FormField::new((id.clone(), "email-field"), "Email").child(EmailInput::new(&email)),
            )
            .child(
                FormField::new((id.clone(), "password-field"), "Password")
                    .child(PasswordInput::new(&password))
                    .child(rules),
            )
            .children(self.terms.map(|terms| {
                Checkbox::new((id.clone(), "terms"), agreed)
                    .label(terms)
                    .on_change(move |on, _, cx| {
                        accepted.update(cx, |accepted, cx| {
                            *accepted = on;
                            cx.notify();
                        })
                    })
            }))
            .children(
                self.error
                    .map(|error| InlineMessage::new(Severity::Danger, error)),
            )
            .child(
                Button::new((id.clone(), "submit"), "Create account")
                    .variant(ButtonVariant::Primary)
                    .full_width()
                    .loading(self.busy)
                    .disabled(!ready)
                    .on_click(move |_, window, cx| submit(window, cx)),
            )
            .children(self.on_sign_in.map(|run| {
                aside(
                    (id.clone(), "sign-in"),
                    "Have an account?",
                    "Sign in",
                    run,
                    cx,
                )
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::password_rules;

    #[test]
    fn a_password_keeps_each_rule_on_its_own() {
        let kept = |password: &str| password_rules(password).map(|(_, kept)| kept);
        assert_eq!(kept(""), [false, false, false]);
        assert_eq!(kept("abcdefgh"), [true, true, false]);
        assert_eq!(kept("12345678"), [true, false, true]);
        assert_eq!(
            kept("été2026"),
            [false, true, true],
            "seven characters, counted as characters"
        );
        assert_eq!(kept("été20261"), [true, true, true]);
    }
}
