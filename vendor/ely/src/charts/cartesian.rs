use std::{collections::HashSet, rc::Rc};

use gpui::{
    App, Div, ElementId, InteractiveElement, IntoElement, Refineable, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, div,
};

use super::{
    export::{Sheet, svg},
    geometry::{Categories, Ink, Mark, categories, scatter},
    plot::{Format, Layout, Pick, Plot, Scene, Tips, plot},
    scale::compact,
    series::drawable,
    series::{Points, Series, csv, points_csv},
};
use crate::theme::ActiveTheme;

/// A category chart's settings, before its plot takes them.
pub(crate) struct Chart {
    id: ElementId,
    labels: Vec<SharedString>,
    pub series: Vec<Series>,
    mark: Mark,
    /// The share of each band bars leave empty.
    pub gap: f32,
    stacked: bool,
    horizontal: bool,
    smooth: bool,
    rules: Vec<(f64, SharedString)>,
    notes: Vec<(usize, SharedString)>,
    zoom: bool,
    format: Format,
}

impl Chart {
    pub(crate) fn new(id: ElementId, labels: Vec<SharedString>, mark: Mark) -> Self {
        Self {
            id,
            labels,
            series: Vec::new(),
            mark,
            gap: 0.28,
            stacked: false,
            horizontal: false,
            smooth: false,
            rules: Vec::new(),
            notes: Vec::new(),
            zoom: false,
            format: Rc::new(compact),
        }
    }

    fn names(&self) -> Vec<SharedString> {
        self.series
            .iter()
            .map(|series| series.name.clone())
            .collect()
    }

    fn values(&self) -> Vec<Vec<f64>> {
        self.series
            .iter()
            .map(|series| series.values.clone())
            .collect()
    }

    fn layout(&self) -> Layout {
        let (labels, names) = (self.labels.clone(), self.names());
        let (mark, stacked, horizontal, gap) = (self.mark, self.stacked, self.horizontal, self.gap);
        Rc::new(move |scene: &Scene| {
            let chart = Categories {
                labels: &labels,
                names: &names,
                values: scene.values,
                mark,
                stacked,
                horizontal,
                gap,
                hidden: scene.hidden,
                span: scene.span,
            };
            categories(&chart, scene.frame)
        })
    }

    fn tips(&self) -> Tips {
        let (labels, names, format) = (self.labels.clone(), self.names(), self.format.clone());
        Rc::new(move |ix, scene: &Scene| {
            let rows = names
                .iter()
                .enumerate()
                .filter(|(_, name)| !scene.hidden.contains(*name))
                .map(|(color, name)| {
                    (
                        Some(Ink::Series(color)),
                        name.clone(),
                        format(scene.values[color][ix]),
                    )
                })
                .collect();
            (labels[ix].clone(), rows)
        })
    }

    pub(crate) fn plot(self) -> Plot {
        let pick = if self.mark == Mark::Bar {
            Pick::Band
        } else {
            Pick::Point
        };
        let mut plot = Plot::new(self.id.clone(), pick, self.layout(), self.tips());
        (plot.names, plot.values) = (self.names(), self.values());
        (plot.labels, plot.horizontal, plot.smooth) = (self.labels, self.horizontal, self.smooth);
        (plot.rules, plot.notes, plot.zoom, plot.format) =
            (self.rules, self.notes, self.zoom, self.format);
        plot
    }

    fn svg(&self, width: f32, height: f32, cx: &App) -> String {
        let sheet = Sheet::new((width, height), self.horizontal, self.smooth, cx);
        let (values, hidden) = (self.values(), HashSet::new());
        let span = (0, self.labels.len().saturating_sub(1));
        let scene = Scene {
            frame: sheet.frame,
            values: &values,
            hidden: &hidden,
            span,
        };
        svg(&(self.layout())(&scene), &sheet, &*self.format)
    }
}

/// Renders a plot at the theme's chart height unless the owner sizes it.
pub(crate) fn sized(base: &mut Div, plotted: Plot, window: &mut Window, cx: &mut App) -> Div {
    let mut frame = div()
        .debug_selector(|| "chart-root".into())
        .h(cx.theme().chart().height);
    frame.style().refine(base.style());
    plot(plotted, frame, window, cx)
}

macro_rules! category_chart {
    ($name:ident, $mark:expr, $doc:literal) => {
        #[doc = $doc]
        #[derive(IntoElement)]
        pub struct $name {
            base: Div,
            chart: Chart,
        }

        impl $name {
            pub fn new(
                id: impl Into<ElementId>,
                labels: impl IntoIterator<Item = impl Into<SharedString>>,
            ) -> Self {
                let labels = labels.into_iter().map(Into::into).collect();
                Self {
                    base: div(),
                    chart: Chart::new(id.into(), labels, $mark),
                }
            }

            /// A run of values, one for each label.
            pub fn series(mut self, series: Series) -> Self {
                assert_eq!(
                    series.values.len(),
                    self.chart.labels.len(),
                    "series {} needs a value per label",
                    series.name
                );
                self.chart.series.push(series);
                self
            }

            /// A dashed level across the chart with its label, as a target or a limit.
            pub fn rule(mut self, value: f64, label: impl Into<SharedString>) -> Self {
                assert!(
                    drawable(value),
                    "a rule needs a value, within charts::LIMIT"
                );
                self.chart.rules.push((value, label.into()));
                self
            }

            /// A mark at category `ix` with its label, as a release or an incident.
            pub fn note(mut self, ix: usize, label: impl Into<SharedString>) -> Self {
                let label = label.into();
                if ix >= self.chart.labels.len() {
                    log::error!("chart: note {label} at {ix} is past the last label; skipped");
                    return self;
                }
                self.chart.notes.push((ix, label));
                self
            }

            /// A drag across the chart zooms to its span; Show all, or a double press, shows every category again.
            pub fn zoom(mut self) -> Self {
                self.chart.zoom = true;
                self
            }

            /// How values read on the axis and in the tooltip.
            pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
                self.chart.format = Rc::new(format);
                self
            }

            /// The data as comma-separated text: a header of labels, then a row per series.
            pub fn csv(&self) -> String {
                csv(&self.chart.labels, &self.chart.series)
            }

            /// The chart as an SVG drawing of this size, in the theme's colors.
            pub fn svg(&self, width: f32, height: f32, cx: &App) -> String {
                self.chart.svg(width, height, cx)
            }
        }

        impl Styled for $name {
            fn style(&mut self) -> &mut StyleRefinement {
                self.base.style()
            }
        }

        impl RenderOnce for $name {
            fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
                sized(&mut self.base, self.chart.plot(), window, cx)
            }
        }
    };
}

category_chart!(
    LineChart,
    Mark::Line,
    "Values over categories as lines. Hover for a crosshair and every series' value; a press on a legend name hides it."
);
category_chart!(
    AreaChart,
    Mark::Area,
    "Values over categories as lines with the area beneath lightly filled; `stacked` piles each series on the ones before it."
);
category_chart!(
    BarChart,
    Mark::Bar,
    "Values over categories as bars: grouped side by side, `stacked`, or `horizontal`."
);

impl LineChart {
    /// Curves through the values that never overshoot them.
    pub fn smooth(mut self) -> Self {
        self.chart.smooth = true;
        self
    }
}

impl AreaChart {
    pub fn smooth(mut self) -> Self {
        self.chart.smooth = true;
        self
    }

    pub fn stacked(mut self) -> Self {
        self.chart.stacked = true;
        self
    }
}

impl BarChart {
    pub fn stacked(mut self) -> Self {
        self.chart.stacked = true;
        self
    }

    /// Bars run sideways, categories down the side.
    pub fn horizontal(mut self) -> Self {
        self.chart.horizontal = true;
        self.chart.gap = 0.45;
        self
    }
}

/// Points placed by two values; with sizes, bubbles whose areas compare. Hover a point to read it.
#[derive(IntoElement)]
pub struct ScatterChart {
    base: Div,
    id: ElementId,
    clouds: Vec<Points>,
    axes: (SharedString, SharedString),
    format: Format,
}

impl ScatterChart {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            clouds: Vec::new(),
            axes: ("x".into(), "y".into()),
            format: Rc::new(compact),
        }
    }

    pub fn points(mut self, points: Points) -> Self {
        self.clouds.push(points);
        self
    }

    /// What the two values measure, as the tooltip names them.
    pub fn axes(mut self, x: impl Into<SharedString>, y: impl Into<SharedString>) -> Self {
        self.axes = (x.into(), y.into());
        self
    }

    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }

    /// The points as comma-separated text: a row per point with its series, x, y and size.
    pub fn csv(&self) -> String {
        points_csv(&self.clouds)
    }

    pub fn svg(&self, width: f32, height: f32, cx: &App) -> String {
        let sheet = Sheet::new((width, height), false, false, cx);
        let (values, hidden) = (Vec::new(), HashSet::new());
        let scene = Scene {
            frame: sheet.frame,
            values: &values,
            hidden: &hidden,
            span: (0, 0),
        };
        svg(
            &(self.layout(radii(cx.theme().base_rem(), cx)))(&scene),
            &sheet,
            &*self.format,
        )
    }

    fn layout(&self, radii: (f32, f32)) -> Layout {
        let (clouds, format) = (Rc::new(self.clouds.clone()), self.format.clone());
        Rc::new(move |scene: &Scene| scatter(&clouds, scene.hidden, scene.frame, radii, &*format))
    }

    fn tips(&self) -> Tips {
        let (clouds, format, (x_name, y_name)) = (
            Rc::new(self.clouds.clone()),
            self.format.clone(),
            self.axes.clone(),
        );
        Rc::new(move |key, _| {
            let (cloud, ix) = locate(&clouds, key);
            let ((x, y), points) = (clouds[cloud].points[ix], &clouds[cloud]);
            let mut rows = vec![
                (None, x_name.clone(), format(x)),
                (None, y_name.clone(), format(y)),
            ];
            rows.extend(
                points
                    .sizes
                    .as_ref()
                    .map(|sizes| (None, "Size".into(), format(sizes[ix]))),
            );
            (points.name.clone(), rows)
        })
    }
}

/// The widest bubble's radius and a plain dot's, at a rem size.
fn radii(rem: gpui::Pixels, cx: &App) -> (f32, f32) {
    let theme = cx.theme();
    (
        f32::from(theme.chart().bubble.to_pixels(rem)),
        f32::from(theme.status_dot().to_pixels(rem)) / 2.0,
    )
}

/// Which cloud and which of its points a flat index names.
fn locate(clouds: &[Points], mut key: usize) -> (usize, usize) {
    for (cloud, points) in clouds.iter().enumerate() {
        if key < points.points.len() {
            return (cloud, key);
        }
        key -= points.points.len();
    }
    panic!("point {key} is past the last cloud");
}

impl Styled for ScatterChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for ScatterChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let layout = self.layout(radii(window.rem_size(), cx));
        let mut plotted = Plot::new(self.id.clone(), Pick::Dot, layout, self.tips());
        plotted.names = self.clouds.iter().map(|cloud| cloud.name.clone()).collect();
        plotted.format = self.format.clone();
        sized(&mut self.base, plotted, window, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::{Points, locate};

    #[test]
    fn a_flat_index_finds_its_cloud() {
        let clouds = [
            Points::new("a", [(0.0, 0.0), (1.0, 1.0)]),
            Points::new("b", [(2.0, 2.0)]),
        ];
        assert_eq!(locate(&clouds, 1), (0, 1));
        assert_eq!(locate(&clouds, 2), (1, 0));
    }
}
