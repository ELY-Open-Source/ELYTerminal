mod limits;

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use gpui::{
    Context, IntoElement, KeyUpEvent, Keystroke, Modifiers, MouseMoveEvent, ParentElement, Render,
    SharedString, Styled, TestAppContext, VisualTestContext, Window, black, div, point, px,
};
use jiff::civil::date;

use super::axes::PAD;
use super::{
    AreaChart, BarChart, BoxPlot, Bullet, BulletChart, CalendarHeatmap, ChartLegend, ChordDiagram,
    FunnelChart, GanttChart, HeatmapChart, Histogram, LineChart, NetworkGraph, ParallelCoordinates,
    PieChart, Points, ProgressChart, RadarChart, RealtimeChart, SankeyChart, ScatterChart, Series,
    Slice, Sunburst, Task, Treemap, ViolinPlot, WaterfallChart,
};
use crate::{layout::tests::narrow_width, theme::Theme};

#[gpui::test]
fn a_chart_exports_as_svg_and_csv(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        let line = LineChart::new("sales", ["Jan", "Feb", "Mar"])
            .series(Series::new("Sales, net", [1.0, 3.0, 2.0]))
            .smooth();
        let svg = line.svg(400.0, 200.0, cx);
        assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
        assert_eq!(svg.matches("<path").count(), 1, "one line, no area");
        assert!(svg.contains(" C"), "a smooth line curves");
        assert!(svg.contains(">Feb</text>"), "categories label the axis");
        assert_eq!(line.csv(), ",Jan,Feb,Mar\n\"Sales, net\",1,3,2");
        let bars = BarChart::new("share", ["A", "B"])
            .series(Series::new("Now", [2.0, 4.0]))
            .series(Series::new("Before", [1.0, 3.0]));
        let svg = bars.svg(400.0, 200.0, cx);
        assert_eq!(
            svg.matches("<rect").count(),
            5,
            "the page, then a bar per value"
        );
    });
}

/// Every chart that picks a part under the pointer, stacked 200 tall at 400 across, with many parts or few.
struct Picked(Rc<Cell<bool>>);

const TALL: f32 = 200.0;

impl Render for Picked {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let many = self.0.get();
        let count = if many { 6 } else { 3 };
        let values = |count: usize| {
            (0..count)
                .map(|ix| 10.0 + ix as f64 * 3.0)
                .collect::<Vec<_>>()
        };
        let names: Vec<String> = (0..count).map(|ix| format!("n{ix}")).collect();
        let tile = |chart: gpui::AnyElement| {
            div()
                .w(px(400.0))
                .h(px(TALL))
                .overflow_hidden()
                .child(chart)
        };
        let treemap = names
            .iter()
            .zip(values(count))
            .fold(Treemap::new("treemap"), |map, (name, value)| {
                map.tile(name.clone(), value)
            });
        let heatmap = names
            .iter()
            .fold(HeatmapChart::new("heatmap", names.clone()), |map, name| {
                map.row(name.clone(), values(count))
            });
        let radar = RadarChart::new("radar", names.clone()).series(Series::new("a", values(count)));
        let funnel = names
            .iter()
            .zip(values(count).into_iter().rev())
            .fold(FunnelChart::new("funnel"), |funnel, (name, value)| {
                funnel.stage(name.clone(), value)
            });
        let sankey = (1..count).fold(SankeyChart::new("sankey", names.clone()), |sankey, to| {
            sankey.link(0, to, 5.0)
        });
        let gantt = (0..count).fold(GanttChart::new("gantt"), |gantt, ix| {
            gantt.task(Task::new(
                format!("t{ix}"),
                date(2026, 3, 1 + ix as i8),
                date(2026, 3, 9 + ix as i8),
            ))
        });
        let network = (1..count).fold(
            names
                .iter()
                .fold(NetworkGraph::new("network"), |graph, name| {
                    graph.node(name.clone(), 0)
                }),
            |graph, to| graph.edge(0, to),
        );
        let matrix: Vec<Vec<f64>> = (0..count)
            .map(|row| {
                (0..count)
                    .map(|col| if row == col { 0.0 } else { 1.0 + col as f64 })
                    .collect()
            })
            .collect();
        let chord = ChordDiagram::new("chord", names.clone(), matrix);
        let parallel = (0..count).fold(
            ParallelCoordinates::new("parallel", ["a", "b"]),
            |chart, ix| chart.record(format!("r{ix}"), [ix as f64, (count - ix) as f64]),
        );
        let bullets = (0..count).fold(BulletChart::new("bullets"), |chart, ix| {
            chart.bullet(Bullet::new(format!("b{ix}"), 5.0 + ix as f64, 8.0))
        });
        let rings = (0..count).fold(ProgressChart::new("rings"), |chart, ix| {
            chart.goal(format!("g{ix}"), 3.0 + ix as f64, 10.0)
        });
        let days = CalendarHeatmap::new("days", date(2026, 1, 5), values(count * 10));
        div()
            .flex()
            .flex_col()
            .child(tile(treemap.h(px(TALL)).into_any_element()))
            .child(tile(heatmap.h(px(TALL)).into_any_element()))
            .child(tile(radar.h(px(TALL)).into_any_element()))
            .child(tile(funnel.h(px(TALL)).into_any_element()))
            .child(tile(sankey.h(px(TALL)).into_any_element()))
            .child(tile(gantt.h(px(TALL)).into_any_element()))
            .child(tile(network.h(px(TALL)).into_any_element()))
            .child(tile(chord.h(px(TALL)).into_any_element()))
            .child(tile(parallel.h(px(TALL)).into_any_element()))
            .child(tile(bullets.h(px(TALL)).into_any_element()))
            .child(tile(rings.h(px(TALL)).into_any_element()))
            .child(tile(days.h(px(TALL)).into_any_element()))
    }
}

pub(super) fn settle(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.run_until_parked();
        cx.update(|window, _| window.refresh());
    }
    cx.run_until_parked();
}

#[gpui::test]
fn a_part_under_the_pointer_lets_go_when_its_data_shrinks(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.reduced_motion = true);
    });
    let many = Rc::new(Cell::new(true));
    let seen = many.clone();
    let (view, cx) = cx.add_window_view(|_, _| Picked(seen));
    cx.simulate_resize(gpui::size(px(400.0), px(TALL * 12.0)));
    settle(cx);
    for chart in 0..12 {
        for (x, y) in [(360.0, 20.0), (200.0, 100.0), (60.0, 170.0)] {
            let at = point(px(x), px(TALL * chart as f32 + y));
            cx.simulate_event(MouseMoveEvent {
                position: at,
                pressed_button: None,
                modifiers: Modifiers::none(),
            });
            settle(cx);
            for full in [false, true] {
                many.set(full);
                view.update(cx, |_, cx| cx.notify());
                settle(cx);
            }
        }
    }
}

/// A legend of two series that hears which one is toggled.
struct Keyed(Rc<RefCell<Vec<SharedString>>>);

impl Render for Keyed {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let heard = self.0.clone();
        div().child(
            ChartLegend::new("legend")
                .entry(black(), "Training", false)
                .entry(black(), "Held out", true)
                .on_toggle(move |name, _, _| heard.borrow_mut().push(name.clone())),
        )
    }
}

#[gpui::test]
fn tab_reaches_a_legend_entry_and_enter_toggles_it(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let heard = Rc::new(RefCell::new(Vec::new()));
    let seen = heard.clone();
    let (_, cx) = cx.add_window_view(|_, _| Keyed(seen));
    cx.update(|window, _| window.activate_window());
    settle(cx);
    cx.update(|window, cx| {
        window.focus_next(cx);
        window.focus_next(cx);
    });
    settle(cx);
    cx.simulate_keystrokes("enter");
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("enter").expect("a key"),
    });
    settle(cx);
    assert_eq!(*heard.borrow(), ["Held out"]);
}

#[gpui::test]
fn every_chart_fills_a_column_its_block_measures_by_content(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let two = || Series::new("s", [1.0, 2.0]);
    let pairs = [vec![0.0, 1.0], vec![1.0, 0.0]];
    let widths = [
        narrow_width(cx, "chart-root", move |_, _| {
            LineChart::new("line", ["a", "b"])
                .series(two())
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", move |_, _| {
            AreaChart::new("area", ["a", "b"])
                .series(two())
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", move |_, _| {
            BarChart::new("bar", ["a", "b"])
                .series(two())
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            let points = Points::new("p", [(0.0, 1.0), (1.0, 2.0)]);
            ScatterChart::new("scatter")
                .points(points)
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            Histogram::new("histogram", [1.0, 2.0, 2.0, 3.0]).into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            WaterfallChart::new("waterfall")
                .step("a", 2.0)
                .step("b", -1.0)
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            BoxPlot::new("box")
                .group("a", [1.0, 2.0, 3.0])
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            ViolinPlot::new("violin")
                .group("a", [1.0, 2.0, 3.0])
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            CalendarHeatmap::new("days", date(2026, 1, 5), [1.0; 14]).into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            SankeyChart::new("sankey", ["a", "b"])
                .link(0, 1, 5.0)
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            FunnelChart::new("funnel")
                .stage("a", 2.0)
                .stage("b", 1.0)
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            NetworkGraph::new("network")
                .node("a", 0)
                .node("b", 0)
                .edge(0, 1)
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            ParallelCoordinates::new("parallel", ["a", "b"])
                .record("r", [1.0, 2.0])
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            PieChart::new("pie")
                .slice("a", 1.0)
                .slice("b", 2.0)
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", move |_, _| {
            RadarChart::new("radar", ["a", "b", "c"])
                .series(Series::new("s", [1.0, 2.0, 3.0]))
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            RealtimeChart::new("live", [1.0, 2.0], 2).into_any_element()
        }),
        narrow_width(cx, "chart-root", move |_, _| {
            ChordDiagram::new("chord", ["a", "b"], pairs.to_vec()).into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            GanttChart::new("gantt")
                .task(Task::new("t", date(2026, 3, 1), date(2026, 3, 9)))
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            Sunburst::new("sun", [Slice::new("a", 1.0)]).into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            BulletChart::new("bullets")
                .bullet(Bullet::new("b", 5.0, 8.0))
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            Treemap::new("treemap")
                .tile("a", 1.0)
                .tile("b", 2.0)
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            HeatmapChart::new("heatmap", ["a"])
                .row("a", [1.0])
                .into_any_element()
        }),
    ];
    assert_eq!(
        widths,
        [px(240.0); 22],
        "each chart spans the card inside its padding"
    );
}

/// A sideways bar chart `wide` across with one band named `name`.
struct Sideways(String, f32);

impl Render for Sideways {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(self.1)).child(
            BarChart::new("sideways", [self.0.clone()])
                .series(Series::new("s", [3.0]))
                .horizontal(),
        )
    }
}

#[gpui::test]
fn a_long_band_name_widens_the_gutter_to_half_the_chart(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let long = "x".repeat(200);
    for (name, wide) in [
        ("claude-sonnet-4-5-20250929", 400.0),
        (&long, 400.0),
        (&long, 80.0),
    ] {
        let (_, cx) = cx.add_window_view(|_, _| Sideways(name.to_string(), wide));
        settle(cx);
        let root = cx.debug_bounds("chart-root").expect("the chart draws");
        let band = cx.debug_bounds("chart-band").expect("the band is named");
        let half = root.left() + px(wide / 2.0);
        let pad = cx.update(|window, _| PAD.to_pixels(window.rem_size()));
        assert!(
            band.left() >= root.left(),
            "{wide}: the name's box starts in the chart"
        );
        assert!(
            band.right() + pad <= half,
            "{wide}: the gutter takes half at most"
        );
        if wide > 200.0 {
            assert!(band.size.width > px(48.0), "past the theme gutter");
        }
    }
}

/// A note, a link, an edge and a wait that name what their chart lacks.
struct Dangling;

impl Render for Dangling {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let line = LineChart::new("line", ["a", "b"])
            .series(Series::new("s", [1.0, 2.0]))
            .note(7, "late");
        let sankey = SankeyChart::new("sankey", ["a", "b"])
            .link(0, 9, 5.0)
            .link(0, 1, 5.0);
        let network = NetworkGraph::new("network")
            .node("a", 0)
            .node("b", 0)
            .edge(0, 9);
        let gantt = GanttChart::new("gantt")
            .task(Task::new("t", date(2026, 3, 1), date(2026, 3, 9)).after(4));
        div()
            .w(px(400.0))
            .child(line)
            .child(sankey)
            .child(network)
            .child(gantt)
    }
}

#[gpui::test]
fn entries_past_their_lists_are_skipped(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| Dangling);
    settle(cx);
}
