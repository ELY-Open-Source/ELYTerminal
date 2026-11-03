use gpui::{
    App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div,
};
use jiff::civil::Date;

use super::tasks::clock_today;
use crate::{
    motion::ProgressBar,
    primitives::Severity,
    theme::{ActiveTheme, TextSize},
    typography::{Ellipsis, fresh, tabular},
};

/// Whether a milestone keeps pace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    OnTrack,
    AtRisk,
    Late,
    Done,
}

impl Standing {
    pub fn words(self) -> &'static str {
        match self {
            Standing::OnTrack => "On track",
            Standing::AtRisk => "At risk",
            Standing::Late => "Late",
            Standing::Done => "Done",
        }
    }

    fn severity(self) -> Severity {
        match self {
            Standing::OnTrack | Standing::Done => Severity::Success,
            Standing::AtRisk => Severity::Warning,
            Standing::Late => Severity::Danger,
        }
    }
}

/// Days from `from` to `to`.
fn days(from: Date, to: Date) -> i64 {
    i64::from(to.since(from).expect("days between two dates").get_days())
}

/// Where `done` of `total` stands on `today`, from `start` to `target`: done when all are, late past the target, at risk when the share done trails the share of time gone by more than a tenth.
pub(crate) fn standing(
    start: Date,
    target: Date,
    today: Date,
    done: usize,
    total: usize,
) -> Standing {
    if done == total {
        return Standing::Done;
    }
    if today > target {
        return Standing::Late;
    }
    let gone = days(start, today).max(0) as f32 / days(start, target).max(1) as f32;
    let share = done as f32 / total as f32;
    match share + 0.1 < gone {
        true => Standing::AtRisk,
        false => Standing::OnTrack,
    }
}

/// When a target falls from `today`, in words: due today, in so many days, or so many days late.
fn due(target: Date, today: Date) -> String {
    match days(today, target) {
        0 => "Due today".into(),
        1 => "Due tomorrow".into(),
        left if left > 1 => format!("{left} days left"),
        -1 => "A day late".into(),
        late => format!("{} days late", -late),
    }
}

/// A milestone on its way: its name, whether it keeps pace, how many of its issues are done as a bar and a count, and when it is due.
#[derive(IntoElement)]
pub struct MilestoneProgress {
    id: ElementId,
    name: SharedString,
    start: Date,
    target: Date,
    done: usize,
    total: usize,
    today: Option<Date>,
}

impl MilestoneProgress {
    /// Work from `start` meant to end by `target`, with `done` of `total` issues done.
    pub fn new(
        id: impl Into<ElementId>,
        name: impl Into<SharedString>,
        (start, target): (Date, Date),
        (done, total): (usize, usize),
    ) -> Self {
        assert!(
            target >= start,
            "a milestone's target comes after its start"
        );
        assert!(total > 0, "{done} of no issues done");
        if done > total {
            log::error!("milestone: {done} of {total} issues done; all count as done");
        }
        Self {
            id: id.into(),
            name: name.into(),
            start,
            target,
            done,
            total,
            today: None,
        }
    }

    /// The day pace is read on; the clock's otherwise.
    pub fn today(mut self, day: Date) -> Self {
        self.today = Some(day);
        self
    }
}

impl RenderOnce for MilestoneProgress {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let today = match self.today {
            Some(day) => day,
            None => {
                fresh((self.id.clone(), "clock"), window, cx);
                clock_today("milestone progress")
            }
        };
        let done = self.done.min(self.total);
        let stands = standing(self.start, self.target, today, done, self.total);
        let theme = cx.theme();
        let colors = &theme.colors;
        let ink = stands.severity().color(colors);
        let quiet = |text: String| {
            tabular(div())
                .flex_none()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(colors.fg_muted)
                .child(text)
        };
        let share = done as f32 / self.total as f32;
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .font_weight(FontWeight::MEDIUM)
                            .child(Ellipsis::new(self.name)),
                    )
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(ink)
                            .child(div().size(theme.status_dot()).rounded_full().bg(ink))
                            .child(stands.words()),
                    ),
            )
            .child(ProgressBar::new((self.id.clone(), "done"), share))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .justify_between()
                    .gap_x_3()
                    .child(quiet(format!("{} of {} issues", self.done, self.total)))
                    .child(quiet(match stands {
                        Standing::Done => format!("Due {}", self.target.strftime("%b %-d")),
                        _ => due(self.target, today),
                    })),
            )
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::{Standing, due, standing};

    #[test]
    fn a_milestone_keeps_pace_trails_or_runs_late() {
        let (start, target) = (date(2026, 9, 1), date(2026, 9, 21));
        let today = date(2026, 9, 11);
        assert_eq!(
            standing(start, target, today, 5, 10),
            Standing::OnTrack,
            "half the time, half the work"
        );
        assert_eq!(
            standing(start, target, today, 3, 10),
            Standing::AtRisk,
            "half the time, three tenths done"
        );
        assert_eq!(
            standing(start, target, today, 9, 20),
            Standing::OnTrack,
            "trailing by less than a tenth"
        );
        assert_eq!(
            standing(start, target, date(2026, 9, 22), 9, 10),
            Standing::Late
        );
        assert_eq!(
            standing(start, target, date(2026, 9, 30), 10, 10),
            Standing::Done
        );
    }

    #[test]
    fn a_target_reads_as_days_left_or_late() {
        let target = date(2026, 9, 21);
        let words = [21, 20, 16, 22, 24].map(|day| due(target, date(2026, 9, day)));
        assert_eq!(
            words,
            [
                "Due today",
                "Due tomorrow",
                "5 days left",
                "A day late",
                "3 days late"
            ]
        );
    }
}
