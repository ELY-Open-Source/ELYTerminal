use gpui::{
    AnyElement, App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div, prelude::*, relative,
};
use jiff::Timestamp;
use smallvec::SmallVec;

use crate::{
    data_display::Avatar,
    documents::source,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, AvatarSize, ContainerSize, IconSize, Radius, TextSize},
    typography::RelativeTime,
};

/// Who a message is from, or what it says about the conversation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    System,
    Error,
}

/// Who wrote a message at a glance: a person's picture or initials, the assistant's mark, or a warning for an error.
#[derive(IntoElement)]
pub struct MessageAvatar {
    id: ElementId,
    role: Role,
    name: SharedString,
    picture: Option<SharedString>,
}

impl MessageAvatar {
    pub fn new(id: impl Into<ElementId>, role: Role, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            role,
            name: name.into(),
            picture: None,
        }
    }

    /// A person's picture, a file or a web address.
    pub fn picture(mut self, picture: impl Into<SharedString>) -> Self {
        self.picture = Some(picture.into());
        self
    }
}

impl RenderOnce for MessageAvatar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        match self.role {
            Role::User | Role::System => {
                let avatar = Avatar::new(self.id, self.name).size(AvatarSize::Sm);
                match self.picture {
                    Some(picture) => avatar.image(source(&picture)),
                    None => avatar,
                }
                .into_any_element()
            }
            Role::Assistant | Role::Error => {
                let (icon, tint) = if self.role == Role::Error {
                    (IconName::TriangleAlert, colors.danger)
                } else {
                    (IconName::Sparkles, colors.accent)
                };
                div()
                    .flex_none()
                    .size(theme.avatar_size(AvatarSize::Sm))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(theme.radius(Radius::Md))
                    .bg(tint.opacity(0.12))
                    .child(Icon::new(icon).size(IconSize::Sm).color(tint))
                    .into_any_element()
            }
        }
    }
}

/// A message's name line: who, when, and what model answered.
#[derive(IntoElement)]
pub struct MessageHeader {
    id: ElementId,
    name: SharedString,
    at: Option<Timestamp>,
    model: Option<SharedString>,
}

impl MessageHeader {
    pub fn new(id: impl Into<ElementId>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            at: None,
            model: None,
        }
    }

    pub fn at(mut self, at: Timestamp) -> Self {
        self.at = Some(at);
        self
    }

    pub fn model(mut self, model: impl Into<SharedString>) -> Self {
        self.model = Some(model.into());
        self
    }
}

impl RenderOnce for MessageHeader {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        div()
            .flex()
            .items_baseline()
            .gap_2()
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.fg)
                    .child(self.name),
            )
            .children(self.model.map(|model| {
                div()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.fg_muted)
                    .child(model)
            }))
            .children(self.at.map(|at| {
                div()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.fg_subtle)
                    .child(RelativeTime::new((self.id.clone(), "when"), at))
            }))
    }
}

/// A message's last line: quiet facts, such as tokens and time, then its actions.
#[derive(IntoElement, Default)]
pub struct MessageFooter {
    facts: Vec<SharedString>,
    actions: SmallVec<[AnyElement; 2]>,
}

impl MessageFooter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn fact(mut self, fact: impl Into<SharedString>) -> Self {
        self.facts.push(fact.into());
        self
    }
}

impl ParentElement for MessageFooter {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.actions.extend(elements);
    }
}

impl RenderOnce for MessageFooter {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .flex()
            .items_center()
            .gap_2()
            .text_size(theme.text_size(TextSize::Xs))
            .text_color(theme.colors.fg_subtle)
            .children(self.actions)
            .when(!self.facts.is_empty(), |row| {
                row.child(div().child(self.facts.join(" · ")))
            })
    }
}

/// One message: yours in a quiet bubble on the right, the assistant's as prose beside its mark, a system note centered and small, an error beside a warning, its body an `ErrorMessage`. A header, a footer and an avatar are optional.
#[derive(IntoElement)]
pub struct MessageBubble {
    role: Role,
    avatar: Option<MessageAvatar>,
    header: Option<MessageHeader>,
    footer: Option<MessageFooter>,
    body: SmallVec<[AnyElement; 2]>,
}

impl MessageBubble {
    pub fn new(role: Role) -> Self {
        Self {
            role,
            avatar: None,
            header: None,
            footer: None,
            body: SmallVec::new(),
        }
    }

    pub fn avatar(mut self, avatar: MessageAvatar) -> Self {
        self.avatar = Some(avatar);
        self
    }

    pub fn header(mut self, header: MessageHeader) -> Self {
        self.header = Some(header);
        self
    }

    pub fn footer(mut self, footer: MessageFooter) -> Self {
        self.footer = Some(footer);
        self
    }
}

impl ParentElement for MessageBubble {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl RenderOnce for MessageBubble {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let body = div()
            .flex()
            .flex_col()
            .gap_2()
            .text_size(theme.text_size(TextSize::Base))
            .line_height(relative(1.6))
            .children(self.body);
        match self.role {
            Role::System => div()
                .flex()
                .justify_center()
                .py_1()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(colors.fg_muted)
                .child(body.text_size(theme.text_size(TextSize::Xs)))
                .into_any_element(),
            Role::User => div()
                .flex()
                .gap_3()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .children(
                            self.header
                                .map(|header| div().flex().justify_end().child(header)),
                        )
                        .child(
                            div().flex().justify_end().child(
                                div()
                                    .min_w_0()
                                    .max_w(theme.container_width(ContainerSize::Sm))
                                    .px_4()
                                    .py_2p5()
                                    .rounded(theme.radius(Radius::Xl))
                                    .bg(colors.hover)
                                    .text_color(colors.fg)
                                    .child(body),
                            ),
                        )
                        .children(
                            self.footer
                                .map(|footer| div().flex().justify_end().child(footer)),
                        ),
                )
                .children(self.avatar)
                .into_any_element(),
            Role::Assistant | Role::Error => div()
                .flex()
                .gap_3()
                .children(self.avatar)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .children(self.header)
                        .child(body.text_color(colors.fg))
                        .children(self.footer),
                )
                .into_any_element(),
        }
    }
}

/// A conversation's frame: its messages in a column sized for reading, the composer held at the bottom.
#[derive(IntoElement)]
pub struct ChatContainer {
    messages: AnyElement,
    composer: Option<AnyElement>,
}

impl ChatContainer {
    pub fn new(messages: impl IntoElement) -> Self {
        Self {
            messages: messages.into_any_element(),
            composer: None,
        }
    }

    pub fn composer(mut self, composer: impl IntoElement) -> Self {
        self.composer = Some(composer.into_any_element());
        self
    }
}

impl RenderOnce for ChatContainer {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let column = theme.container_width(ContainerSize::Md);
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.bg)
            .child(div().flex_1().min_h_0().child(self.messages))
            .children(self.composer.map(|composer| {
                div()
                    .flex_none()
                    .w_full()
                    .flex()
                    .justify_center()
                    .px_4()
                    .pb_4()
                    .child(div().w_full().max_w(column).child(composer))
            }))
    }
}
