use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, ImageSource, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div, prelude::*,
};
use jiff::{Timestamp, tz::TimeZone};

use crate::{
    buttons::{Button, ButtonVariant, IconButton},
    data_display::{Avatar, Presence},
    forms::Run,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, AvatarSize, ControlSize, IconSize, TextSize},
    typography::{Ellipsis, format, fresh},
};

/// Where someone stands: a dot in the presence's color and its word.
#[derive(IntoElement)]
pub struct OnlineStatus {
    presence: Presence,
}

impl OnlineStatus {
    pub fn new(presence: Presence) -> Self {
        Self { presence }
    }
}

impl RenderOnce for OnlineStatus {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        div()
            .flex()
            .items_center()
            .gap_1p5()
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(colors.fg_muted)
            .child(
                div()
                    .flex_none()
                    .size(theme.status_dot())
                    .rounded_full()
                    .bg(self.presence.color(colors)),
            )
            .child(self.presence.label())
    }
}

/// A person at a glance: their picture with presence, name and standing, their title and status, the time where they are, and ways to reach them, each only with its handler. The host gives the width.
#[derive(IntoElement)]
pub struct UserProfileCard {
    id: ElementId,
    name: SharedString,
    presence: Presence,
    picture: Option<ImageSource>,
    title: Option<SharedString>,
    status: Option<SharedString>,
    zone: Option<TimeZone>,
    on_message: Option<Run>,
    on_call: Option<Run>,
}

impl UserProfileCard {
    pub fn new(
        id: impl Into<ElementId>,
        name: impl Into<SharedString>,
        presence: Presence,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            presence,
            picture: None,
            title: None,
            status: None,
            zone: None,
            on_message: None,
            on_call: None,
        }
    }

    pub fn picture(mut self, source: impl Into<ImageSource>) -> Self {
        self.picture = Some(source.into());
        self
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Their own words, an emoji first when they chose one.
    pub fn status(mut self, status: impl Into<SharedString>) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Their time zone, for the time where they are.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }

    pub fn on_message(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_message = Some(Rc::new(handler));
        self
    }

    pub fn on_call(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_call = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for UserProfileCard {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if self.zone.is_some() {
            fresh((self.id.clone(), "clock"), window, cx);
        }
        let theme = cx.theme();
        let colors = &theme.colors;
        let avatar = Avatar::new((self.id.clone(), "avatar"), self.name.clone())
            .size(AvatarSize::Xl)
            .presence(self.presence);
        let avatar = match self.picture {
            Some(picture) => avatar.image(picture),
            None => avatar,
        };
        let line = |text: SharedString| div().min_w_0().child(Ellipsis::new(text));
        let local = self.zone.map(|zone| {
            let time = format::datetime(Timestamp::now(), &zone, "%H:%M")
                .expect("a fixed pattern formats");
            div()
                .flex()
                .items_center()
                .gap_1p5()
                .text_color(colors.fg_muted)
                .child(
                    Icon::new(IconName::Clock)
                        .size(IconSize::Sm)
                        .color(colors.fg_subtle),
                )
                .child(line(format!("{time} local time").into()))
        });
        let name = self.name.clone();
        let message = self.on_message.map(|run| {
            let name = name.clone();
            Button::new((self.id.clone(), "message"), "Message")
                .icon(IconName::MessageSquare)
                .variant(ButtonVariant::Primary)
                .size(ControlSize::Sm)
                .on_click(move |_, window, cx| {
                    log::info!("profile card {name}: message");
                    run(window, cx)
                })
        });
        let call = self.on_call.map(|run| {
            IconButton::new((self.id.clone(), "call"), IconName::Phone)
                .variant(ButtonVariant::Outline)
                .size(ControlSize::Sm)
                .tooltip("Call")
                .on_click(move |_, window, cx| {
                    log::info!("profile card {name}: call");
                    run(window, cx)
                })
        });
        let reach = (message.is_some() || call.is_some()).then(|| {
            div()
                .flex()
                .items_center()
                .gap_2()
                .children(message.map(|message| div().flex_1().min_w_0().child(message)))
                .children(call)
        });
        div()
            .debug_selector(|| "profile-card".into())
            .flex()
            .flex_col()
            .gap_3()
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(div().flex_none().child(avatar))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                line(self.name)
                                    .text_size(theme.text_size(TextSize::Lg))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.fg),
                            )
                            .child(OnlineStatus::new(self.presence)),
                    ),
            )
            .when(
                self.title.is_some() || self.status.is_some() || local.is_some(),
                |card| {
                    card.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .children(
                                self.title
                                    .map(|title| line(title).text_color(colors.fg_muted)),
                            )
                            .children(self.status.map(|status| line(status).text_color(colors.fg)))
                            .children(local),
                    )
                },
            )
            .children(reach)
    }
}
