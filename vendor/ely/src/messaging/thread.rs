use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div, prelude::*,
};
use jiff::Timestamp;
use smallvec::SmallVec;

use crate::{
    buttons::{ButtonVariant, IconButton},
    data_display::{Avatar, AvatarGroup},
    forms::Run,
    layout::on_axis,
    primitives::{Divider, FocusRing, IconName},
    theme::{ActiveTheme, AvatarSize, ControlSize, Radius, TextSize},
    typography::{Ellipsis, format},
};

/// A thread's mark under its first message: who replied, how many replies, and when the last came. With a handler, a press opens the thread.
#[derive(IntoElement)]
pub struct MessageThread {
    id: ElementId,
    replies: usize,
    last: Timestamp,
    repliers: Vec<Avatar>,
    on_open: Option<Run>,
}

impl MessageThread {
    pub fn new(id: impl Into<ElementId>, replies: usize, last: Timestamp) -> Self {
        assert!(replies > 0, "a thread holds a reply");
        Self {
            id: id.into(),
            replies,
            last,
            repliers: Vec::new(),
            on_open: None,
        }
    }

    /// Who replied; the first three show.
    pub fn repliers(mut self, repliers: impl IntoIterator<Item = Avatar>) -> Self {
        self.repliers = repliers.into_iter().collect();
        assert!(
            self.repliers.len() <= self.replies,
            "more repliers than replies"
        );
        self
    }

    pub fn on_open(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for MessageThread {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let count = format::plural(self.replies as u64, "reply", "replies");
        let when = format!(
            "Last reply {}",
            format::relative(self.last, Timestamp::now())
        );
        let id = self.id.clone();
        div()
            .id(self.id)
            .debug_selector(|| "message-thread".into())
            .flex()
            .items_center()
            .gap_2()
            .px_1p5()
            .py_1()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(gpui::transparent_black())
            .text_size(theme.text_size(TextSize::Sm))
            .when_some(self.on_open, |row, open| {
                row.cursor_pointer()
                    .tab_index(0)
                    .focus_ring(cx)
                    .hover(|style| style.bg(colors.surface))
                    .on_click(move |_, window, cx| {
                        log::info!("message thread {id}: open");
                        open(window, cx)
                    })
            })
            .when(!self.repliers.is_empty(), |row| {
                row.child(
                    div()
                        .flex_none()
                        .child(AvatarGroup::new(self.repliers).max(3).size(AvatarSize::Xs)),
                )
            })
            .child(
                div()
                    .flex_none()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.link)
                    .child(count),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_color(colors.fg_subtle)
                    .child(Ellipsis::new(when)),
            )
    }
}

/// A thread beside its chat: a head with the chat's name and a close, the first message, a rule counting the replies, the replies, and a composer held at the bottom. The body scrolls; the host gives the panel its size.
#[derive(IntoElement)]
pub struct ThreadPanel {
    id: ElementId,
    root: AnyElement,
    chat: Option<SharedString>,
    replies: SmallVec<[AnyElement; 4]>,
    composer: Option<AnyElement>,
    on_close: Option<Run>,
}

impl ThreadPanel {
    pub fn new(id: impl Into<ElementId>, root: impl IntoElement) -> Self {
        Self {
            id: id.into(),
            root: root.into_any_element(),
            chat: None,
            replies: SmallVec::new(),
            composer: None,
            on_close: None,
        }
    }

    /// The chat the thread is in, after the title.
    pub fn chat(mut self, name: impl Into<SharedString>) -> Self {
        self.chat = Some(name.into());
        self
    }

    pub fn composer(mut self, composer: impl IntoElement) -> Self {
        self.composer = Some(composer.into_any_element());
        self
    }

    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for ThreadPanel {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.replies.extend(elements);
    }
}

impl RenderOnce for ThreadPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let count = self.replies.len();
        let close = self.on_close.map(|close| {
            let id = self.id.clone();
            IconButton::new((self.id.clone(), "close"), IconName::X)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip("Close thread")
                .on_click(move |_, window, cx| {
                    log::info!("thread panel {id}: close");
                    close(window, cx)
                })
        });
        let head = div()
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .px_4()
            .py_2()
            .border_b_1()
            .border_color(colors.border)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_baseline()
                    .gap_2()
                    .child(
                        div()
                            .flex_none()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.fg)
                            .child("Thread"),
                    )
                    .children(self.chat.map(|chat| {
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(colors.fg_muted)
                            .child(Ellipsis::new(chat))
                    })),
            )
            .children(close.map(|close| div().flex_none().child(close)));
        let rule = (count > 0).then(|| {
            div()
                .debug_selector(|| "thread-replies".into())
                .px_3()
                .py_1()
                .child(Divider::horizontal().label(format::plural(
                    count as u64,
                    "reply",
                    "replies",
                )))
        });
        let body = on_axis(
            div()
                .id((self.id.clone(), "body"))
                .flex_1()
                .min_h_0()
                .overflow_y_scroll(),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .py_2()
                .child(self.root)
                .children(rule)
                .children(self.replies),
        );
        div()
            .debug_selector(|| "thread-panel".into())
            .size_full()
            .flex()
            .flex_col()
            .child(head)
            .child(body)
            .children(self.composer.map(|composer| {
                div()
                    .flex_none()
                    .p_3()
                    .border_t_1()
                    .border_color(colors.border)
                    .child(composer)
            }))
    }
}
