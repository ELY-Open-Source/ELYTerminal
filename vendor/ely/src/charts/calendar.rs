use std::rc::Rc;

use gpui::{
    App, Div, ElementId, Entity, Hsla, InteractiveElement, IntoElement, ParentElement, Pixels,
    Refineable, RenderOnce, StyleRefinement, Styled, Window, canvas, div, fill,
};
use jiff::{
    Span,
    civil::{Date, date},
};

use super::{
    axes::{anchored, beside},
    geometry::Rect,
    layout::{calendar, level},
    paint::{measure, place, tint},
    parts::ChartTooltip,
    plot::Format,
    scale::compact,
    series::drawable,
    tiles::{Tiles, key, tracked},
};
use crate::theme::{ActiveTheme, Radius, TextSize};

/// A year of daily counts as a grid of weeks, Monday first, each day as strong as its count on a scale of five. Hover reads a day.
#[derive(IntoElement)]
pub struct CalendarHeatmap {
    base: Div,
    id: ElementId,
    start: Date,
    counts: Vec<f64>,
    format: Format,
}

impl CalendarHeatmap {
    /// A count for each day from `start` on.
    pub fn new(
        id: impl Into<ElementId>,
        start: Date,
        counts: impl IntoIterator<Item = f64>,
    ) -> Self {
        let counts: Vec<f64> = counts.into_iter().collect();
        assert!(
            counts.iter().all(|count| drawable(*count) && *count >= 0.0),
            "a calendar needs counts of zero or more, within charts::LIMIT"
        );
        let span = Span::new().days(counts.len().saturating_sub(1) as i64);
        assert!(
            start.checked_add(span).is_ok(),
            "a calendar's last day lies past the dates jiff holds"
        );
        Self {
            base: div(),
            id: id.into(),
            start,
            counts,
            format: Rc::new(compact),
        }
    }

    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }
}

impl Styled for CalendarHeatmap {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for CalendarHeatmap {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tiles: Entity<Tiles> =
            window.use_keyed_state((self.id.clone(), "tiles"), cx, |_, _| Tiles::default());
        let (bounds, hover) = (
            tiles.read(cx).bounds,
            tiles.read(cx).pointed(self.counts.len()),
        );
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let first = self.start.weekday().to_monday_zero_offset() as usize;
        let (places, weeks) = calendar(first, self.counts.len());
        let (gutter, top) = (pixels(sizes.gutter), pixels(sizes.foot) * 0.75);
        let side = ((f32::from(bounds.size.width) - gutter) / weeks.max(1) as f32)
            .min((f32::from(bounds.size.height) - top) / 7.0)
            .max(0.0);
        let day = |ix: usize| {
            self.start
                .checked_add(Span::new().days(ix as i64))
                .expect("a day within the calendar")
        };
        let square = |(week, weekday): (usize, usize)| Rect {
            x: gutter + side * week as f32,
            y: top + side * weekday as f32,
            w: side,
            h: side,
        };
        let highest = self.counts.iter().copied().fold(0.0, f64::max);
        let ink = tint(&colors, 0);
        let shades = [
            colors.border.opacity(0.5),
            ink.opacity(0.3),
            ink.opacity(0.52),
            ink.opacity(0.76),
            ink,
        ];
        let text = |content: String| {
            div()
                .flex_none()
                .whitespace_nowrap()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(colors.fg_subtle)
                .child(content)
        };
        let months = (0..self.counts.len())
            .filter(|ix| *ix == 0 || day(*ix).day() == 1)
            .map(|ix| {
                let at = square(places[ix]);
                div()
                    .absolute()
                    .left(Pixels::from(at.x))
                    .top_0()
                    .child(text(day(ix).strftime("%b").to_string()))
            });
        let weekdays = [0usize, 2, 4].map(|weekday| {
            let date = date(2024, 1, 1 + weekday as i8);
            beside(
                top + side * (weekday as f32 + 0.5),
                sizes.gutter.to_pixels(rem),
                text(date.strftime("%a").to_string()),
            )
        });
        let format = self.format.clone();
        let tooltip = hover.map(|ix| {
            let at = square(places[ix]);
            let card = ChartTooltip::new(day(ix).strftime("%a %b %-d, %Y").to_string()).row(
                Some(shades[level(self.counts[ix], highest)]),
                "Count",
                format(self.counts[ix]),
            );
            anchored(
                (at.x + at.w, at.y + at.h / 2.0),
                at.x > f32::from(bounds.size.width) * 0.6,
                card,
            )
        });
        let days: Vec<(Rect, Hsla)> = self
            .counts
            .iter()
            .enumerate()
            .map(|(ix, count)| (square(places[ix]), shades[level(*count, highest)]))
            .collect();
        let (gap, corner, edge, hairline) = (
            pixels(sizes.stroke) / 2.0,
            Pixels::from(pixels(theme.radius(Radius::Sm)).min(side * 0.2)),
            colors.fg,
            sizes.hairline,
        );
        let hit = days.iter().map(|(rect, _)| *rect).collect::<Vec<_>>();
        let legend = key(shades, ("Less", "More"), cx);
        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .h(sizes.height * 0.5);
        root.style().refine(self.base.style());
        let chart = tracked(
            div().relative().flex_1().min_h_0(),
            (self.id.clone(), "calendar").into(),
            &tiles,
            move |at| hit.iter().position(|rect| rect.contains(at)),
        )
        .child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    for (ix, (square, color)) in days.iter().enumerate() {
                        let inner = Rect {
                            x: square.x + gap,
                            y: square.y + gap,
                            w: (square.w - gap * 2.0).max(0.0),
                            h: (square.h - gap * 2.0).max(0.0),
                        };
                        let quad = fill(place(bounds.origin, inner), *color).corner_radii(corner);
                        window.paint_quad(if hover == Some(ix) {
                            quad.border_widths(hairline).border_color(edge)
                        } else {
                            quad
                        });
                    }
                },
            )
            .absolute()
            .inset_0(),
        )
        .children(months)
        .children(weekdays)
        .children(tooltip)
        .child(measure(tiles, |tiles| &mut tiles.bounds));
        root.flex().flex_col().gap_2().child(chart).child(legend)
    }
}
