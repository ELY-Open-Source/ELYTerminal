use std::{rc::Rc, time::Duration};

use gpui::{
    App, ElementId, Entity, FontWeight, InteractiveElement, IntoElement, MouseButton,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::*, relative, transparent_black,
};
use web_time::Instant;

use super::stream::StreamingText;
use crate::{
    buttons::{Button, ButtonVariant},
    feedback::{Alert, Countdown},
    forms::{Choice, ChoiceChips, Input, OnValues, Run, TextInput},
    primitives::{Disclosure, FocusRing, Icon, IconName, Severity},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
    typography::ShimmerText,
};

/// An answer that failed: what went wrong in a line, a detail, and a way to try again.
#[derive(IntoElement)]
pub struct ErrorMessage {
    id: ElementId,
    title: SharedString,
    detail: Option<SharedString>,
    on_retry: Option<Run>,
}

impl ErrorMessage {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            detail: None,
            on_retry: None,
        }
    }

    pub fn detail(mut self, text: impl Into<SharedString>) -> Self {
        self.detail = Some(text.into());
        self
    }

    pub fn on_retry(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_retry = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ErrorMessage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let alert = Alert::new(self.id.clone(), Severity::Danger, self.title);
        let alert = match self.detail {
            Some(detail) => alert.body(detail),
            None => alert,
        };
        match self.on_retry {
            Some(retry) => alert.action(
                Button::new((self.id.clone(), "retry"), "Try again")
                    .size(ControlSize::Sm)
                    .icon(IconName::RefreshCw)
                    .on_click(move |_, window, cx| {
                        log::info!("error message: retry");
                        retry(window, cx)
                    }),
            ),
            None => alert,
        }
    }
}

/// Out of messages for now: when more come back, counting down, and the owner's way to raise the limit.
#[derive(IntoElement)]
pub struct RateLimitNotice {
    id: ElementId,
    until: Instant,
    on_done: Option<Run>,
    on_upgrade: Option<Run>,
}

impl RateLimitNotice {
    pub fn new(id: impl Into<ElementId>, until: Instant) -> Self {
        Self {
            id: id.into(),
            until,
            on_done: None,
            on_upgrade: None,
        }
    }

    /// Runs once the wait is over.
    pub fn on_done(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_done = Some(Rc::new(handler));
        self
    }

    pub fn on_upgrade(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_upgrade = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for RateLimitNotice {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let countdown = Countdown::new((self.id.clone(), "left"), self.until).size(TextSize::Sm);
        let countdown = match self.on_done {
            Some(done) => countdown.on_done(move |window, cx| done(window, cx)),
            None => countdown,
        };
        div()
            .flex()
            .items_center()
            .gap_3()
            .px_3()
            .py_2()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                Icon::new(IconName::Clock)
                    .size(IconSize::Sm)
                    .color(colors.warning),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .text_color(colors.fg)
                            .child("You've reached the message limit. More in"),
                    )
                    .child(countdown),
            )
            .children(self.on_upgrade.map(|upgrade| {
                Button::new((self.id.clone(), "upgrade"), "Raise the limit")
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .on_click(move |_, window, cx| upgrade(window, cx))
            }))
    }
}

/// What went wrong with an answer, after a bad mark: reasons to pick, a note, then send; sending waits for a reason or a note.
#[derive(IntoElement)]
pub struct FeedbackForm {
    id: ElementId,
    reasons: Vec<SharedString>,
    picked: Vec<SharedString>,
    note: Entity<TextInput>,
    on_pick: OnValues,
    on_send: Run,
    on_cancel: Option<Run>,
}

impl FeedbackForm {
    /// `picked` are the reasons chosen and `on_pick` gets the next set; `note` is the owner's field; `on_send` sends both.
    pub fn new(
        id: impl Into<ElementId>,
        reasons: impl IntoIterator<Item = impl Into<SharedString>>,
        picked: impl IntoIterator<Item = impl Into<SharedString>>,
        note: &Entity<TextInput>,
        on_pick: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
        on_send: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            reasons: reasons.into_iter().map(Into::into).collect(),
            picked: picked.into_iter().map(Into::into).collect(),
            note: note.clone(),
            on_pick: Rc::new(on_pick),
            on_send: Rc::new(on_send),
            on_cancel: None,
        }
    }

    pub fn on_cancel(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_cancel = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for FeedbackForm {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let ready = !self.picked.is_empty() || !self.note.read(cx).text().trim().is_empty();
        let (pick, send) = (self.on_pick, self.on_send);
        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Sm))
                    .font_weight(FontWeight::MEDIUM)
                    .child("What went wrong?"),
            )
            .child(
                ChoiceChips::new(
                    (self.id.clone(), "reasons"),
                    self.reasons
                        .iter()
                        .map(|reason| Choice::new(reason.clone(), reason.clone())),
                )
                .multiple()
                .selected(self.picked.clone())
                .on_change(move |next, window, cx| pick(next, window, cx)),
            )
            .child(Input::new(&self.note))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .children(self.on_cancel.map(|cancel| {
                        Button::new((self.id.clone(), "cancel"), "Cancel")
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Sm)
                            .on_click(move |_, window, cx| cancel(window, cx))
                    }))
                    .child(
                        Button::new((self.id.clone(), "send"), "Send feedback")
                            .variant(ButtonVariant::Primary)
                            .size(ControlSize::Sm)
                            .disabled(!ready)
                            .on_click(move |_, window, cx| {
                                log::info!("feedback form: sent");
                                send(window, cx)
                            }),
                    ),
            )
    }
}

/// How long thinking took, in words.
pub(crate) fn thought_for(took: Duration) -> String {
    match took.as_secs() {
        0 => "Thought for a moment".to_string(),
        1 => "Thought for 1 second".to_string(),
        seconds @ 2..60 => format!("Thought for {seconds} seconds"),
        seconds => format!("Thought for {}m {}s", seconds / 60, seconds % 60),
    }
}

/// How long thinking took: a quiet line, such as “Thought for 12 seconds”.
#[derive(IntoElement)]
pub struct ThinkingDuration {
    took: Duration,
}

impl ThinkingDuration {
    pub fn new(took: Duration) -> Self {
        Self { took }
    }
}

impl RenderOnce for ThinkingDuration {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .text_color(cx.theme().colors.fg_muted)
            .child(thought_for(self.took))
    }
}

/// Work under way before an answer: a word, such as Thinking, with light sweeping across it; still under reduced motion.
#[derive(IntoElement)]
pub struct ThinkingIndicator {
    id: ElementId,
    label: SharedString,
}

impl ThinkingIndicator {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: "Thinking".into(),
        }
    }

    /// What the work is, such as Searching the web.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }
}

impl RenderOnce for ThinkingIndicator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors.clone();
        div()
            .flex()
            .items_center()
            .gap_1p5()
            .child(
                Icon::new(IconName::Sparkles)
                    .size(IconSize::Sm)
                    .color(colors.fg_subtle),
            )
            .child(ShimmerText::new(self.id, self.label))
    }
}

/// The reasoning behind an answer, folded under how long it took; open, its steps read in a quiet column. While thinking, it says so and the reasoning streams in.
#[derive(IntoElement)]
pub struct ThinkingBlock {
    id: ElementId,
    reasoning: SharedString,
    thinking: bool,
    took: Duration,
}

impl ThinkingBlock {
    /// `took` is how long thinking ran, or has run while `thinking`.
    pub fn new(
        id: impl Into<ElementId>,
        reasoning: impl Into<SharedString>,
        thinking: bool,
        took: Duration,
    ) -> Self {
        Self {
            id: id.into(),
            reasoning: reasoning.into(),
            thinking,
            took,
        }
    }
}

impl RenderOnce for ThinkingBlock {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open = window.use_keyed_state((self.id.clone(), "open"), cx, |_, _| false);
        let opened = *open.read(cx);
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let header: gpui::AnyElement = if self.thinking {
            ThinkingIndicator::new((self.id.clone(), "thinking")).into_any_element()
        } else {
            ThinkingDuration::new(self.took).into_any_element()
        };
        div()
            .flex()
            .flex_col()
            .gap_2()
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div()
                    .id((self.id.clone(), "header"))
                    .flex()
                    .items_center()
                    .gap_1()
                    .px_1()
                    .rounded(theme.radius(Radius::Sm))
                    .border_1()
                    .border_color(transparent_black())
                    .tab_index(0)
                    .focus_ring(cx)
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .on_click(move |_, _, cx| {
                        open.update(cx, |open, cx| {
                            *open = !*open;
                            log::info!("thinking block: open {open}");
                            cx.notify();
                        })
                    })
                    .child(header)
                    .child(
                        Disclosure::new((self.id.clone(), "chevron"), opened).size(IconSize::Sm),
                    ),
            )
            .when(opened, |block| {
                block.child(
                    div()
                        .pl_3()
                        .border_l_2()
                        .border_color(colors.border)
                        .text_color(colors.fg_muted)
                        .line_height(relative(1.6))
                        .child(StreamingText::new(
                            (self.id.clone(), "reasoning"),
                            self.reasoning,
                            self.thinking,
                        )),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::thought_for;
    use std::time::Duration;

    #[test]
    fn thinking_time_reads_in_words() {
        assert_eq!(
            thought_for(Duration::from_millis(400)),
            "Thought for a moment"
        );
        assert_eq!(thought_for(Duration::from_secs(1)), "Thought for 1 second");
        assert_eq!(
            thought_for(Duration::from_secs(12)),
            "Thought for 12 seconds"
        );
        assert_eq!(thought_for(Duration::from_secs(75)), "Thought for 1m 15s");
    }
}
