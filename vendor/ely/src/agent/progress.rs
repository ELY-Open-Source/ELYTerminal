use std::time::Duration;

use gpui::{
    AnyElement, App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div, prelude::*,
};

use crate::{
    chat::{StepState, step_mark},
    data_display::{Timeline, TimelineItem},
    motion::{ProgressBar, Spinner},
    primitives::{Icon, IconName, Severity},
    theme::{ActiveTheme, IconSize, TextSize},
    typography::{Ellipsis, format::took, tabular},
};

/// One step of an agent's work: what it did, where it stands, a line below, how long it took.
#[derive(Clone, Debug, PartialEq)]
pub struct AgentStep {
    pub title: SharedString,
    pub state: StepState,
    pub detail: Option<SharedString>,
    pub took: Option<Duration>,
}

/// An agent's steps down a rail, each marked by where it stands.
#[derive(IntoElement)]
pub struct AgentStepList {
    id: ElementId,
    steps: Vec<AgentStep>,
}

impl AgentStepList {
    pub fn new(id: impl Into<ElementId>, steps: impl IntoIterator<Item = AgentStep>) -> Self {
        Self {
            id: id.into(),
            steps: steps.into_iter().collect(),
        }
    }
}

impl RenderOnce for AgentStepList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let marks: Vec<AnyElement> = (self.steps.iter().enumerate())
            .map(|(ix, step)| {
                step_mark(
                    (self.id.clone(), format!("step-{ix}")).into(),
                    step.state,
                    window,
                    cx,
                )
            })
            .collect();
        let colors = cx.theme().colors.clone();
        self.steps
            .into_iter()
            .zip(marks)
            .fold(Timeline::new(), |timeline, (step, mark)| {
                let title = div()
                    .text_color(match step.state {
                        StepState::Waiting => colors.fg_subtle,
                        _ => colors.fg,
                    })
                    .child(step.title);
                let item = TimelineItem::new(title)
                    .marker(mark)
                    .when_some(step.took, |item, spent| item.time(took(spent)));
                timeline.item(match step.detail {
                    Some(detail) => item.child(detail),
                    None => item,
                })
            })
    }
}

/// An agent's plan: items waiting, at work or done, with how many are done; a done item goes quiet and struck through.
#[derive(IntoElement)]
pub struct AgentPlan {
    id: ElementId,
    title: SharedString,
    items: Vec<(SharedString, StepState)>,
}

impl AgentPlan {
    pub fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        items: impl IntoIterator<Item = (impl Into<SharedString>, StepState)>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            items: items
                .into_iter()
                .map(|(label, state)| (label.into(), state))
                .collect(),
        }
    }
}

impl RenderOnce for AgentPlan {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let marks: Vec<AnyElement> = (self.items.iter().enumerate())
            .map(|(ix, (_, state))| {
                step_mark(
                    (self.id.clone(), format!("item-{ix}")).into(),
                    *state,
                    window,
                    cx,
                )
            })
            .collect();
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let done = self
            .items
            .iter()
            .filter(|(_, state)| *state == StepState::Done)
            .count();
        div()
            .flex()
            .flex_col()
            .gap_2()
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
                            .font_weight(FontWeight::MEDIUM)
                            .child(Ellipsis::new(self.title)),
                    )
                    .child(
                        tabular(div().flex_none().text_color(colors.fg_subtle))
                            .child(format!("{done} of {}", self.items.len())),
                    ),
            )
            .children(
                self.items
                    .into_iter()
                    .zip(marks)
                    .map(|((label, state), mark)| {
                        div().flex().items_center().gap_2().child(mark).child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_color(match state {
                                    StepState::Waiting => colors.fg_muted,
                                    StepState::Working => colors.fg,
                                    StepState::Done => colors.fg_subtle,
                                    StepState::Failed => Severity::Danger.color(&colors),
                                })
                                .when(state == StepState::Done, |text| text.line_through())
                                .child(label),
                        )
                    }),
            )
    }
}

/// Where an agent stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentState {
    Planning,
    Working,
    Waiting,
    Done,
    Failed,
}

impl AgentState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Planning => "Planning",
            Self::Working => "Working",
            Self::Waiting => "Waiting for you",
            Self::Done => "Done",
            Self::Failed => "Failed",
        }
    }
}

/// A pill that says where an agent stands, moving while it plans or works.
#[derive(IntoElement)]
pub struct AgentStatus {
    id: ElementId,
    state: AgentState,
}

impl AgentStatus {
    pub fn new(id: impl Into<ElementId>, state: AgentState) -> Self {
        Self {
            id: id.into(),
            state,
        }
    }
}

impl RenderOnce for AgentStatus {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let tone = match self.state {
            AgentState::Planning | AgentState::Working => Severity::Info,
            AgentState::Waiting => Severity::Warning,
            AgentState::Done => Severity::Success,
            AgentState::Failed => Severity::Danger,
        };
        let ink = tone.color(&colors);
        let mark = match self.state {
            AgentState::Planning | AgentState::Working => Spinner::new((self.id.clone(), "mark"))
                .size(IconSize::Xs)
                .color(ink)
                .into_any_element(),
            AgentState::Waiting => Icon::new(IconName::Hand)
                .size(IconSize::Xs)
                .color(ink)
                .into_any_element(),
            AgentState::Done => Icon::new(IconName::Check)
                .size(IconSize::Xs)
                .color(ink)
                .into_any_element(),
            AgentState::Failed => Icon::new(IconName::X)
                .size(IconSize::Xs)
                .color(ink)
                .into_any_element(),
        };
        div()
            .flex()
            .items_center()
            .gap_1p5()
            .px_2()
            .py_0p5()
            .rounded_full()
            .bg(tone.subtle(&colors))
            .text_size(theme.text_size(TextSize::Xs))
            .font_weight(FontWeight::MEDIUM)
            .text_color(ink)
            .child(mark)
            .child(self.state.label())
    }
}

/// How far an agent has come: the step at work, how many are done of how many, the time so far, and a bar split by step when there are two or more.
#[derive(IntoElement)]
pub struct AgentProgress {
    id: ElementId,
    current: SharedString,
    done: usize,
    total: usize,
    elapsed: Option<Duration>,
}

impl AgentProgress {
    pub fn new(
        id: impl Into<ElementId>,
        current: impl Into<SharedString>,
        done: usize,
        total: usize,
    ) -> Self {
        assert!(total > 0, "progress of {done} in none");
        if done > total {
            log::error!("agent progress: {done} done of {total}; the bar fills");
        }
        Self {
            id: id.into(),
            current: current.into(),
            done,
            total,
            elapsed: None,
        }
    }

    pub fn elapsed(mut self, elapsed: Duration) -> Self {
        self.elapsed = Some(elapsed);
        self
    }
}

impl RenderOnce for AgentProgress {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        div()
            .flex()
            .flex_col()
            .gap_2()
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
                            .text_color(colors.fg)
                            .child(Ellipsis::new(self.current)),
                    )
                    .child(
                        tabular(div().flex_none().text_color(colors.fg_subtle))
                            .child(format!("{} of {}", self.done, self.total)),
                    )
                    .children(self.elapsed.map(|spent| {
                        tabular(div().flex_none().text_color(colors.fg_subtle)).child(took(spent))
                    })),
            )
            .child({
                let bar = ProgressBar::new(
                    (self.id, "bar"),
                    self.done.min(self.total) as f32 / self.total as f32,
                );
                if self.total >= 2 {
                    bar.segments(self.total)
                } else {
                    bar
                }
            })
    }
}
