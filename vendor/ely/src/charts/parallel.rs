use std::rc::Rc;

use gpui::{
    App, Div, ElementId, Entity, InteractiveElement, IntoElement, ParentElement, PathBuilder,
    Pixels, Refineable, RenderOnce, SharedString, StyleRefinement, Styled, Window, canvas, div,
};

use super::{
    axes::anchored,
    geometry::Rect,
    paint::{at, finish, measure, tint},
    parts::ChartTooltip,
    plot::Format,
    scale::{Linear, compact, nice},
    series::drawable,
    tiles::{Tiles, tracked},
};
use crate::theme::{ActiveTheme, TextSize};

/// Parallel axes: where each stands, its scale, and each record's line across them.
pub(crate) struct Parallel {
    pub xs: Vec<f32>,
    pub scales: Vec<Linear>,
    pub lines: Vec<Vec<(f32, f32)>>,
}

/// Records as lines across parallel axes, one axis per measure, each with its own rounded scale.
pub(crate) fn parallel(records: &[Vec<f64>], axes: usize, frame: Rect) -> Parallel {
    let xs: Vec<f32> = (0..axes)
        .map(|ix| frame.x + frame.w * ix as f32 / (axes - 1).max(1) as f32)
        .collect();
    let scales: Vec<Linear> = (0..axes)
        .map(|axis| {
            let (low, high) = records
                .iter()
                .map(|record| record[axis])
                .fold((f64::MAX, f64::MIN), |(low, high), value| {
                    (low.min(value), high.max(value))
                });
            let (domain, _) = if low <= high {
                nice(low, high, 4)
            } else {
                ((0.0, 1.0), Vec::new())
            };
            Linear::new(domain, (frame.y + frame.h, frame.y))
        })
        .collect();
    let lines = records
        .iter()
        .map(|record| {
            record
                .iter()
                .enumerate()
                .map(|(axis, value)| (xs[axis], scales[axis].at(*value)))
                .collect()
        })
        .collect();
    Parallel { xs, scales, lines }
}

/// How far a place lies from a line through points.
fn distance((x, y): (f32, f32), line: &[(f32, f32)]) -> f32 {
    line.windows(2)
        .map(|pair| {
            let ((ax, ay), (bx, by)) = (pair[0], pair[1]);
            let (dx, dy) = (bx - ax, by - ay);
            let share = (((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy).max(f32::EPSILON))
                .clamp(0.0, 1.0);
            (x - ax - dx * share).hypot(y - ay - dy * share)
        })
        .fold(f32::MAX, f32::min)
}

/// Records with many measures as lines across parallel axes, each axis on its own scale. Hover a line to read its record.
#[derive(IntoElement)]
pub struct ParallelCoordinates {
    base: Div,
    id: ElementId,
    axes: Vec<SharedString>,
    records: Vec<(SharedString, Vec<f64>)>,
    format: Format,
}

impl ParallelCoordinates {
    pub fn new(
        id: impl Into<ElementId>,
        axes: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        let axes: Vec<SharedString> = axes.into_iter().map(Into::into).collect();
        assert!(axes.len() >= 2, "parallel coordinates need two axes");
        Self {
            base: div(),
            id: id.into(),
            axes,
            records: Vec::new(),
            format: Rc::new(compact),
        }
    }

    /// A record, a value for each axis.
    pub fn record(
        mut self,
        name: impl Into<SharedString>,
        values: impl IntoIterator<Item = f64>,
    ) -> Self {
        let values: Vec<f64> = values.into_iter().collect();
        assert_eq!(
            values.len(),
            self.axes.len(),
            "a record needs a value per axis"
        );
        assert!(
            values.iter().all(|value| drawable(*value)),
            "a record needs values, within charts::LIMIT"
        );
        self.records.push((name.into(), values));
        self
    }

    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }
}

impl Styled for ParallelCoordinates {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for ParallelCoordinates {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tiles: Entity<Tiles> =
            window.use_keyed_state((self.id.clone(), "lines"), cx, |_, _| Tiles::default());
        let (bounds, hover) = (
            tiles.read(cx).bounds,
            tiles.read(cx).pointed(self.records.len()),
        );
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let (side, top) = (pixels(sizes.gutter), pixels(sizes.foot));
        let (width, height) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let frame = Rect {
            x: side,
            y: top,
            w: (width - side * 2.0).max(0.0),
            h: (height - top * 2.0).max(0.0),
        };
        let values: Vec<Vec<f64>> = self
            .records
            .iter()
            .map(|(_, values)| values.clone())
            .collect();
        let Parallel { xs, scales, lines } = parallel(&values, self.axes.len(), frame);
        let format = self.format.clone();
        let text = |content: String, color| {
            div()
                .flex_none()
                .whitespace_nowrap()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(color)
                .child(content)
        };
        let axis_labels: Vec<Div> = xs
            .iter()
            .zip(&self.axes)
            .map(|(x, name)| {
                div()
                    .absolute()
                    .left(Pixels::from(*x))
                    .top_0()
                    .w_0()
                    .h(Pixels::from(top))
                    .flex()
                    .justify_center()
                    .items_center()
                    .child(text(name.to_string(), colors.fg_muted))
            })
            .collect();
        let ends: Vec<Div> = xs
            .iter()
            .zip(&scales)
            .flat_map(|(x, scale)| {
                [
                    (scale.domain.1, frame.y),
                    (scale.domain.0, frame.y + frame.h),
                ]
                .map(|(value, y)| (*x, value, y))
            })
            .map(|(x, value, y)| {
                div()
                    .absolute()
                    .left(Pixels::from(x))
                    .top(Pixels::from(y))
                    .h_0()
                    .flex()
                    .items_center()
                    .pl_1p5()
                    .child(text(format(value), colors.fg_subtle))
            })
            .collect();
        let tooltip = hover.map(|ix| {
            let card = self.axes.iter().zip(&self.records[ix].1).fold(
                ChartTooltip::new(self.records[ix].0.clone()),
                |card, (axis, value)| card.row(None, axis.clone(), format(*value)),
            );
            let (x, y) = lines[ix][lines[ix].len() / 2];
            anchored((x, y), x > width * 0.6, card)
        });
        let (stroke, hairline, palette, painted) = (
            sizes.stroke.to_pixels(rem),
            sizes.hairline,
            colors.clone(),
            lines.clone(),
        );
        let reach = pixels(sizes.inset);
        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());
        tracked(
            root,
            (self.id.clone(), "parallel").into(),
            &tiles,
            move |place| {
                lines
                    .iter()
                    .enumerate()
                    .map(|(ix, line)| (ix, distance(place, line)))
                    .filter(|(_, far)| *far <= reach)
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|(ix, _)| ix)
            },
        )
        .child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    let origin = bounds.origin;
                    for x in &xs {
                        let mut axis = PathBuilder::stroke(hairline);
                        axis.move_to(at(origin, (*x, frame.y)));
                        axis.line_to(at(origin, (*x, frame.y + frame.h)));
                        finish(axis, palette.border_strong, window);
                    }
                    let order = (0..painted.len())
                        .filter(|ix| Some(*ix) != hover)
                        .chain(hover);
                    for ix in order {
                        let ink = tint(&palette, ix);
                        let (color, width) = match hover {
                            None => (ink.opacity(0.7), stroke / 1.5),
                            Some(on) if on == ix => (ink, stroke),
                            Some(_) => (ink.opacity(0.15), stroke / 1.5),
                        };
                        let mut path = PathBuilder::stroke(width);
                        path.move_to(at(origin, painted[ix][0]));
                        painted[ix][1..]
                            .iter()
                            .for_each(|point| path.line_to(at(origin, *point)));
                        finish(path, color, window);
                    }
                },
            )
            .absolute()
            .inset_0(),
        )
        .children(axis_labels)
        .children(ends)
        .children(tooltip)
        .child(measure(tiles, |tiles| &mut tiles.bounds))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallel_axes_share_the_frame_and_lines_meet_their_values() {
        let frame = Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        };
        let Parallel { xs, scales, lines } =
            parallel(&[vec![0.0, 10.0], vec![100.0, 0.0]], 2, frame);
        assert_eq!(xs, [0.0, 100.0]);
        assert_eq!(scales[0].domain, (0.0, 100.0));
        assert_eq!(
            lines[0],
            [(0.0, 100.0), (100.0, 0.0)],
            "the least sits low, the most high"
        );
        assert_eq!(distance((50.0, 50.0), &lines[0]), 0.0);
        assert!(distance((0.0, 0.0), &lines[0]) > 60.0);
    }
}
