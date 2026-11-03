use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div, prelude::*,
};

use super::login::{Run, field};
use crate::{
    buttons::{Button, ButtonVariant, ConfirmButton, ConfirmMode, CopyButton},
    feedback::Callout,
    forms::{Enter, Input},
    primitives::Severity,
    theme::{ActiveTheme, ControlSize, TextSize},
    typography::{Ellipsis, literal},
};

type OnKey = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// An API key as a list shows it: its key, its name, its ends only, and when it was made and last used.
#[derive(Clone, Debug, PartialEq)]
pub struct ApiKey {
    pub key: SharedString,
    pub name: SharedString,
    pub hint: SharedString,
    pub created: SharedString,
    pub used: SharedString,
}

/// API keys by name, each with its ends, when it was made and last used, and a Revoke that asks twice. A name makes a new key; its secret, handed back by the owner, shows once with a way to copy it.
#[derive(IntoElement)]
pub struct ApiKeyManager {
    id: ElementId,
    keys: Vec<ApiKey>,
    secret: Option<SharedString>,
    on_create: OnKey,
    on_revoke: OnKey,
    on_dismiss: Run,
}

impl ApiKeyManager {
    pub fn new(
        id: impl Into<ElementId>,
        keys: impl IntoIterator<Item = ApiKey>,
        on_create: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
        on_revoke: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
        on_dismiss: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            keys: keys.into_iter().collect(),
            secret: None,
            on_create: Rc::new(on_create),
            on_revoke: Rc::new(on_revoke),
            on_dismiss: Rc::new(on_dismiss),
        }
    }

    /// A new key's secret, shown until the owner clears it.
    pub fn secret(mut self, secret: impl Into<SharedString>) -> Self {
        self.secret = Some(secret.into());
        self
    }
}

impl RenderOnce for ApiKeyManager {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let on_create = self.on_create;
        let on_revoke = self.on_revoke;
        let name = field(&id, "name", "A name, such as Deploys", false, window, cx);
        let named: SharedString = name.read(cx).text().trim().to_string().into();
        let ready = !named.is_empty();
        let clear = name.clone();
        let create: Run = Rc::new(move |window, cx| {
            log::info!("api key manager: create {named}");
            on_create(&named, window, cx);
            clear.update(cx, |field, cx| field.set_text("", cx));
        });
        let enter = create.clone();
        let theme = cx.theme();
        let mono = theme.mono_family.clone();
        let secret = self.secret.map(|secret| {
            let dismiss = self.on_dismiss.clone();
            Callout::new(Severity::Warning)
                .title("Copy this key now. It will not show again.")
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div().flex().child(
                                literal(div())
                                    .flex_1()
                                    .min_w_0()
                                    .font_family(mono.clone())
                                    .child(secret.clone()),
                            ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .gap_2()
                                .child(CopyButton::new((id.clone(), "copy"), secret))
                                .child(
                                    Button::new((id.clone(), "done"), "Done")
                                        .size(ControlSize::Sm)
                                        .on_click(move |_, window, cx| dismiss(window, cx)),
                                ),
                        ),
                )
        });
        let rows = self.keys.iter().enumerate().map(|(ix, each)| {
            let (key, revoke) = (each.key.clone(), on_revoke.clone());
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_3()
                .py_2p5()
                .when(ix > 0, |row| {
                    row.border_t_1().border_color(theme.colors.border)
                })
                .child(
                    div()
                        .flex_1()
                        .min_w(theme.label_width())
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(Ellipsis::new(each.name.clone())),
                                )
                                .child(
                                    literal(div())
                                        .flex_none()
                                        .font_family(mono.clone())
                                        .text_color(theme.colors.fg_muted)
                                        .child(each.hint.clone()),
                                ),
                        )
                        .child(
                            div()
                                .text_size(theme.text_size(TextSize::Xs))
                                .text_color(theme.colors.fg_muted)
                                .child(Ellipsis::new(format!("{} · {}", each.created, each.used))),
                        ),
                )
                .child(
                    div().flex_none().child(
                        ConfirmButton::new(
                            (id.clone(), format!("revoke-{}", each.key)),
                            "Revoke",
                            ConfirmMode::Twice,
                        )
                        .size(ControlSize::Sm)
                        .on_confirm(move |window, cx| {
                            log::info!("api key manager: revoke {key}");
                            revoke(&key, window, cx);
                        }),
                    ),
                )
        });
        let empty = self.keys.is_empty().then(|| {
            div()
                .py_2()
                .text_color(theme.colors.fg_muted)
                .child("No keys yet. Name one to make it.")
        });
        div()
            .flex()
            .flex_col()
            .gap_3()
            .text_size(theme.text_size(TextSize::Sm))
            .children(secret)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .capture_action(move |_: &Enter, window, cx| {
                        if ready {
                            cx.stop_propagation();
                            enter(window, cx);
                        }
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w(theme.label_width())
                            .child(Input::new(&name)),
                    )
                    .child(
                        Button::new((id.clone(), "create"), "Create key")
                            .variant(ButtonVariant::Primary)
                            .disabled(!ready)
                            .on_click(move |_, window, cx| create(window, cx)),
                    ),
            )
            .child(div().flex().flex_col().children(rows).children(empty))
    }
}
