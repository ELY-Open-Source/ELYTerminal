use std::rc::Rc;

use gpui::{
    App, ElementId, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div,
};

use crate::{
    buttons::{Button, ButtonVariant},
    feedback::InlineMessage,
    forms::{Checkbox, EmailInput, Enter, FormField, PasswordInput, TextInput, is_email},
    primitives::Severity,
    theme::{ActiveTheme, TextSize},
    typography::Link,
};

pub(super) type Run = Rc<dyn Fn(&mut Window, &mut App)>;

/// A form's text field, kept under the form's id; `masked` hides what is typed.
pub(super) fn field(
    id: &ElementId,
    key: &'static str,
    hint: &'static str,
    masked: bool,
    window: &mut Window,
    cx: &mut App,
) -> Entity<TextInput> {
    window.use_keyed_state((id.clone(), key), cx, move |window, cx| {
        let field = TextInput::new(window, cx).placeholder(hint);
        if masked { field.masked() } else { field }
    })
}

/// Muted words and a link on one line, as under a form: "No account? Sign up".
pub(super) fn aside(
    id: impl Into<ElementId>,
    words: &'static str,
    link: &'static str,
    run: Run,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .flex_wrap()
        .justify_center()
        .gap_1()
        .text_size(theme.text_size(TextSize::Sm))
        .text_color(theme.colors.fg_muted)
        .child(words)
        .child(Link::new(id, link, move |_, window, cx| run(window, cx)))
}

/// What a sign-in hands its owner.
#[derive(Clone, Debug, PartialEq)]
pub struct Login {
    pub email: SharedString,
    pub password: SharedString,
    pub remember: bool,
}

type OnLogin = Rc<dyn Fn(&Login, &mut Window, &mut App)>;

/// An email and a password, a box to stay signed in, and ways to a forgotten password or a new account. Sign in rests until the address reads and a password is typed; Enter in a field signs in.
#[derive(IntoElement)]
pub struct LoginForm {
    id: ElementId,
    error: Option<SharedString>,
    busy: bool,
    on_submit: OnLogin,
    on_forgot: Option<Run>,
    on_sign_up: Option<Run>,
}

impl LoginForm {
    pub fn new(
        id: impl Into<ElementId>,
        on_submit: impl Fn(&Login, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            error: None,
            busy: false,
            on_submit: Rc::new(on_submit),
            on_forgot: None,
            on_sign_up: None,
        }
    }

    /// Why the last try failed, shown above Sign in.
    pub fn error(mut self, error: impl Into<SharedString>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// While the owner checks: Sign in spins and rests.
    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    /// Shows "Forgot password?".
    pub fn on_forgot(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_forgot = Some(Rc::new(handler));
        self
    }

    /// Shows "No account? Sign up".
    pub fn on_sign_up(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_sign_up = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for LoginForm {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let email = field(&id, "email", "name@example.com", false, window, cx);
        let password = field(&id, "password", "Password", true, window, cx);
        let remember = window.use_keyed_state((id.clone(), "remember"), cx, |_, _| false);
        let login = Login {
            email: email.read(cx).text().trim().to_string().into(),
            password: password.read(cx).text().to_string().into(),
            remember: *remember.read(cx),
        };
        let remembered = login.remember;
        let ready = is_email(&login.email) && !login.password.is_empty() && !self.busy;
        let on_submit = self.on_submit;
        let submit: Run = Rc::new(move |window, cx| {
            log::info!("login form: sign in");
            on_submit(&login, window, cx);
        });
        let enter = submit.clone();
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
            .child(
                FormField::new((id.clone(), "email-field"), "Email").child(EmailInput::new(&email)),
            )
            .child(
                FormField::new((id.clone(), "password-field"), "Password")
                    .child(PasswordInput::new(&password)),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        Checkbox::new((id.clone(), "remember"), remembered)
                            .label("Keep me signed in")
                            .on_change(move |on, _, cx| {
                                remember.update(cx, |remember, cx| {
                                    *remember = on;
                                    cx.notify();
                                })
                            }),
                    )
                    .children(self.on_forgot.map(|run| {
                        Link::new(
                            (id.clone(), "forgot"),
                            "Forgot password?",
                            move |_, window, cx| run(window, cx),
                        )
                    })),
            )
            .children(
                self.error
                    .map(|error| InlineMessage::new(Severity::Danger, error)),
            )
            .child(
                Button::new((id.clone(), "submit"), "Sign in")
                    .variant(ButtonVariant::Primary)
                    .full_width()
                    .loading(self.busy)
                    .disabled(!ready)
                    .on_click(move |_, window, cx| submit(window, cx)),
            )
            .children(
                self.on_sign_up
                    .map(|run| aside((id.clone(), "sign-up"), "No account?", "Sign up", run, cx)),
            )
    }
}
