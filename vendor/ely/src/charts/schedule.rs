use gpui::{
    App, Bounds, Div, ElementId, Entity, InteractiveElement, IntoElement, ParentElement,
    PathBuilder, Pixels, Refineable, RenderOnce, SharedString, StyleRefinement, Styled, Window,
    canvas, div, fill, size,
};
use jiff::{
    Span,
    civil::{Date, Weekday},
};

use super::{
    axes::anchored,
    geometry::Rect,
    paint::{at, finish, measure, place, tint},
    parts::ChartTooltip,
    tiles::{Tiles, tracked},
};
use crate::{
    theme::{ActiveTheme, Density, Radius, TextSize},
    typography::format,
};

/// A task on a schedule: its name, first and last day, how much is done, and the tasks it waits for.
#[derive(Clone, Debug, PartialEq)]
pub struct Task {
    name: SharedString,
    start: Date,
    end: Date,
    done: f32,
    after: Vec<usize>,
}

impl Task {
    /// A task from its first day through its last.
    pub fn new(name: impl Into<SharedString>, start: Date, end: Date) -> Self {
        assert!(end >= start, "a task ends on or after its first day");
        Self {
            name: name.into(),
            start,
            end,
            done: 0.0,
            after: Vec::new(),
        }
    }

    /// How much is done, zero to one.
    pub fn done(mut self, share: f32) -> Self {
        assert!((0.0..=1.0).contains(&share), "done is a share, zero to one");
        self.done = share;
        self
    }

    /// Waits for the task at `ix`; an arrow runs from its end.
    pub fn after(mut self, ix: usize) -> Self {
        self.after.push(ix);
        self
    }
}

/// Whole days from `from` to `to`.
pub(super) fn days(from: Date, to: Date) -> i32 {
    to.since(from)
        .expect("dates within jiff's range")
        .get_days()
}

/// Days to mark along a span: each month's first day when the span is long, each Monday when it is short.
pub(super) fn marks(first: Date, count: i32) -> Vec<(i32, String)> {
    let long = count > 62;
    (0..count)
        .filter_map(|offset| {
            let day = first
                .checked_add(Span::new().days(offset))
                .expect("a day within the schedule");
            let marked = if long {
                day.day() == 1
            } else {
                day.weekday() == Weekday::Monday
            };
            marked.then(|| {
                (
                    offset,
                    day.strftime(if long { "%b" } else { "%b %-d" }).to_string(),
                )
            })
        })
        .collect()
}

/// Tasks on a timeline, a row each: bars from first day to last, filled as far as they are done, with arrows from the tasks they wait for and a line at today. Hover a row to read it.
#[derive(IntoElement)]
pub struct GanttChart {
    base: Div,
    id: ElementId,
    tasks: Vec<Task>,
    today: Option<Date>,
}

impl GanttChart {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            tasks: Vec::new(),
            today: None,
        }
    }

    /// A task; the ones it waits for come before it.
    pub fn task(mut self, mut task: Task) -> Self {
        let known = self.tasks.len();
        if task.after.iter().any(|ix| *ix >= known) {
            log::error!(
                "gantt: task {} waits for one not given; that wait is dropped",
                task.name
            );
            task.after.retain(|ix| *ix < known);
        }
        self.tasks.push(task);
        self
    }

    /// Marks this day with a line.
    pub fn today(mut self, today: Date) -> Self {
        self.today = Some(today);
        self
    }
}

impl Styled for GanttChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for GanttChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tiles: Entity<Tiles> =
            window.use_keyed_state((self.id.clone(), "rows"), cx, |_, _| Tiles::default());
        let (bounds, hover) = (
            tiles.read(cx).bounds,
            tiles.read(cx).pointed(self.tasks.len()),
        );
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let count = self.tasks.len();
        let first = self.tasks.iter().map(|task| task.start).min();
        let last = self.tasks.iter().map(|task| task.end).max();
        let span = first
            .zip(last)
            .map_or(1, |(first, last)| days(first, last) + 1);
        let (names, axis, row, inset) = (
            pixels(theme.label_width()),
            pixels(sizes.foot),
            pixels(theme.table_row(Density::Compact)),
            pixels(sizes.inset),
        );
        let day = (f32::from(bounds.size.width) - names - inset).max(0.0) / span as f32;
        let x_of = move |offset: i32| names + day * offset as f32;
        let bars: Vec<Rect> = self
            .tasks
            .iter()
            .enumerate()
            .map(|(ix, task)| {
                let from = first.map_or(0, |first| days(first, task.start));
                Rect {
                    x: x_of(from),
                    y: axis + row * ix as f32 + row * 0.22,
                    w: day * (days(task.start, task.end) + 1) as f32,
                    h: row * 0.56,
                }
            })
            .collect();
        let text = |content: String, color| {
            div()
                .flex_none()
                .whitespace_nowrap()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(color)
                .child(content)
        };
        let ticks = first.map_or_else(Vec::new, |first| marks(first, span));
        let tick_labels: Vec<Div> = ticks
            .iter()
            .map(|(offset, label)| {
                div()
                    .absolute()
                    .left(Pixels::from(x_of(*offset)))
                    .top_0()
                    .h(Pixels::from(axis))
                    .flex()
                    .items_center()
                    .pl_1()
                    .child(text(label.clone(), colors.fg_subtle))
            })
            .collect();
        let name_labels: Vec<Div> = self
            .tasks
            .iter()
            .enumerate()
            .map(|(ix, task)| {
                let label = div()
                    .truncate()
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(if hover == Some(ix) {
                        colors.fg
                    } else {
                        colors.fg_muted
                    })
                    .child(task.name.clone());
                div()
                    .absolute()
                    .left_0()
                    .top(Pixels::from(axis + row * ix as f32))
                    .w(Pixels::from(names))
                    .h(Pixels::from(row))
                    .flex()
                    .items_center()
                    .pr_3()
                    .child(label)
            })
            .collect();
        let today = self
            .today
            .zip(first)
            .map(|(today, first)| x_of(days(first, today)) + day / 2.0)
            .filter(|x| (names..=names + day * span as f32).contains(x));
        let tooltip = hover.map(|ix| {
            let (task, bar) = (&self.tasks[ix], bars[ix]);
            let dates = format!(
                "{} – {}",
                task.start.strftime("%b %-d"),
                task.end.strftime("%b %-d")
            );
            let card = ChartTooltip::new(task.name.clone())
                .row(Some(tint(&colors, 0)), "Dates", dates)
                .row(None, "Days", (days(task.start, task.end) + 1).to_string())
                .row(
                    None,
                    "Done",
                    format::percent(f64::from(task.done), 0, false),
                );
            let x = (bar.x + bar.w).min(f32::from(bounds.size.width));
            anchored(
                (x, bar.y + bar.h / 2.0),
                x > f32::from(bounds.size.width) * 0.6,
                card,
            )
        });
        let links: Vec<(Rect, Rect)> = self
            .tasks
            .iter()
            .enumerate()
            .flat_map(|(ix, task)| task.after.iter().map(move |before| (*before, ix)))
            .map(|(before, ix)| (bars[before], bars[ix]))
            .collect();
        let done: Vec<f32> = self.tasks.iter().map(|task| task.done).collect();
        let (stroke, hairline, corner, palette) = (
            sizes.stroke.to_pixels(rem),
            sizes.hairline,
            theme.radius(Radius::Sm).to_pixels(rem),
            colors.clone(),
        );
        let (width, grid) = (
            f32::from(bounds.size.width),
            ticks
                .iter()
                .map(|(offset, _)| x_of(*offset))
                .collect::<Vec<_>>(),
        );
        let painted = bars.clone();
        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.foot + theme.table_row(Density::Compact) * count as f32);
        root.style().refine(self.base.style());
        tracked(
            root,
            (self.id.clone(), "gantt").into(),
            &tiles,
            move |(_, y)| {
                let at = ((y - axis) / row).floor();
                (at >= 0.0 && (at as usize) < count).then_some(at as usize)
            },
        )
        .child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    let origin = bounds.origin;
                    let bottom = axis + row * count as f32;
                    if let Some(ix) = hover {
                        window.paint_quad(fill(
                            place(
                                origin,
                                Rect {
                                    x: 0.0,
                                    y: axis + row * ix as f32,
                                    w: width,
                                    h: row,
                                },
                            ),
                            palette.hover,
                        ));
                    }
                    for x in &grid {
                        window.paint_quad(fill(
                            Bounds::new(
                                at(origin, (*x, axis)),
                                size(hairline, Pixels::from(bottom - axis)),
                            ),
                            palette.border.opacity(0.5),
                        ));
                    }
                    window.paint_quad(fill(
                        Bounds::new(
                            at(origin, (names, axis)),
                            size(Pixels::from(width - names), hairline),
                        ),
                        palette.border,
                    ));
                    let ink = tint(&palette, 0);
                    for (bar, share) in painted.iter().zip(&done) {
                        window.paint_quad(
                            fill(place(origin, *bar), ink.opacity(0.22)).corner_radii(corner),
                        );
                        window.paint_quad(
                            fill(
                                place(
                                    origin,
                                    Rect {
                                        w: bar.w * share,
                                        ..*bar
                                    },
                                ),
                                ink,
                            )
                            .corner_radii(corner),
                        );
                    }
                    let tip = f32::from(stroke) * 2.0;
                    for (from, to) in &links {
                        let (start, end) = (
                            (from.x + from.w, from.y + from.h / 2.0),
                            (to.x, to.y + to.h / 2.0),
                        );
                        let turn = start.0 + tip * 2.0;
                        let mut path = PathBuilder::stroke(hairline);
                        path.move_to(at(origin, start));
                        if turn + tip <= end.0 {
                            path.line_to(at(origin, (turn, start.1)));
                            path.line_to(at(origin, (turn, end.1)));
                        } else {
                            let between = from.y + from.h + (to.y - from.y - from.h) / 2.0;
                            path.line_to(at(origin, (turn, start.1)));
                            path.line_to(at(origin, (turn, between)));
                            path.line_to(at(origin, (end.0 - tip * 2.0, between)));
                            path.line_to(at(origin, (end.0 - tip * 2.0, end.1)));
                        }
                        path.line_to(at(origin, (end.0 - tip, end.1)));
                        finish(path, palette.fg_subtle, window);
                        let mut head = PathBuilder::fill();
                        head.move_to(at(origin, end));
                        head.line_to(at(origin, (end.0 - tip, end.1 - tip / 1.5)));
                        head.line_to(at(origin, (end.0 - tip, end.1 + tip / 1.5)));
                        head.close();
                        finish(head, palette.fg_subtle, window);
                    }
                    if let Some(x) = today {
                        window.paint_quad(fill(
                            Bounds::new(
                                at(origin, (x, axis)),
                                size(stroke / 2.0, Pixels::from(bottom - axis)),
                            ),
                            tint(&palette, 3),
                        ));
                    }
                },
            )
            .absolute()
            .inset_0(),
        )
        .children(tick_labels)
        .children(name_labels)
        .children(tooltip)
        .child(measure(tiles, |tiles| &mut tiles.bounds))
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::{days, marks};

    #[test]
    fn short_spans_mark_mondays_and_long_ones_months() {
        assert_eq!(days(date(2026, 3, 1), date(2026, 3, 15)), 14);
        let weekly = marks(date(2026, 3, 1), 14);
        assert_eq!(
            weekly.iter().map(|mark| mark.0).collect::<Vec<_>>(),
            [1, 8],
            "the first of March 2026 is a Sunday"
        );
        let monthly = marks(date(2026, 1, 15), 90);
        assert_eq!(
            monthly
                .iter()
                .map(|mark| mark.1.as_str())
                .collect::<Vec<_>>(),
            ["Feb", "Mar", "Apr"]
        );
    }
}
