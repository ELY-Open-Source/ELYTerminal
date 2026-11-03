use std::collections::HashSet;

use gpui::SharedString;

use super::{
    scale::{Band, Linear, nice},
    series::{Points, extent, stacked},
};

/// How a category series draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mark {
    Line,
    Area,
    Bar,
}

/// What a mark paints with: a series' palette color, a rise or a fall, or the chart's own ink.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ink {
    Series(usize),
    Rise,
    Fall,
    /// Connectors and whiskers, quiet.
    Rule,
    /// Medians and marks that read over a fill.
    Strong,
}

/// A place in a box's own pixels: across, then down.
pub(crate) type Place = (f32, f32);

/// A rectangle in a box's own pixels: left, top, width, height.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub(crate) fn contains(&self, (x, y): (f32, f32)) -> bool {
        x >= self.x && x <= self.x + self.w && y >= self.y && y <= self.y + self.h
    }
}

/// A run of points drawn as a line, filled down to its floor when it is an area.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Line {
    pub ink: Ink,
    pub points: Vec<(f32, f32)>,
    pub floor: Option<Vec<(f32, f32)>>,
}

/// A dot, and the index its tooltip reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Dot {
    pub ink: Ink,
    pub center: (f32, f32),
    pub radius: f32,
    pub key: usize,
}

/// What a chart draws, in its box's own pixels, for the canvas and for SVG alike.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Geometry {
    /// Value ticks: where each sits along the value axis, and its value.
    pub ticks: Vec<(f32, f64)>,
    /// Labels along the other axis and where each sits: categories, or a scatter's x ticks.
    pub labels: Vec<(f32, SharedString)>,
    pub lines: Vec<Line>,
    pub bars: Vec<(Ink, Rect)>,
    pub dots: Vec<Dot>,
    /// Straight strokes from one point to another.
    pub strokes: Vec<(Ink, Place, Place)>,
    /// Closed outlines, filled.
    pub shapes: Vec<(Ink, Vec<(f32, f32)>)>,
}

/// The plot inside a box: past the gutter on the left and the foot below, the inset above and to the right.
pub(crate) fn frame((width, height): (f32, f32), gutter: f32, foot: f32, inset: f32) -> Rect {
    Rect {
        x: gutter,
        y: inset,
        w: (width - gutter - inset).max(0.0),
        h: (height - foot - inset).max(0.0),
    }
}

/// What a category chart asks of its geometry.
pub(crate) struct Categories<'a> {
    pub labels: &'a [SharedString],
    pub names: &'a [SharedString],
    pub values: &'a [Vec<f64>],
    pub mark: Mark,
    pub stacked: bool,
    pub horizontal: bool,
    /// The share of each band bars leave empty.
    pub gap: f32,
    pub hidden: &'a HashSet<SharedString>,
    pub span: (usize, usize),
}

/// Lays out categories and their series inside `frame`: bands along one axis, values along the other.
pub(crate) fn categories(chart: &Categories, frame: Rect) -> Geometry {
    if chart.labels.is_empty() {
        return Geometry::default();
    }
    let (first, last) = chart.span;
    let shown: Vec<(usize, &[f64])> = chart
        .names
        .iter()
        .zip(chart.values)
        .enumerate()
        .filter(|(_, (name, _))| !chart.hidden.contains(*name))
        .map(|(ix, (_, values))| (ix, &values[first..=last]))
        .collect();
    let runs: Vec<&[f64]> = shown.iter().map(|(_, values)| *values).collect();
    let stacks = chart.stacked.then(|| stacked(&runs));
    let (low, high) = match &stacks {
        Some(stacks) => extent(
            stacks
                .iter()
                .flatten()
                .flat_map(|(bottom, top)| [*bottom, *top]),
        ),
        None => extent(runs.iter().flat_map(|values| values.iter().copied())),
    };
    let ((low, high), ticks) = nice(low, high, 5);
    let count = last - first + 1;
    let (value_range, band_range) = if chart.horizontal {
        ((frame.x, frame.x + frame.w), (frame.y, frame.y + frame.h))
    } else {
        ((frame.y + frame.h, frame.y), (frame.x, frame.x + frame.w))
    };
    let scale = Linear::new((low, high), value_range);
    let bars = chart.mark == Mark::Bar;
    let band = Band {
        count,
        range: band_range,
        padding: if bars { chart.gap } else { 0.0 },
    };
    let along = |ix: usize| {
        if bars {
            band.middle(ix)
        } else {
            point_at(&band, ix, count)
        }
    };
    let place = |band_at: f32, value_at: f32| {
        if chart.horizontal {
            (value_at, band_at)
        } else {
            (band_at, value_at)
        }
    };
    let mut geometry = Geometry {
        ticks: ticks.iter().map(|tick| (scale.at(*tick), *tick)).collect(),
        labels: (0..count)
            .map(|ix| (along(ix), chart.labels[first + ix].clone()))
            .collect(),
        ..Geometry::default()
    };
    for (lane, (color, values)) in shown.iter().enumerate() {
        let spans: Vec<(f64, f64)> = match &stacks {
            Some(stacks) => stacks[lane].clone(),
            None => values.iter().map(|value| (0.0, *value)).collect(),
        };
        let ink = Ink::Series(*color);
        if bars {
            for (ix, (bottom, top)) in spans.iter().enumerate() {
                let (start, width) = band.slot(ix);
                let (start, width) = if chart.stacked || shown.len() == 1 {
                    (start, width)
                } else {
                    let each = width / shown.len() as f32;
                    (start + each * lane as f32 + each * 0.07, each * 0.86)
                };
                let (a, b) = (scale.at(*bottom), scale.at(*top));
                let (from, to) = (a.min(b), a.max(b));
                let rect = if chart.horizontal {
                    Rect {
                        x: from,
                        y: start,
                        w: to - from,
                        h: width,
                    }
                } else {
                    Rect {
                        x: start,
                        y: from,
                        w: width,
                        h: to - from,
                    }
                };
                geometry.bars.push((ink, rect));
            }
            continue;
        }
        let points = spans
            .iter()
            .enumerate()
            .map(|(ix, (_, top))| place(along(ix), scale.at(*top)))
            .collect();
        let floor = (chart.mark == Mark::Area).then(|| {
            spans
                .iter()
                .enumerate()
                .map(|(ix, (bottom, _))| place(along(ix), scale.at(*bottom)))
                .collect()
        });
        geometry.lines.push(Line { ink, points, floor });
    }
    geometry
}

/// Where point `ix` of `count` sits: spread edge to edge, or centered when alone.
pub(crate) fn point_at(band: &Band, ix: usize, count: usize) -> f32 {
    if count < 2 {
        return (band.range.0 + band.range.1) / 2.0;
    }
    band.range.0 + (band.range.1 - band.range.0) * ix as f32 / (count - 1) as f32
}

/// Lays out clouds of points: both axes linear, bubbles sized by area up to `widest`; x ticks read through `format`.
pub(crate) fn scatter(
    clouds: &[Points],
    hidden: &HashSet<SharedString>,
    frame: Rect,
    (widest, dot): (f32, f32),
    format: &dyn Fn(f64) -> String,
) -> Geometry {
    let shown = || clouds.iter().filter(|cloud| !hidden.contains(&cloud.name));
    let every = || shown().flat_map(|cloud| cloud.points.iter());
    let (x_low, x_high) = every().fold((f64::MAX, f64::MIN), |(low, high), (x, _)| {
        (low.min(*x), high.max(*x))
    });
    let (y_low, y_high) = every().fold((f64::MAX, f64::MIN), |(low, high), (_, y)| {
        (low.min(*y), high.max(*y))
    });
    if x_low > x_high {
        return Geometry::default();
    }
    let ((x_low, x_high), x_ticks) = nice(x_low, x_high, 5);
    let ((y_low, y_high), y_ticks) = nice(y_low, y_high, 5);
    let xs = Linear::new((x_low, x_high), (frame.x, frame.x + frame.w));
    let ys = Linear::new((y_low, y_high), (frame.y + frame.h, frame.y));
    let largest = shown()
        .filter_map(|cloud| cloud.sizes.as_ref())
        .flatten()
        .copied()
        .fold(0.0, f64::max);
    let mut geometry = Geometry {
        ticks: y_ticks.iter().map(|tick| (ys.at(*tick), *tick)).collect(),
        labels: x_ticks
            .iter()
            .map(|tick| (xs.at(*tick), format(*tick).into()))
            .collect(),
        ..Geometry::default()
    };
    let mut key = 0;
    for (color, cloud) in clouds.iter().enumerate() {
        let skip = hidden.contains(&cloud.name);
        for (ix, (x, y)) in cloud.points.iter().enumerate() {
            let radius = match &cloud.sizes {
                Some(sizes) if largest > 0.0 => widest * (sizes[ix] / largest).sqrt() as f32,
                _ => dot,
            };
            if !skip {
                let center = (xs.at(*x), ys.at(*y));
                geometry.dots.push(Dot {
                    ink: Ink::Series(color),
                    center,
                    radius: radius.max(dot / 2.0),
                    key,
                });
            }
            key += 1;
        }
    }
    geometry
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chart<'a>(
        labels: &'a [SharedString],
        values: &'a [Vec<f64>],
        hidden: &'a HashSet<SharedString>,
        mark: Mark,
        stacked: bool,
    ) -> Categories<'a> {
        static NAMES: [SharedString; 2] = [
            SharedString::new_static("North"),
            SharedString::new_static("South"),
        ];
        Categories {
            labels,
            names: &NAMES,
            values,
            mark,
            stacked,
            horizontal: false,
            gap: 0.28,
            hidden,
            span: (0, 2),
        }
    }

    fn sales() -> Vec<Vec<f64>> {
        vec![vec![10.0, 20.0, 30.0], vec![5.0, 5.0, 10.0]]
    }

    #[test]
    fn stacked_bars_share_a_band_and_grouped_bars_split_it() {
        let labels = ["Jan", "Feb", "Mar"].map(SharedString::from);
        let (values, hidden) = (sales(), HashSet::new());
        let frame = Rect {
            x: 0.0,
            y: 0.0,
            w: 300.0,
            h: 100.0,
        };
        let stack = categories(&chart(&labels, &values, &hidden, Mark::Bar, true), frame);
        assert_eq!(stack.bars.len(), 6);
        assert_eq!(
            stack.bars[0].1.x, stack.bars[3].1.x,
            "a stack shares its band"
        );
        assert_eq!(
            stack.bars[3].1.y + stack.bars[3].1.h,
            stack.bars[0].1.y,
            "the second sits on the first"
        );
        let group = categories(&chart(&labels, &values, &hidden, Mark::Bar, false), frame);
        assert!(
            group.bars[3].1.x > group.bars[0].1.x + group.bars[0].1.w,
            "grouped bars stand apart"
        );
        assert_eq!(group.ticks.first().map(|tick| tick.1), Some(0.0));
    }

    #[test]
    fn lines_run_edge_to_edge_and_hidden_series_drop_out() {
        let labels = ["Jan", "Feb", "Mar"].map(SharedString::from);
        let values = sales();
        let hidden: HashSet<SharedString> = ["South".into()].into_iter().collect();
        let frame = Rect {
            x: 10.0,
            y: 0.0,
            w: 200.0,
            h: 100.0,
        };
        let lines = categories(&chart(&labels, &values, &hidden, Mark::Line, false), frame);
        assert_eq!(lines.lines.len(), 1);
        let xs: Vec<f32> = lines.lines[0].points.iter().map(|point| point.0).collect();
        assert_eq!(xs, [10.0, 110.0, 210.0]);
        assert_eq!(
            lines.lines[0].points[2].1, 0.0,
            "the highest value meets the top of the rounded scale"
        );
        assert_eq!(lines.lines[0].floor, None, "a line has no floor");
    }

    #[test]
    fn bubbles_grow_by_area_and_keep_their_keys_past_a_hidden_cloud() {
        let first = Points::new("Towns", [(1.0, 1.0)]);
        let cloud = Points::new("Cities", [(0.0, 0.0), (10.0, 10.0)]).sizes([1.0, 4.0]);
        let hidden: HashSet<SharedString> = ["Towns".into()].into_iter().collect();
        let frame = Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        };
        let geometry = scatter(&[first, cloud], &hidden, frame, (20.0, 4.0), &|value| {
            value.to_string()
        });
        assert_eq!(geometry.dots[1].radius, 20.0);
        assert_eq!(
            geometry.dots[0].radius, 10.0,
            "a quarter of the area, half the radius"
        );
        assert_eq!((geometry.dots[0].key, geometry.dots[1].key), (1, 2));
        assert_eq!(
            geometry.labels.first().map(|label| label.1.clone()),
            Some("0".into())
        );
    }

    #[test]
    fn no_categories_lay_out_nothing() {
        let hidden = HashSet::new();
        let empty = Categories {
            labels: &[],
            names: &[],
            values: &[],
            mark: Mark::Line,
            stacked: false,
            horizontal: false,
            gap: 0.28,
            hidden: &hidden,
            span: (0, 0),
        };
        assert_eq!(
            categories(
                &empty,
                Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 100.0,
                    h: 100.0
                }
            ),
            Geometry::default()
        );
    }

    #[test]
    fn a_frame_leaves_its_gutters() {
        let rect = frame((400.0, 200.0), 48.0, 24.0, 8.0);
        assert_eq!(
            rect,
            Rect {
                x: 48.0,
                y: 8.0,
                w: 344.0,
                h: 168.0
            }
        );
        assert!(rect.contains((48.0, 8.0)) && !rect.contains((40.0, 50.0)));
        assert_eq!(
            frame((10.0, 10.0), 48.0, 24.0, 8.0).w,
            0.0,
            "a small box leaves no room"
        );
    }
}
