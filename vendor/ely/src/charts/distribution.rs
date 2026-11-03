use std::rc::Rc;

use gpui::{
    App, Div, ElementId, IntoElement, RenderOnce, SharedString, StyleRefinement, Styled, Window,
    div,
};

use super::{
    cartesian::{Chart, sized},
    geometry::{Dot, Geometry, Ink, Mark, Rect},
    plot::{Layout, Pick, Plot, Scene, Tips},
    scale::{Band, Linear, compact, nice},
    series::Series,
    series::drawable,
    stats::{Step, bins, density, five, steps},
};

/// Value ticks up the side and a band per category along the bottom of `frame`.
fn axes(
    labels: &[SharedString],
    (low, high): (f64, f64),
    frame: Rect,
    padding: f32,
) -> (Geometry, Linear, Band) {
    let ((low, high), ticks) = nice(low, high, 5);
    let scale = Linear::new((low, high), (frame.y + frame.h, frame.y));
    let band = Band {
        count: labels.len().max(1),
        range: (frame.x, frame.x + frame.w),
        padding,
    };
    let geometry = Geometry {
        ticks: ticks.iter().map(|tick| (scale.at(*tick), *tick)).collect(),
        labels: labels
            .iter()
            .enumerate()
            .map(|(ix, label)| (band.middle(ix), label.clone()))
            .collect(),
        ..Geometry::default()
    };
    (geometry, scale, band)
}

/// A plot of categories whose marks its own layout draws; the pointer picks a band.
fn banded(id: ElementId, labels: Vec<SharedString>, layout: Layout, tips: Tips) -> Plot {
    let mut plot = Plot::new(id, Pick::Band, layout, tips);
    plot.labels = labels;
    plot
}

macro_rules! sized_chart {
    ($name:ident) => {
        impl Styled for $name {
            fn style(&mut self) -> &mut StyleRefinement {
                self.base.style()
            }
        }

        impl RenderOnce for $name {
            fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
                let mut base = std::mem::replace(&mut self.base, div());
                sized(&mut base, self.plot(), window, cx)
            }
        }
    };
}

/// How values spread: bars counting them in equal bins across their rounded range.
#[derive(IntoElement)]
pub struct Histogram {
    base: Div,
    id: ElementId,
    values: Vec<f64>,
    bins: usize,
}

impl Histogram {
    pub fn new(id: impl Into<ElementId>, values: impl IntoIterator<Item = f64>) -> Self {
        let values: Vec<f64> = values.into_iter().collect();
        assert!(
            values.iter().all(|value| drawable(*value)),
            "a histogram needs values, within charts::LIMIT"
        );
        Self {
            base: div(),
            id: id.into(),
            values,
            bins: 10,
        }
    }

    pub fn bins(mut self, bins: usize) -> Self {
        assert!(bins > 0, "a histogram needs a bin");
        self.bins = bins;
        self
    }

    fn plot(self) -> Plot {
        let bins = bins(&self.values, self.bins);
        let labels = bins
            .iter()
            .map(|(start, end, _)| format!("{}–{}", compact(*start), compact(*end)).into())
            .collect();
        let mut chart = Chart::new(self.id, labels, Mark::Bar);
        chart.gap = 0.06;
        chart
            .series
            .push(Series::new("Count", bins.iter().map(|bin| bin.2 as f64)));
        chart.plot()
    }
}

sized_chart!(Histogram);

/// A running total from a start through its changes: rises and falls float from where the last left off, and totals stand on zero.
#[derive(IntoElement)]
pub struct WaterfallChart {
    base: Div,
    id: ElementId,
    steps: Vec<(SharedString, f64, bool)>,
}

impl WaterfallChart {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            steps: Vec::new(),
        }
    }

    /// A change, up or down, from the running total.
    pub fn step(mut self, label: impl Into<SharedString>, change: f64) -> Self {
        assert!(
            drawable(change),
            "a step needs a change, within charts::LIMIT"
        );
        self.steps.push((label.into(), change, false));
        self
    }

    /// A bar showing the running total so far.
    pub fn total(mut self, label: impl Into<SharedString>) -> Self {
        self.steps.push((label.into(), 0.0, true));
        self
    }

    fn plot(self) -> Plot {
        let labels: Vec<SharedString> = self
            .steps
            .iter()
            .map(|(label, _, _)| label.clone())
            .collect();
        let bars = Rc::new(steps(
            &self
                .steps
                .iter()
                .map(|(_, change, total)| (*change, *total))
                .collect::<Vec<_>>(),
        ));
        let ink = |step: Step| match step {
            Step::Up => Ink::Rise,
            Step::Down => Ink::Fall,
            Step::Total => Ink::Series(0),
        };
        let running: Rc<Vec<f64>> = Rc::new(
            self.steps
                .iter()
                .scan(0.0, |sum, (_, change, total)| {
                    *sum += if *total { 0.0 } else { *change };
                    Some(*sum)
                })
                .collect(),
        );
        let (drawn, names, levels) = (bars.clone(), labels.clone(), running.clone());
        let layout: Layout = Rc::new(move |scene: &Scene| {
            let (low, high) = drawn
                .iter()
                .fold((0.0f64, 0.0f64), |(low, high), (bottom, top, _)| {
                    (low.min(*bottom), high.max(*top))
                });
            let (mut geometry, scale, band) = axes(&names, (low, high), scene.frame, 0.3);
            for (ix, (bottom, top, step)) in drawn.iter().enumerate() {
                let (start, width) = band.slot(ix);
                let (y0, y1) = (scale.at(*top), scale.at(*bottom));
                geometry.bars.push((
                    ink(*step),
                    Rect {
                        x: start,
                        y: y0,
                        w: width,
                        h: y1 - y0,
                    },
                ));
                if ix + 1 < drawn.len() {
                    let level = scale.at(levels[ix]);
                    geometry.strokes.push((
                        Ink::Rule,
                        (start + width, level),
                        (band.slot(ix + 1).0, level),
                    ));
                }
            }
            geometry
        });
        let changes = self.steps;
        let tips: Tips = Rc::new(move |ix, _| {
            let ((label, change, total), (_, _, step)) = (&changes[ix], bars[ix]);
            let (name, value) = if *total {
                ("Total", compact(running[ix]))
            } else {
                (
                    "Change",
                    format!(
                        "{}{}",
                        if *change >= 0.0 { "+" } else { "" },
                        compact(*change)
                    ),
                )
            };
            (label.clone(), vec![(Some(ink(step)), name.into(), value)])
        });
        banded(self.id, labels, layout, tips)
    }
}

sized_chart!(WaterfallChart);

/// Spreads side by side: a box from the first quartile to the third with the median across it, whiskers to the farthest values within reach, and dots past them.
#[derive(IntoElement)]
pub struct BoxPlot {
    base: Div,
    id: ElementId,
    groups: Vec<(SharedString, Vec<f64>)>,
}

/// Spreads side by side as mirrored shapes, each as wide as its values gather, the median marked.
#[derive(IntoElement)]
pub struct ViolinPlot {
    base: Div,
    id: ElementId,
    groups: Vec<(SharedString, Vec<f64>)>,
}

macro_rules! grouped {
    ($name:ident) => {
        impl $name {
            pub fn new(id: impl Into<ElementId>) -> Self {
                Self {
                    base: div(),
                    id: id.into(),
                    groups: Vec::new(),
                }
            }

            /// A named group of values.
            pub fn group(
                mut self,
                label: impl Into<SharedString>,
                values: impl IntoIterator<Item = f64>,
            ) -> Self {
                let values: Vec<f64> = values.into_iter().collect();
                assert!(
                    values.len() >= 2 && values.iter().all(|value| drawable(*value)),
                    "a group needs two values, within charts::LIMIT"
                );
                self.groups.push((label.into(), values));
                self
            }
        }
    };
}

grouped!(BoxPlot);
grouped!(ViolinPlot);

/// Where every group's values run, low to high.
fn reach(groups: &[(SharedString, Vec<f64>)]) -> (f64, f64) {
    groups
        .iter()
        .flat_map(|(_, values)| values.iter())
        .fold((f64::MAX, f64::MIN), |(low, high), value| {
            (low.min(*value), high.max(*value))
        })
}

/// A group's five numbers as tooltip rows.
fn spread_tips(groups: Rc<Vec<(SharedString, Vec<f64>)>>) -> Tips {
    Rc::new(move |ix, _| {
        let (label, values) = &groups[ix];
        let spread = five(values);
        let rows = [
            ("Highest", spread.high),
            ("Third quartile", spread.q3),
            ("Median", spread.median),
            ("First quartile", spread.q1),
            ("Lowest", spread.low),
        ]
        .into_iter()
        .map(|(name, value)| (None, name.into(), compact(value)))
        .collect();
        (label.clone(), rows)
    })
}

/// A rectangle as the outline of a shape.
fn outline(rect: Rect) -> Vec<(f32, f32)> {
    vec![
        (rect.x, rect.y),
        (rect.x + rect.w, rect.y),
        (rect.x + rect.w, rect.y + rect.h),
        (rect.x, rect.y + rect.h),
    ]
}

impl BoxPlot {
    fn plot(self) -> Plot {
        let labels: Vec<SharedString> =
            self.groups.iter().map(|(label, _)| label.clone()).collect();
        let groups = Rc::new(self.groups);
        let (drawn, names) = (groups.clone(), labels.clone());
        let layout: Layout = Rc::new(move |scene: &Scene| {
            let (mut geometry, scale, band) = axes(&names, reach(&drawn), scene.frame, 0.55);
            for (ix, (_, values)) in drawn.iter().enumerate() {
                let (spread, (start, width)) = (five(values), band.slot(ix));
                let middle = start + width / 2.0;
                let (top, bottom) = (scale.at(spread.q3), scale.at(spread.q1));
                geometry.shapes.push((
                    Ink::Series(0),
                    outline(Rect {
                        x: start,
                        y: top,
                        w: width,
                        h: bottom - top,
                    }),
                ));
                geometry.strokes.push((
                    Ink::Strong,
                    (start, scale.at(spread.median)),
                    (start + width, scale.at(spread.median)),
                ));
                for (from, to) in [(spread.q3, spread.high), (spread.q1, spread.low)] {
                    geometry.strokes.push((
                        Ink::Rule,
                        (middle, scale.at(from)),
                        (middle, scale.at(to)),
                    ));
                    geometry.strokes.push((
                        Ink::Rule,
                        (middle - width / 4.0, scale.at(to)),
                        (middle + width / 4.0, scale.at(to)),
                    ));
                }
                for outlier in spread.outliers {
                    geometry.dots.push(Dot {
                        ink: Ink::Series(0),
                        center: (middle, scale.at(outlier)),
                        radius: width / 16.0,
                        key: ix,
                    });
                }
            }
            geometry
        });
        banded(self.id, labels, layout, spread_tips(groups))
    }
}

impl ViolinPlot {
    fn plot(self) -> Plot {
        let labels: Vec<SharedString> =
            self.groups.iter().map(|(label, _)| label.clone()).collect();
        let groups = Rc::new(self.groups);
        let (drawn, names) = (groups.clone(), labels.clone());
        let layout: Layout = Rc::new(move |scene: &Scene| {
            let shapes: Vec<Vec<(f64, f64)>> = drawn
                .iter()
                .map(|(_, values)| density(values, 40))
                .collect();
            let (low, high) = shapes
                .iter()
                .flatten()
                .fold((f64::MAX, f64::MIN), |(low, high), (at, _)| {
                    (low.min(*at), high.max(*at))
                });
            let widest = shapes
                .iter()
                .flatten()
                .fold(0.0f64, |widest, (_, value)| widest.max(*value));
            let (mut geometry, scale, band) = axes(&names, (low, high), scene.frame, 0.2);
            for (ix, shape) in shapes.iter().enumerate() {
                let (start, width) = band.slot(ix);
                let middle = start + width / 2.0;
                let half = |value: f64| (width / 2.0) * (value / widest) as f32;
                let right = shape
                    .iter()
                    .map(|(at, value)| (middle + half(*value), scale.at(*at)));
                let left = shape
                    .iter()
                    .rev()
                    .map(|(at, value)| (middle - half(*value), scale.at(*at)));
                geometry
                    .shapes
                    .push((Ink::Series(ix), right.chain(left).collect()));
                let median = scale.at(five(&drawn[ix].1).median);
                geometry.strokes.push((
                    Ink::Strong,
                    (middle - width / 6.0, median),
                    (middle + width / 6.0, median),
                ));
            }
            geometry
        });
        banded(self.id, labels, layout, spread_tips(groups))
    }
}

sized_chart!(BoxPlot);
sized_chart!(ViolinPlot);
