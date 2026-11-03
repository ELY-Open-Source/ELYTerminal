use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
    prelude::*,
};

use super::login::Run;
use crate::{
    buttons::{Button, ButtonVariant, ConfirmButton, ConfirmMode},
    data_display::{Badge, Tone},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, TextSize},
    typography::Ellipsis,
};

type OnKey = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// A place an account is signed in: its key, an icon for the device, the device and the app on it, where it is, when it was last seen, and whether it is this one.
#[derive(Clone, Debug, PartialEq)]
pub struct Session {
    pub key: SharedString,
    pub icon: IconName,
    pub device: SharedString,
    pub client: SharedString,
    pub place: SharedString,
    pub seen: SharedString,
    pub current: bool,
}

/// Where an account is signed in, this device first. Each other one signs out on its own, or all of them at once after a second press.
#[derive(IntoElement)]
pub struct SessionList {
    id: ElementId,
    sessions: Vec<Session>,
    on_sign_out: OnKey,
    on_sign_out_others: Option<Run>,
}

impl SessionList {
    pub fn new(
        id: impl Into<ElementId>,
        sessions: impl IntoIterator<Item = Session>,
        on_sign_out: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        let mut sessions: Vec<_> = sessions.into_iter().collect();
        assert!(
            sessions.iter().filter(|session| session.current).count() <= 1,
            "session list: two sessions are this device"
        );
        sessions.sort_by_key(|session| !session.current);
        Self {
            id: id.into(),
            sessions,
            on_sign_out: Rc::new(on_sign_out),
            on_sign_out_others: None,
        }
    }

    /// Shows "Sign out of all other devices" while there are others.
    pub fn on_sign_out_others(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_sign_out_others = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SessionList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let on_sign_out = self.on_sign_out;
        let others = self.sessions.iter().any(|session| !session.current);
        let theme = cx.theme();
        let rows = self.sessions.into_iter().enumerate().map(|(ix, session)| {
            let end = match session.current {
                true => Badge::new("This device")
                    .tone(Tone::Neutral)
                    .into_any_element(),
                false => {
                    let (key, out) = (session.key.clone(), on_sign_out.clone());
                    Button::new((id.clone(), format!("out-{}", session.key)), "Sign out")
                        .variant(ButtonVariant::Ghost)
                        .size(ControlSize::Sm)
                        .on_click(move |_, window, cx| {
                            log::info!("session list: sign out {key}");
                            out(&key, window, cx);
                        })
                        .into_any_element()
                }
            };
            let tag = format!("session-{}", session.key);
            div()
                .debug_selector(move || tag)
                .flex()
                .items_center()
                .gap_3()
                .py_2p5()
                .when(ix > 0, |row| {
                    row.border_t_1().border_color(theme.colors.border)
                })
                .child(
                    Icon::new(session.icon)
                        .size(IconSize::Md)
                        .color(theme.colors.fg_muted),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(Ellipsis::new(format!(
                            "{} · {}",
                            session.device, session.client
                        )))
                        .child(
                            div()
                                .text_size(theme.text_size(TextSize::Xs))
                                .text_color(theme.colors.fg_muted)
                                .child(Ellipsis::new(format!(
                                    "{} · {}",
                                    session.place, session.seen
                                ))),
                        ),
                )
                .child(div().flex_none().child(end))
        });
        let everyone = self.on_sign_out_others.filter(|_| others).map(|run| {
            div().pt_2().flex().child(
                ConfirmButton::new(
                    (id.clone(), "others"),
                    "Sign out of all other devices",
                    ConfirmMode::Twice,
                )
                .on_confirm(move |window, cx| {
                    log::info!("session list: sign out of the others");
                    run(window, cx);
                }),
            )
        });
        div()
            .flex()
            .flex_col()
            .text_size(theme.text_size(TextSize::Sm))
            .children(rows)
            .children(everyone)
    }
}
