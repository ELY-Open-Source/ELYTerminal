use std::{rc::Rc, time::Duration};

use gpui::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
    prelude::*,
};

use crate::{
    buttons::{Button, ButtonVariant, IconButton},
    documents::source,
    forms::OnValue,
    motion::{AnimatePresence, ProgressRing},
    primitives::{Icon, IconName, Image},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
    typography::{
        Ellipsis,
        format::{DurationStyle, duration, percent},
    },
};

/// Where a generation stands.
#[derive(Clone, Debug, PartialEq)]
pub enum JobState {
    /// Waiting, with this many ahead of it.
    Queued(usize),
    /// Under way: the share done, and the time left when known.
    Running(f32, Option<Duration>),
    /// Finished, and how long it took.
    Done(Duration),
    Failed(SharedString),
}

/// One generation in line: its key, its prompt, where it stands, and its picture once done.
#[derive(Clone, Debug, PartialEq)]
pub struct Job {
    pub key: SharedString,
    pub prompt: SharedString,
    pub state: JobState,
    pub picture: Option<SharedString>,
}

/// What a job's state says under its prompt.
pub(crate) fn status(state: &JobState) -> String {
    match state {
        JobState::Queued(0) => "Queued · next".to_string(),
        JobState::Queued(ahead) => format!("Queued · {ahead} ahead"),
        JobState::Running(share, None) => percent(f64::from(*share), 0, false),
        JobState::Running(share, Some(left)) => format!(
            "{} · about {} left",
            percent(f64::from(*share), 0, false),
            duration(left.as_secs().max(1), DurationStyle::Compact)
        ),
        JobState::Done(took) => format!(
            "Done in {}",
            duration(took.as_secs().max(1), DurationStyle::Compact)
        ),
        JobState::Failed(reason) => format!("Failed · {reason}"),
    }
}

/// Generations in line, newest last: each row shows its picture or where it stands, its prompt, and how far it has come; Cancel while it waits or runs, Retry once it fails, and Remove once it ends. New rows fold in.
#[derive(IntoElement)]
pub struct GenerationQueue {
    id: ElementId,
    jobs: Vec<Job>,
    on_cancel: Option<OnValue>,
    on_retry: Option<OnValue>,
    on_remove: Option<OnValue>,
}

impl GenerationQueue {
    pub fn new(id: impl Into<ElementId>, jobs: impl IntoIterator<Item = Job>) -> Self {
        let jobs: Vec<Job> = jobs.into_iter().collect();
        for job in &jobs {
            if let JobState::Running(share, _) = job.state {
                assert!(
                    (0.0..=1.0).contains(&share),
                    "job {} at {share} of 1",
                    job.key
                );
            }
        }
        Self {
            id: id.into(),
            jobs,
            on_cancel: None,
            on_retry: None,
            on_remove: None,
        }
    }

    pub fn on_cancel(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_cancel = Some(Rc::new(handler));
        self
    }

    pub fn on_retry(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_retry = Some(Rc::new(handler));
        self
    }

    pub fn on_remove(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_remove = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for GenerationQueue {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let (side, radius) = (theme.progress_ring(), theme.radius(Radius::Md));
        let action = |label: &'static str, key: &SharedString, handler: &Option<OnValue>| {
            handler.clone().map(|handler| {
                let key = key.clone();
                move |_: &gpui::ClickEvent, window: &mut Window, cx: &mut App| {
                    log::info!("generation queue: {label} {key}");
                    handler(&key, window, cx)
                }
            })
        };
        let rows = self.jobs.iter().map(|job| {
            let id = (self.id.clone(), format!("job-{}", job.key));
            let lead = match (&job.state, &job.picture) {
                (JobState::Running(share, _), _) => {
                    ProgressRing::new((self.id.clone(), format!("ring-{}", job.key)), *share)
                        .into_any_element()
                }
                (JobState::Done(_), Some(picture)) => div()
                    .size(side)
                    .child(
                        Image::new(
                            (self.id.clone(), format!("picture-{}", job.key)),
                            source(picture),
                        )
                        .size_full()
                        .rounded(radius),
                    )
                    .into_any_element(),
                (state, _) => {
                    let (icon, ink) = match state {
                        JobState::Queued(_) => (IconName::Clock, colors.fg_subtle),
                        JobState::Failed(_) => (IconName::CircleAlert, colors.danger),
                        _ => (IconName::Check, colors.success),
                    };
                    div()
                        .size(side)
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(radius)
                        .bg(colors.sunken)
                        .child(Icon::new(icon).size(IconSize::Sm).color(ink))
                        .into_any_element()
                }
            };
            let failed = matches!(job.state, JobState::Failed(_));
            let ended = matches!(job.state, JobState::Done(_) | JobState::Failed(_));
            let actions = div()
                .flex_none()
                .flex()
                .items_center()
                .gap_1()
                .when(failed, |row| {
                    row.children(action("retry", &job.key, &self.on_retry).map(|retry| {
                        Button::new((self.id.clone(), format!("retry-{}", job.key)), "Retry")
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Sm)
                            .on_click(retry)
                    }))
                })
                .children(match ended {
                    true => action("remove", &job.key, &self.on_remove).map(|remove| {
                        IconButton::new(
                            (self.id.clone(), format!("remove-{}", job.key)),
                            IconName::X,
                        )
                        .size(ControlSize::Sm)
                        .tooltip("Remove")
                        .on_click(remove)
                    }),
                    false => action("cancel", &job.key, &self.on_cancel).map(|cancel| {
                        IconButton::new(
                            (self.id.clone(), format!("cancel-{}", job.key)),
                            IconName::X,
                        )
                        .size(ControlSize::Sm)
                        .tooltip("Cancel")
                        .on_click(cancel)
                    }),
                });
            let row = div()
                .id(id)
                .w_full()
                .flex()
                .items_center()
                .gap_3()
                .py_2()
                .child(div().flex_none().child(lead))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .child(
                            div()
                                .text_size(theme.text_size(TextSize::Sm))
                                .text_color(colors.fg)
                                .child(Ellipsis::new(job.prompt.clone())),
                        )
                        .child(
                            div()
                                .text_size(theme.text_size(TextSize::Xs))
                                .text_color(if failed {
                                    colors.danger
                                } else {
                                    colors.fg_muted
                                })
                                .child(Ellipsis::new(status(&job.state))),
                        ),
                )
                .child(actions);
            (job.key.clone(), row)
        });
        rows.fold(
            AnimatePresence::new((self.id.clone(), "rows")),
            |list, (key, row)| list.row(key, true, row),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{JobState, status};

    #[test]
    fn a_status_says_where_a_job_stands() {
        assert_eq!(status(&JobState::Queued(0)), "Queued · next");
        assert_eq!(status(&JobState::Queued(2)), "Queued · 2 ahead");
        let left = Some(Duration::from_millis(12_400));
        assert_eq!(
            status(&JobState::Running(0.62, left)),
            "62% · about 12s left"
        );
        assert_eq!(status(&JobState::Running(0.05, None)), "5%");
        assert_eq!(
            status(&JobState::Done(Duration::from_secs(95))),
            "Done in 1m 35s"
        );
        assert_eq!(
            status(&JobState::Failed("out of memory".into())),
            "Failed · out of memory"
        );
    }
}
