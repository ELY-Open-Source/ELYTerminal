use std::rc::Rc;

use gpui::{
    App, ElementId, Entity, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div, prelude::*,
};
use jiff::Timestamp;

use super::{
    peers::Peer,
    reactions::{Reaction, Reactions},
};
use crate::{
    buttons::{Button, ButtonVariant},
    forms::{Enter, Input, OnFlag, Run, TextInput},
    i18n,
    overlays::Popover,
    primitives::IconName,
    theme::{ActiveTheme, AvatarSize, ControlSize, Radius, TextSize},
    typography::RelativeTime,
};

/// A comment: who wrote it, when, what, and the reactions it has.
#[derive(Clone, Debug, PartialEq)]
pub struct Comment {
    pub author: Peer,
    pub at: Timestamp,
    pub body: SharedString,
    pub reactions: Vec<Reaction>,
}

/// A conversation on a document: its key, the words it is about, its comments in order, and whether it is resolved.
#[derive(Clone, Debug, PartialEq)]
pub struct Thread {
    pub key: SharedString,
    pub quote: Option<SharedString>,
    pub comments: Vec<Comment>,
    pub resolved: bool,
}

type OnReact = Rc<dyn Fn(usize, &SharedString, &mut Window, &mut App)>;

/// A field to answer in and a button to send; Enter sends too, never an empty answer.
#[derive(IntoElement)]
pub struct Reply {
    id: ElementId,
    field: Entity<TextInput>,
    on_send: Run,
}

impl Reply {
    pub fn new(
        id: impl Into<ElementId>,
        field: &Entity<TextInput>,
        on_send: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            field: field.clone(),
            on_send: Rc::new(on_send),
        }
    }
}

impl RenderOnce for Reply {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let empty = self.field.read(cx).text().trim().is_empty();
        let (send, enter) = (self.on_send.clone(), self.on_send);
        div()
            .flex()
            .items_center()
            .gap_2()
            .capture_action(move |_: &Enter, window, cx| {
                cx.stop_propagation();
                if !empty {
                    log::info!("reply: sent");
                    enter(window, cx);
                }
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(Input::new(&self.field).size(ControlSize::Sm)),
            )
            .child(
                Button::new(
                    (self.id.clone(), "send"),
                    i18n::text(cx, "comments.reply", &[]),
                )
                .variant(ButtonVariant::Primary)
                .size(ControlSize::Sm)
                .disabled(empty)
                .on_click(move |_, window, cx| {
                    log::info!("reply: sent");
                    send(window, cx)
                }),
            )
    }
}

/// A conversation on a document: the words it is about, each comment with its author, time and reactions, a reply below, and resolve or reopen above; a resolved thread quiets down and takes no replies.
#[derive(IntoElement)]
pub struct CommentThread {
    id: ElementId,
    thread: Thread,
    active: bool,
    reply: Option<(Entity<TextInput>, Run)>,
    on_resolve: Option<OnFlag>,
    on_react: Option<OnReact>,
}

impl CommentThread {
    pub fn new(id: impl Into<ElementId>, thread: Thread) -> Self {
        assert!(
            !thread.comments.is_empty(),
            "thread {} has no comment",
            thread.key
        );
        Self {
            id: id.into(),
            thread,
            active: false,
            reply: None,
            on_resolve: None,
            on_react: None,
        }
    }

    /// Marks the thread the reader is on.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Answers in `field`; `on_send` takes its text.
    pub fn reply(
        mut self,
        field: &Entity<TextInput>,
        on_send: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.reply = Some((field.clone(), Rc::new(on_send)));
        self
    }

    /// Gets true to resolve, false to reopen.
    pub fn on_resolve(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_resolve = Some(Rc::new(handler));
        self
    }

    /// Gets a comment's index and the emoji to toggle on it.
    pub fn on_react(
        mut self,
        handler: impl Fn(usize, &SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_react = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for CommentThread {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let resolved = self.thread.resolved;
        let resolve = self.on_resolve.clone().map(|resolve| {
            Button::new(
                (self.id.clone(), "resolve"),
                i18n::text(
                    cx,
                    if resolved {
                        "comments.reopen"
                    } else {
                        "comments.resolve"
                    },
                    &[],
                ),
            )
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Sm)
            .icon(if resolved {
                IconName::RotateCcw
            } else {
                IconName::Check
            })
            .on_click(move |_, window, cx| {
                cx.stop_propagation();
                log::info!("comment thread: resolved {}", !resolved);
                resolve(!resolved, window, cx)
            })
        });
        let comments = self
            .thread
            .comments
            .iter()
            .enumerate()
            .map(|(ix, comment)| {
                let react = self.on_react.clone();
                let reactions = (!comment.reactions.is_empty() || react.is_some()).then(|| {
                    let reactions = Reactions::new(
                        (self.id.clone(), format!("reactions-{ix}")),
                        comment.reactions.clone(),
                    );
                    match react {
                        Some(react) => reactions
                            .on_toggle(move |emoji, window, cx| react(ix, emoji, window, cx)),
                        None => reactions,
                    }
                });
                div()
                    .flex()
                    .gap_2()
                    .child(
                        comment
                            .author
                            .avatar((self.id.clone(), format!("author-{ix}")), AvatarSize::Xs),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .flex()
                                    .items_baseline()
                                    .gap_2()
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(comment.author.name.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(theme.text_size(TextSize::Xs))
                                            .text_color(colors.fg_subtle)
                                            .child(RelativeTime::new(
                                                (self.id.clone(), format!("when-{ix}")),
                                                comment.at,
                                            )),
                                    ),
                            )
                            .child(
                                div()
                                    .text_color(if resolved { colors.fg_muted } else { colors.fg })
                                    .child(comment.body.clone()),
                            )
                            .children(reactions),
                    )
            });
        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(if self.active {
                colors.focus
            } else {
                colors.border
            })
            .bg(colors.surface)
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .children(self.thread.quote.clone().map(|quote| {
                                div()
                                    .pl_2()
                                    .border_l_2()
                                    .border_color(colors.border_strong)
                                    .text_color(colors.fg_muted)
                                    .italic()
                                    .child(quote)
                            })),
                    )
                    .children(resolve),
            )
            .children(comments)
            .when(!resolved, |thread| {
                thread.children(self.reply.map(|(field, send)| {
                    Reply::new((self.id.clone(), "reply"), &field, move |window, cx| {
                        send(window, cx)
                    })
                }))
            })
    }
}

/// A comment's mark beside the words it is about, with how many comments it holds; a press opens the thread in a bubble over the page.
#[derive(IntoElement)]
pub struct CommentMarker {
    id: ElementId,
    thread: Thread,
    reply: Option<(Entity<TextInput>, Run)>,
    on_resolve: Option<OnFlag>,
}

impl CommentMarker {
    pub fn new(id: impl Into<ElementId>, thread: Thread) -> Self {
        Self {
            id: id.into(),
            thread,
            reply: None,
            on_resolve: None,
        }
    }

    pub fn reply(
        mut self,
        field: &Entity<TextInput>,
        on_send: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.reply = Some((field.clone(), Rc::new(on_send)));
        self
    }

    pub fn on_resolve(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_resolve = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for CommentMarker {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let prose = cx.theme().prose_width();
        let count = self.thread.comments.len().to_string();
        let (id, thread, reply, resolve) =
            (self.id.clone(), self.thread, self.reply, self.on_resolve);
        Popover::new(self.id, count, move |_, _| {
            let mut bubble = CommentThread::new((id.clone(), "thread"), thread);
            if let Some((field, send)) = reply {
                bubble = bubble.reply(&field, move |window, cx| send(window, cx));
            }
            if let Some(resolve) = resolve {
                bubble = bubble.on_resolve(move |on, window, cx| resolve(on, window, cx));
            }
            div().w(prose).child(bubble)
        })
        .icon(IconName::MessageSquare)
        .variant(ButtonVariant::Ghost)
        .size(ControlSize::Sm)
    }
}
