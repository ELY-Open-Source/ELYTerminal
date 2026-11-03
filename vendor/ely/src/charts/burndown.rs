use gpui::{
    App, Bounds, Div, ElementId, Entity, IntoElement, ParentElement, PathBuilder, Pixels,
    Refineable, RenderOnce, StyleRefinement, Styled, Window, canvas, div, fill, size,
};
use jiff::{ToSpan, civil::Date};

use super::{
    axes::{anchored, below, beside},
    geometry::Rect,
    paint::{at, finish, measure, tint, trace},
    parts::ChartTooltip,
    scale::{Linear, compact, nice},
    schedule::{days, marks},
    series::drawable,
    tiles::{Tiles, tracked},
};
use crate::{
    theme::{ActiveTheme, TextSize},
    typography::tabular,
};

/// Where a burndown's lines and levels fall in its frame.
#[derive(Debug, PartialEq)]
pub(crate) struct Burn {
    pub ideal: [(f32, f32); 2],
    pub left: Vec<(f32, f32)>,
    pub ticks: Vec<(f32, f64)>,
    pub step: f32,
}

/// Lays a sprint of `count` days, `scope` at the start and `left` at the end of each day so far, into `frame`.
pub(crate) fn burn(count: usize, scope: f64, left: &[f64], frame: Rect) -> Burn {
    let high = left.iter().copied().fold(scope, f64::max);
    let ((low, high), ticks) = nice(0.0, high, 4);
    let y = Linear::new((low, high), (frame.y + frame.h, frame.y));
    let step = frame.w / (count - 1) as f32;
    let x = |ix: usize| frame.x + step * ix as f32;
    Burn {
        ideal: [(x(0), y.at(scope)), (x(count - 1), y.at(0.0))],
        left: left
            .iter()
            .enumerate()
            .map(|(ix, value)| (x(ix), y.at(*value)))
            .collect(),
        ticks: ticks.iter().map(|tick| (y.at(*tick), *tick)).collect(),
        step,
    }
}

/// Work left over a sprint, a day at a time: the ideal line from the scope down to nothing on the last day, what was left at each day's end so far, and a line at today. Hover a day to read it.
#[derive(IntoElement)]
pub struct BurndownChart {
    base: Div,
    id: ElementId,
    first: Date,
    last: Date,
    scope: f64,
    left: Vec<f64>,
    today: Option<Date>,
}

impl BurndownChart {
    /// A sprint from its first day through its last, with `scope` to do, in points or issues.
    pub fn new(id: impl Into<ElementId>, first: Date, last: Date, scope: f64) -> Self {
        assert!(last > first, "a sprint ends after its first day");
        assert!(
            drawable(scope) && scope > 0.0,
            "a sprint has work to do, within charts::LIMIT"
        );
        Self {
            base: div(),
            id: id.into(),
            first,
            last,
            scope,
            left: Vec::new(),
            today: None,
        }
    }

    /// What was left at the end of each day from the first, as far as the sprint has gone.
    pub fn left(mut self, left: impl IntoIterator<Item = f64>) -> Self {
        self.left = left.into_iter().collect();
        let count = days(self.first, self.last) as usize + 1;
        assert!(
            self.left.len() <= count,
            "more days left than the sprint has"
        );
        assert!(
            self.left
                .iter()
                .all(|value| drawable(*value) && *value >= 0.0),
            "work left is a count, zero or more, within charts::LIMIT"
        );
        self
    }

    /// Marks this day with a line.
    pub fn today(mut self, today: Date) -> Self {
        self.today = Some(today);
        self
    }
}

impl Styled for BurndownChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for BurndownChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let count = days(self.first, self.last) as usize + 1;
        let tiles: Entity<Tiles> =
            window.use_keyed_state((self.id.clone(), "days"), cx, |_, _| Tiles::default());
        let (bounds, hover) = (tiles.read(cx).bounds, tiles.read(cx).pointed(count));
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let (gutter, foot, inset) = (
            pixels(sizes.gutter),
            pixels(sizes.foot),
            pixels(sizes.inset),
        );
        let frame = Rect {
            x: gutter,
            y: inset,
            w: (f32::from(bounds.size.width) - gutter - inset).max(0.0),
            h: (pixels(sizes.height) - inset - foot).max(0.0),
        };
        let burn = burn(count, self.scope, &self.left, frame);
        let label = |text: String| {
            tabular(div())
                .whitespace_nowrap()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(colors.fg_subtle)
                .child(text)
        };
        let levels: Vec<Div> = burn
            .ticks
            .iter()
            .map(|(y, value)| beside(*y, sizes.gutter.to_pixels(rem), label(compact(*value))))
            .collect();
        let dates: Vec<Div> = marks(self.first, count as i32)
            .into_iter()
            .map(|(offset, text)| below(frame.x + burn.step * offset as f32, frame, label(text)))
            .collect();
        let x_of = |ix: usize| frame.x + burn.step * ix as f32;
        let today = self
            .today
            .map(|today| days(self.first, today))
            .filter(|offset| (0..count as i32).contains(offset))
            .map(|offset| x_of(offset as usize));
        let tooltip = hover.map(|ix| {
            let day = self
                .first
                .checked_add((ix as i64).days())
                .expect("a day of the sprint");
            let ideal = self.scope * (1.0 - ix as f64 / (count - 1) as f64);
            let mut card = ChartTooltip::new(day.strftime("%a %b %-d").to_string());
            if let Some(left) = self.left.get(ix) {
                card = card.row(Some(tint(&colors, 0)), "Left", compact(*left));
            }
            card = card.row(None, "Ideal", compact(ideal));
            let y = burn
                .left
                .get(ix)
                .map_or(frame.y + frame.h / 2.0, |point| point.1);
            anchored((x_of(ix), y), x_of(ix) > frame.x + frame.w * 0.6, card)
        });
        let (stroke, hairline, palette) =
            (sizes.stroke.to_pixels(rem), sizes.hairline, colors.clone());
        let points = burn.left.clone();
        let (ideal, levels_at) = (burn.ideal, burn.ticks.clone());
        let pointed = hover.map(x_of);
        let (step, first_x) = (burn.step, frame.x);
        let mut root = div().relative().h(sizes.height);
        root.style().refine(self.base.style());
        tracked(
            root,
            (self.id.clone(), "burndown").into(),
            &tiles,
            move |(x, _)| {
                let ix = ((x - first_x) / step).round();
                (ix >= 0.0 && (ix as usize) < count).then_some(ix as usize)
            },
        )
        .child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    let origin = bounds.origin;
                    for (y, _) in &levels_at {
                        window.paint_quad(fill(
                            Bounds::new(
                                at(origin, (frame.x, *y)),
                                size(Pixels::from(frame.w), hairline),
                            ),
                            palette.border.opacity(0.5),
                        ));
                    }
                    for x in pointed.iter().chain(today.iter()) {
                        let ink = match Some(*x) == today {
                            true => tint(&palette, 3),
                            false => palette.border_strong,
                        };
                        window.paint_quad(fill(
                            Bounds::new(
                                at(origin, (*x, frame.y)),
                                size(stroke / 2.0, Pixels::from(frame.h)),
                            ),
                            ink,
                        ));
                    }
                    let mut line = PathBuilder::stroke(stroke / 2.0);
                    trace(&mut line, &ideal, false, origin);
                    finish(line, palette.fg_subtle, window);
                    if points.len() > 1 {
                        let mut line = PathBuilder::stroke(stroke);
                        trace(&mut line, &points, false, origin);
                        finish(line, tint(&palette, 0), window);
                    }
                },
            )
            .absolute()
            .inset_0(),
        )
        .children(levels)
        .children(dates)
        .children(tooltip)
        .child(measure(tiles, |tiles| &mut tiles.bounds))
    }
}

#[cfg(test)]
mod tests {
    use super::{Rect, burn};

    #[test]
    fn the_ideal_runs_from_the_scope_to_nothing_and_the_days_step_evenly() {
        let frame = Rect {
            x: 40.0,
            y: 0.0,
            w: 90.0,
            h: 100.0,
        };
        let burn = burn(10, 20.0, &[20.0, 18.0, 18.0, 11.0], frame);
        assert_eq!(burn.step, 10.0);
        assert_eq!(burn.ideal, [(40.0, 0.0), (130.0, 100.0)]);
        assert_eq!(
            burn.left.iter().map(|point| point.0).collect::<Vec<_>>(),
            [40.0, 50.0, 60.0, 70.0]
        );
        assert_eq!(
            burn.left[3].1, 45.0,
            "11 of 20 left sits at 55 from the floor"
        );
    }

    #[test]
    fn work_that_grew_past_the_scope_raises_the_top() {
        let frame = Rect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 100.0,
        };
        let burn = burn(2, 20.0, &[26.0], frame);
        let top = burn.ticks.last().expect("levels").1;
        assert!(top >= 26.0, "the scale reaches {top}");
    }
}
