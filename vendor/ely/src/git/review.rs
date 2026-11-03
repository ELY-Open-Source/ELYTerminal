use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, Entity, FontWeight, Hsla, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::*, relative,
};

use super::badges::DiffStat;
use crate::{
    buttons::{Button, ButtonVariant},
    data_display::{Avatar, AvatarGroup, Tag, Tone},
    forms::{Input, TextInput},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, AvatarSize, ControlSize, IconSize, Palette, Radius, TextSize},
    typography::{Ellipsis, LEADING, format},
};

type Run = Rc<dyn Fn(&mut Window, &mut App)>;
type OnBool = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// Where a pull request stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PullState {
    Open,
    Draft,
    Merged,
    Closed,
}

impl PullState {
    fn look(self, colors: &Palette) -> (IconName, Hsla, &'static str) {
        match self {
            Self::Open => (IconName::GitPullRequest, colors.success, "Open"),
            Self::Draft => (IconName::GitPullRequestDraft, colors.fg_muted, "Draft"),
            Self::Merged => (IconName::GitMerge, colors.info, "Merged"),
            Self::Closed => (IconName::GitPullRequestClosed, colors.danger, "Closed"),
        }
    }
}

/// A pull request: number, title and author, the branch it merges and where, its state, checks, reviewers, comments and size.
#[derive(Clone, Debug, PartialEq)]
pub struct PullRequest {
    pub number: u32,
    pub title: SharedString,
    pub author: SharedString,
    pub head: SharedString,
    pub base: SharedString,
    pub state: PullState,
    /// Checks passed, failed, and still running.
    pub checks: (usize, usize, usize),
    pub reviewers: Vec<SharedString>,
    pub comments: usize,
    pub added: usize,
    pub removed: usize,
    pub when: SharedString,
}

/// A pull request at a glance: state and title, the branches, checks, reviewers, comments and size; a press opens it.
#[derive(IntoElement)]
pub struct PullRequestCard {
    id: ElementId,
    pull: PullRequest,
    on_open: Option<Run>,
}

impl PullRequestCard {
    pub fn new(id: impl Into<ElementId>, pull: PullRequest) -> Self {
        Self {
            id: id.into(),
            pull,
            on_open: None,
        }
    }

    pub fn on_open(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for PullRequestCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let pull = self.pull;
        let (icon, tint, state) = pull.state.look(&colors);
        let (passed, failed, running) = pull.checks;
        let (check_icon, check_tint) = match (failed, running) {
            (0, 0) => (IconName::CircleCheck, colors.success),
            (0, _) => (IconName::LoaderCircle, colors.warning),
            _ => (IconName::CircleX, colors.danger),
        };
        let checks = [(passed, "passed"), (failed, "failed"), (running, "running")]
            .iter()
            .filter(|(count, _)| *count > 0)
            .map(|(count, word)| format!("{count} {word}"))
            .collect::<Vec<_>>()
            .join(" · ");
        let branch = |name: SharedString, key: &str| {
            Tag::new((self.id.clone(), key.to_string()), name).tone(Tone::Neutral)
        };
        let open = self.on_open;
        div()
            .id(self.id.clone())
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .text_size(theme.text_size(TextSize::Sm))
            .when_some(open, |card, open| {
                card.cursor_pointer()
                    .hover(|card| card.border_color(colors.border_strong))
                    .on_click(move |_, window, cx| open(window, cx))
            })
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_2()
                    .child(Icon::new(icon).size(IconSize::Md).color(tint))
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
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(colors.fg)
                                            .child(Ellipsis::new(pull.title)),
                                    )
                                    .child(
                                        div()
                                            .flex_none()
                                            .text_color(colors.fg_subtle)
                                            .child(format!("#{}", pull.number)),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1p5()
                                    .text_color(colors.fg_muted)
                                    .child(format!("{state} · {} wants to merge", pull.author))
                                    .child(branch(pull.head, "head"))
                                    .child("into")
                                    .child(branch(pull.base, "base"))
                                    .child(div().text_color(colors.fg_subtle).child(pull.when)),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .pl_6()
                    .text_color(colors.fg_muted)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(Icon::new(check_icon).size(IconSize::Sm).color(check_tint))
                            .child(checks),
                    )
                    .child(
                        AvatarGroup::new(pull.reviewers.iter().enumerate().map(|(ix, name)| {
                            Avatar::new((self.id.clone(), format!("reviewer-{ix}")), name.clone())
                        }))
                        .size(AvatarSize::Xs),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                Icon::new(IconName::MessageSquare)
                                    .size(IconSize::Sm)
                                    .color(colors.fg_subtle),
                            )
                            .child(pull.comments.to_string()),
                    )
                    .child(div().flex_1())
                    .child(DiffStat::new(pull.added, pull.removed)),
            )
    }
}

/// One comment in a review thread.
#[derive(Clone, Debug, PartialEq)]
pub struct ReviewNote {
    pub author: SharedString,
    pub when: SharedString,
    pub body: SharedString,
}

/// A review thread on a line: the line it is about, each comment, a reply field, and resolve or reopen; a resolved thread quiets down.
#[derive(IntoElement)]
pub struct ReviewComment {
    id: ElementId,
    path: SharedString,
    line: usize,
    code: SharedString,
    notes: Vec<ReviewNote>,
    resolved: bool,
    reply: Entity<TextInput>,
    on_reply: Option<Run>,
    on_resolve: Option<OnBool>,
}

impl ReviewComment {
    /// `line` counts from one; `code` is its text.
    pub fn new(
        id: impl Into<ElementId>,
        path: impl Into<SharedString>,
        line: usize,
        code: impl Into<SharedString>,
        notes: impl IntoIterator<Item = ReviewNote>,
        reply: &Entity<TextInput>,
    ) -> Self {
        Self {
            id: id.into(),
            path: path.into(),
            line,
            code: code.into(),
            notes: notes.into_iter().collect(),
            resolved: false,
            reply: reply.clone(),
            on_reply: None,
            on_resolve: None,
        }
    }

    pub fn resolved(mut self, resolved: bool) -> Self {
        self.resolved = resolved;
        self
    }

    pub fn on_reply(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_reply = Some(Rc::new(handler));
        self
    }

    /// Called with true to resolve the thread, false to reopen it.
    pub fn on_resolve(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_resolve = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ReviewComment {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let notes: Vec<AnyElement> = self
            .notes
            .iter()
            .enumerate()
            .map(|(ix, note)| {
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Avatar::new((self.id.clone(), format!("note-{ix}")), note.author.clone())
                            .size(AvatarSize::Sm),
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
                                    .gap_2()
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(colors.fg)
                                            .child(note.author.clone()),
                                    )
                                    .child(
                                        div().text_color(colors.fg_subtle).child(note.when.clone()),
                                    ),
                            )
                            .child(div().text_color(colors.fg).child(note.body.clone())),
                    )
                    .into_any_element()
            })
            .collect();
        let (resolve, resolved) = (self.on_resolve, self.resolved);
        let reply = self.on_reply;
        div()
            .id(self.id.clone())
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .text_size(theme.text_size(TextSize::Sm))
            .when(resolved, |thread| thread.opacity(0.7))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Icon::new(IconName::MessageSquare)
                            .size(IconSize::Sm)
                            .color(colors.fg_muted),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_color(colors.fg_muted)
                            .child(format!("{} line {}", self.path, self.line)),
                    )
                    .when(resolved, |header| {
                        header.child(
                            Tag::new((self.id.clone(), "resolved"), "Resolved").tone(Tone::Success),
                        )
                    })
                    .child(format::plural(
                        self.notes.len() as u64,
                        "comment",
                        "comments",
                    )),
            )
            .child(
                div()
                    .px_2()
                    .py_1()
                    .rounded(theme.radius(Radius::Sm))
                    .bg(colors.sunken)
                    .font_family(theme.mono_family.clone())
                    .text_size(theme.text_size(TextSize::Xs))
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .line_height(relative(LEADING))
                    .child(format!("{}  {}", self.line, self.code)),
            )
            .children(notes)
            .when(!resolved, |thread| {
                thread.child(
                    div()
                        .flex()
                        .gap_2()
                        .child(div().flex_1().child(Input::new(&self.reply)))
                        .child(
                            Button::new((self.id.clone(), "reply"), "Reply")
                                .variant(ButtonVariant::Primary)
                                .size(ControlSize::Sm)
                                .when_some(reply, |button, reply| {
                                    button.on_click(move |_, window, cx| reply(window, cx))
                                }),
                        ),
                )
            })
            .child(
                div().flex().child(
                    Button::new(
                        (self.id.clone(), "resolve"),
                        if resolved {
                            "Reopen conversation"
                        } else {
                            "Resolve conversation"
                        },
                    )
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .when_some(resolve, |button, resolve| {
                        button.on_click(move |_, window, cx| resolve(!resolved, window, cx))
                    }),
                ),
            )
    }
}
