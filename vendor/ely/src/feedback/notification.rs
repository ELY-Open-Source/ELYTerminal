use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, Styled, Window, div, prelude::*,
};
use smallvec::SmallVec;

use super::EmptyState;
use crate::{
    buttons::{Button, ButtonVariant},
    forms::Run,
    layout::ScrollArea,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, TextSize},
};

/// A titled message with a line, a time and actions: a row of a `NotificationCenter`, or on its own.
#[derive(IntoElement)]
pub struct Notification {
    id: ElementId,
    icon: IconName,
    title: SharedString,
    body: Option<SharedString>,
    time: Option<SharedString>,
    unread: bool,
    actions: SmallVec<[AnyElement; 2]>,
}

impl Notification {
    pub fn new(id: impl Into<ElementId>, icon: IconName, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            icon,
            title: title.into(),
            body: None,
            time: None,
            unread: false,
            actions: SmallVec::new(),
        }
    }

    pub fn body(mut self, text: impl Into<SharedString>) -> Self {
        self.body = Some(text.into());
        self
    }

    /// When it happened, as the owner words it.
    pub fn time(mut self, time: impl Into<SharedString>) -> Self {
        self.time = Some(time.into());
        self
    }

    pub fn unread(mut self, unread: bool) -> Self {
        self.unread = unread;
        self
    }

    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }
}

impl RenderOnce for Notification {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let disc = theme.control_height(ControlSize::Lg);
        div()
            .id(self.id)
            .relative()
            .flex()
            .items_start()
            .gap_3()
            .px_4()
            .py_3()
            .text_size(theme.text_size(TextSize::Base))
            .hover(|row| row.bg(colors.hover))
            .when(self.unread, |row| {
                row.child(
                    div()
                        .absolute()
                        .left_1p5()
                        .top_0()
                        .h(disc)
                        .mt_3()
                        .flex()
                        .items_center()
                        .child(
                            div()
                                .size(theme.status_dot())
                                .rounded_full()
                                .bg(colors.accent),
                        ),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .size(disc)
                    .rounded_full()
                    .bg(colors.sunken)
                    .child(
                        Icon::new(self.icon)
                            .size(IconSize::Md)
                            .color(colors.fg_muted),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap_2()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(colors.fg)
                                    .child(self.title),
                            )
                            .when_some(self.time, |head, time| {
                                head.child(
                                    div()
                                        .flex_none()
                                        .text_size(theme.text_size(TextSize::Xs))
                                        .text_color(colors.fg_subtle)
                                        .child(time),
                                )
                            }),
                    )
                    .when_some(self.body, |text, body| {
                        text.child(div().text_color(colors.fg_muted).line_clamp(2).child(body))
                    })
                    .when(!self.actions.is_empty(), |text| {
                        text.child(div().flex().gap_2().pt_1p5().children(self.actions))
                    }),
            )
    }
}

/// Notifications under day headings, with Mark all read and Clear. The owner keeps the list and what is read.
#[derive(IntoElement)]
pub struct NotificationCenter {
    id: ElementId,
    groups: Vec<(SharedString, Vec<Notification>)>,
    on_read_all: Option<Run>,
    on_clear: Option<Run>,
}

impl NotificationCenter {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            groups: Vec::new(),
            on_read_all: None,
            on_clear: None,
        }
    }

    /// A heading, then its notifications, newest first. Empty groups are left out.
    pub fn group(
        mut self,
        heading: impl Into<SharedString>,
        items: impl IntoIterator<Item = Notification>,
    ) -> Self {
        let items: Vec<Notification> = items.into_iter().collect();
        if !items.is_empty() {
            self.groups.push((heading.into(), items));
        }
        self
    }

    pub fn on_read_all(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_read_all = Some(Rc::new(handler));
        self
    }

    pub fn on_clear(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_clear = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for NotificationCenter {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let unread = self
            .groups
            .iter()
            .flat_map(|(_, items)| items)
            .filter(|item| item.unread)
            .count();
        let empty = self.groups.is_empty();
        let header = div()
            .flex()
            .flex_none()
            .items_center()
            .gap_2()
            .h(theme.control_height(ControlSize::Lg))
            .pl_4()
            .pr_2()
            .my_1()
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Md))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Notifications"),
            )
            .when(unread > 0, |header| {
                header.child(
                    div()
                        .text_size(theme.text_size(TextSize::Sm))
                        .text_color(colors.fg_subtle)
                        .child(format!("{unread} new")),
                )
            })
            .child(div().flex_1())
            .when_some(self.on_read_all.filter(|_| unread > 0), |header, run| {
                header.child(
                    Button::new((self.id.clone(), "read-all"), "Mark all read")
                        .variant(ButtonVariant::Ghost)
                        .size(ControlSize::Sm)
                        .on_click(move |_, window, cx| run(window, cx)),
                )
            })
            .when_some(self.on_clear.filter(|_| !empty), |header, run| {
                header.child(
                    Button::new((self.id.clone(), "clear"), "Clear")
                        .variant(ButtonVariant::Ghost)
                        .size(ControlSize::Sm)
                        .on_click(move |_, window, cx| run(window, cx)),
                )
            });
        let body = if empty {
            div()
                .flex_1()
                .flex()
                .flex_col()
                .justify_center()
                .child(
                    EmptyState::new(
                        (self.id.clone(), "empty"),
                        IconName::CheckCheck,
                        "All caught up",
                    )
                    .body("New notifications will show here."),
                )
                .into_any_element()
        } else {
            ScrollArea::new((self.id.clone(), "list"))
                .flex_1()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .pb_2()
                        .children(self.groups.into_iter().map(|(heading, items)| {
                            div()
                                .child(
                                    div()
                                        .px_4()
                                        .pt_3()
                                        .pb_1()
                                        .text_size(theme.text_size(TextSize::Xs))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(colors.fg_subtle)
                                        .child(heading),
                                )
                                .children(items)
                        })),
                )
                .into_any_element()
        };
        div()
            .id(self.id)
            .flex()
            .flex_col()
            .size_full()
            .child(header)
            .child(div().flex_none().h_px().bg(colors.border))
            .child(body)
    }
}
