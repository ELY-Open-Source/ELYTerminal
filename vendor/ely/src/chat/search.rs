use std::{ops::Range, rc::Rc};

use gpui::{
    Animation, AnimationExt, AnyElement, App, ElementId, FontWeight, HighlightStyle, Hsla,
    InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, StyledText, Window, div, prelude::*, transparent_black,
};

use super::cite::Source;
use crate::{
    forms::Run,
    motion::{self, Spinner},
    primitives::{FocusRing, Icon, IconName},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
    typography::{Ellipsis, tabular},
};

/// Where a step stands: a search's, a tool call's or an agent's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepState {
    Waiting,
    Working,
    Done,
    Failed,
}

/// A step's mark in an icon's box: a dot waiting, a spinner working, a check done, a cross failed. A check or cross that arrives pops in.
pub(crate) fn step_mark(
    id: ElementId,
    state: StepState,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let changes = motion::changes((id.clone(), "changes"), state, window, cx);
    let theme = cx.theme();
    let colors = &theme.colors;
    let pop = |icon: IconName, ink: Hsla| {
        let icon = Icon::new(icon).size(IconSize::Sm).color(ink);
        if changes == 0 {
            return icon.into_any_element();
        }
        icon.with_animation(
            (id.clone(), format!("pop-{changes}")),
            Animation::new(motion::duration(motion::FAST, cx)).with_easing(motion::ease_out_cubic),
            move |icon, t| icon.scale(0.6 + 0.4 * t).color(ink.opacity(t)),
        )
        .into_any_element()
    };
    let mark = match state {
        StepState::Done => pop(IconName::Check, colors.success),
        StepState::Failed => pop(IconName::X, colors.danger),
        StepState::Working => Spinner::new(id).size(IconSize::Sm).into_any_element(),
        StepState::Waiting => div()
            .size(theme.status_dot())
            .rounded_full()
            .bg(colors.fg_subtle)
            .into_any_element(),
    };
    div()
        .flex_none()
        .size(theme.icon_size(IconSize::Sm))
        .flex()
        .items_center()
        .justify_center()
        .child(mark)
        .into_any_element()
}

/// A search as it runs, step by step: what it looks for, what it reads, each step waiting, working or done.
#[derive(IntoElement)]
pub struct SearchProgress {
    id: ElementId,
    steps: Vec<(SharedString, StepState)>,
}

impl SearchProgress {
    pub fn new(
        id: impl Into<ElementId>,
        steps: impl IntoIterator<Item = (impl Into<SharedString>, StepState)>,
    ) -> Self {
        Self {
            id: id.into(),
            steps: steps
                .into_iter()
                .map(|(label, state)| (label.into(), state))
                .collect(),
        }
    }
}

impl RenderOnce for SearchProgress {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let marks: Vec<AnyElement> = (self.steps.iter().enumerate())
            .map(|(ix, (_, state))| {
                step_mark(
                    (self.id.clone(), format!("step-{ix}")).into(),
                    *state,
                    window,
                    cx,
                )
            })
            .collect();
        let theme = cx.theme();
        let colors = theme.colors.clone();
        div()
            .flex()
            .flex_col()
            .gap_1p5()
            .text_size(theme.text_size(TextSize::Sm))
            .children(
                self.steps
                    .into_iter()
                    .zip(marks)
                    .map(|((label, state), mark)| {
                        div().flex().items_center().gap_2().child(mark).child(
                            div()
                                .text_color(match state {
                                    StepState::Waiting => colors.fg_subtle,
                                    StepState::Working => colors.fg,
                                    StepState::Done => colors.fg_muted,
                                    StepState::Failed => colors.danger,
                                })
                                .child(label),
                        )
                    }),
            )
    }
}

/// A search result: its title as a link, where it lives, a line from it and when it was written; a press opens it.
#[derive(IntoElement)]
pub struct WebResultCard {
    id: ElementId,
    source: Source,
    date: Option<SharedString>,
    on_open: Option<Run>,
}

impl WebResultCard {
    pub fn new(id: impl Into<ElementId>, source: Source) -> Self {
        Self {
            id: id.into(),
            source,
            date: None,
            on_open: None,
        }
    }

    pub fn date(mut self, date: impl Into<SharedString>) -> Self {
        self.date = Some(date.into());
        self
    }

    pub fn on_open(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for WebResultCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let open = self.on_open;
        div()
            .id(self.id.clone())
            .flex()
            .flex_col()
            .gap_0p5()
            .py_2()
            .when_some(open, |card, open| {
                card.px_1()
                    .rounded(theme.radius(Radius::Md))
                    .border_1()
                    .border_color(transparent_black())
                    .tab_index(0)
                    .focus_ring(cx)
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .on_click(move |_, window, cx| open(window, cx))
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.fg_subtle)
                    .child(
                        Icon::new(IconName::Globe)
                            .size(IconSize::Xs)
                            .color(colors.fg_subtle),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(Ellipsis::new(self.source.url.clone())),
                    )
                    .children(self.date.map(|date| div().flex_none().child(date))),
            )
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Base))
                    .text_color(colors.link)
                    .child(self.source.title),
            )
            .children(self.source.snippet.map(|snippet| {
                div()
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(colors.fg_muted)
                    .child(snippet)
            }))
    }
}

/// A passage found in a document for an answer: the document and where in it, the passage with the words that matched washed, and how close a match it was.
#[derive(IntoElement)]
pub struct DocumentChunkPreview {
    document: SharedString,
    place: SharedString,
    text: SharedString,
    matched: Vec<Range<usize>>,
    score: Option<f32>,
}

impl DocumentChunkPreview {
    /// `matched` are byte ranges in `text`, in order.
    pub fn new(
        document: impl Into<SharedString>,
        place: impl Into<SharedString>,
        text: impl Into<SharedString>,
        matched: Vec<Range<usize>>,
    ) -> Self {
        let text: SharedString = text.into();
        let matched: Vec<Range<usize>> = matched
            .into_iter()
            .filter(|range| {
                let inside = text.get(range.clone()).is_some();
                if !inside {
                    log::error!(
                        "document chunk: match {range:?} lies outside the passage; left out"
                    );
                }
                inside
            })
            .collect();
        assert!(
            matched.windows(2).all(|pair| pair[0].end <= pair[1].start),
            "matches come in order without overlapping"
        );
        Self {
            document: document.into(),
            place: place.into(),
            text,
            matched,
            score: None,
        }
    }

    /// How close a match, 0 to 1.
    pub fn score(mut self, score: f32) -> Self {
        assert!(
            (0.0..=1.0).contains(&score),
            "a score of {score} lies outside 0 to 1"
        );
        self.score = Some(score);
        self
    }
}

impl RenderOnce for DocumentChunkPreview {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let wash = HighlightStyle {
            background_color: Some(colors.warning.opacity(0.25)),
            ..HighlightStyle::default()
        };
        div()
            .flex()
            .flex_col()
            .gap_1p5()
            .p_3()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.fg_subtle)
                    .child(
                        Icon::new(IconName::FileText)
                            .size(IconSize::Xs)
                            .color(colors.fg_subtle),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.fg_muted)
                            .child(Ellipsis::new(self.document)),
                    )
                    .child(div().flex_1().min_w_0().child(Ellipsis::new(self.place)))
                    .children(self.score.map(|score| {
                        tabular(div().flex_none()).child(format!("{:.0}% match", score * 100.0))
                    })),
            )
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(colors.fg)
                    .child(
                        StyledText::new(self.text)
                            .with_highlights(self.matched.into_iter().map(|range| (range, wash))),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::DocumentChunkPreview;

    #[test]
    fn a_match_outside_the_passage_is_left_out() {
        let ranges = vec![1..2, 2..3, 3..9];
        let preview = DocumentChunkPreview::new("guide.md", "p. 2", "éabc", ranges);
        let kept = 2..3;
        assert_eq!(preview.matched, [kept]);
    }
}
