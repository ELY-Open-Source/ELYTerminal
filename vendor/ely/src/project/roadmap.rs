use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div, relative,
};
use jiff::{ToSpan, civil::Date};

use crate::{
    layout::on_axis,
    primitives::Tooltip,
    theme::{ActiveTheme, Density, Radius, TextSize},
    typography::{Ellipsis, format},
};

/// An initiative on a roadmap: its key, its name, its first and last day, and how far along it is.
#[derive(Clone, Debug, PartialEq)]
pub struct Initiative {
    pub key: SharedString,
    pub name: SharedString,
    pub start: Date,
    pub end: Date,
    pub progress: f32,
}

impl Initiative {
    pub fn new(
        key: impl Into<SharedString>,
        name: impl Into<SharedString>,
        start: Date,
        end: Date,
    ) -> Self {
        let name = name.into();
        assert!(end >= start, "initiative {name} ends before it starts");
        Self {
            key: key.into(),
            name,
            start,
            end,
            progress: 0.0,
        }
    }

    /// How far along it is, zero to one.
    pub fn progress(mut self, share: f32) -> Self {
        assert!(
            (0.0..=1.0).contains(&share),
            "progress is a share, zero to one"
        );
        self.progress = share;
        self
    }
}

/// Where `day` falls along the span from `first`, `span` days long, as a share clamped to the span.
pub(crate) fn along(first: Date, span: i32, day: Date) -> f32 {
    let offset = day.since(first).expect("days between two dates").get_days();
    (offset as f32 / span as f32).clamp(0.0, 1.0)
}

/// Initiatives across months in lanes, as teams or themes: each a bar from its first day through its last, filled as far as it has come, under the months' names, with a line at today. Hover a bar to read its dates; too narrow for its months, it scrolls sideways.
#[derive(IntoElement)]
pub struct Roadmap {
    id: ElementId,
    first: Date,
    months: i8,
    lanes: Vec<(SharedString, Vec<Initiative>)>,
    today: Option<Date>,
}

impl Roadmap {
    /// `months` months from the one `first` falls in.
    pub fn new(id: impl Into<ElementId>, first: Date, months: i8) -> Self {
        assert!(
            (1..=24).contains(&months),
            "a roadmap spans one to twenty-four months"
        );
        Self {
            id: id.into(),
            first: first.first_of_month(),
            months,
            lanes: Vec::new(),
            today: None,
        }
    }

    /// A lane of initiatives under its name.
    pub fn lane(
        mut self,
        name: impl Into<SharedString>,
        initiatives: impl IntoIterator<Item = Initiative>,
    ) -> Self {
        self.lanes
            .push((name.into(), initiatives.into_iter().collect()));
        self
    }

    /// Marks this day with a line.
    pub fn today(mut self, today: Date) -> Self {
        self.today = Some(today);
        self
    }
}

impl RenderOnce for Roadmap {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (last, span) = match self.first.checked_add((self.months as i64).months()) {
            Ok(end) => (
                end.yesterday().expect("a day before a later one"),
                end.since(self.first)
                    .expect("days of the roadmap")
                    .get_days(),
            ),
            Err(error) => {
                log::error!(
                    "roadmap {:?}: its months run past jiff's last day: {error}",
                    self.id
                );
                let days = Date::MAX.since(self.first).expect("days of the roadmap");
                (Date::MAX, days.get_days() + 1)
            }
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let row = theme.table_row(Density::Compact);
        let months: Vec<(f32, Date)> = (0..self.months)
            .filter_map(|month| self.first.checked_add((month as i64).months()).ok())
            .map(|day| (along(self.first, span, day), day))
            .collect();
        let head = div()
            .relative()
            .h(row)
            .children(months.iter().map(|(x, day)| {
                let name = match day.month() == 1 || *day == self.first {
                    true => day.strftime("%b %Y").to_string(),
                    false => day.strftime("%b").to_string(),
                };
                div()
                    .absolute()
                    .left(relative(*x))
                    .top_0()
                    .h_full()
                    .flex()
                    .items_center()
                    .pl_1p5()
                    .whitespace_nowrap()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.fg_subtle)
                    .child(name)
            }));
        let rules = months.iter().map(|(x, _)| {
            div()
                .absolute()
                .left(relative(*x))
                .top_0()
                .bottom_0()
                .border_l_1()
                .border_color(colors.border)
        });
        let today = self
            .today
            .filter(|today| (self.first..=last).contains(today))
            .map(|today| {
                div()
                    .absolute()
                    .left(relative(along(self.first, span, today)))
                    .top_0()
                    .bottom_0()
                    .border_l_1()
                    .border_color(colors.hue(3, "roadmap today"))
            });
        let radius = theme.radius(Radius::Md);
        let lanes = self
            .lanes
            .into_iter()
            .enumerate()
            .map(|(lane_ix, (name, initiatives))| {
                let hue = colors.hue(lane_ix % colors.chart.len(), format_args!("lane {name}"));
                let bars = initiatives
                    .into_iter()
                    .filter(|each| each.end >= self.first && each.start <= last)
                    .map(|each| {
                        let (from, to) = (
                            along(self.first, span, each.start),
                            each.end
                                .tomorrow()
                                .map_or(1.0, |after| along(self.first, span, after)),
                        );
                        let dates = format!(
                            "{}: {} – {}, {} done",
                            each.name,
                            each.start.strftime("%b %-d"),
                            each.end.strftime("%b %-d"),
                            format::percent(f64::from(each.progress), 0, false)
                        );
                        let named = each.key.clone();
                        div().relative().h(row).child(
                            div()
                                .id((self.id.clone(), format!("initiative-{}", each.key)))
                                .debug_selector(move || format!("initiative {named}"))
                                .absolute()
                                .left(relative(from))
                                .w(relative(to - from))
                                .top_1()
                                .bottom_1()
                                .rounded(radius)
                                .bg(hue.opacity(0.16))
                                .tooltip(Tooltip::text(dates))
                                .child(
                                    div()
                                        .absolute()
                                        .left_0()
                                        .top_0()
                                        .bottom_0()
                                        .w(relative(each.progress))
                                        .rounded(radius)
                                        .bg(hue.opacity(0.32)),
                                )
                                .child(
                                    div()
                                        .relative()
                                        .h_full()
                                        .px_2()
                                        .flex()
                                        .items_center()
                                        .text_size(theme.text_size(TextSize::Sm))
                                        .text_color(colors.fg)
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .child(Ellipsis::new(each.name)),
                                        ),
                                ),
                        )
                    });
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(row)
                            .flex()
                            .items_center()
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(colors.fg_muted)
                            .child(name),
                    )
                    .children(bars.collect::<Vec<_>>())
            });
        let least = theme.project().month * self.months as f32;
        on_axis(
            div()
                .id((self.id.clone(), "scroll"))
                .debug_selector(|| "roadmap".into())
                .overflow_x_scroll(),
        )
        .child(
            div().min_w(least).flex().flex_col().child(head).child(
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .children(rules)
                    .children(lanes.collect::<Vec<_>>())
                    .children(today),
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::along;

    #[test]
    fn days_fall_along_the_span_and_stay_inside_it() {
        let first = date(2026, 9, 1);
        assert_eq!(along(first, 30, date(2026, 9, 16)), 0.5);
        assert_eq!(along(first, 30, date(2026, 8, 20)), 0.0);
        assert_eq!(along(first, 30, date(2026, 12, 1)), 1.0);
    }
}
