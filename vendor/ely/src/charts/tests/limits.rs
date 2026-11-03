use std::{cell::Cell, rc::Rc};

use gpui::{
    AnyElement, Context, IntoElement, Modifiers, ParentElement, Render, Styled, TestAppContext,
    Window, div, point, px,
};
use jiff::civil::date;

use super::settle;
use crate::{
    charts::{
        AreaChart, BarChart, BoxPlot, Bullet, BulletChart, CalendarHeatmap, ChordDiagram,
        FunnelChart, HeatmapChart, Histogram, LIMIT, LineChart, ParallelCoordinates, PieChart,
        ProgressChart, RadarChart, SankeyChart, Series, Slice, Sunburst, Treemap, ViolinPlot,
        WaterfallChart,
    },
    theme::{ActiveTheme, Theme},
};

/// Every chart fed values at the edge of what it draws.
struct Edge(u8);

impl Render for Edge {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let top = Series::new("s", [LIMIT, LIMIT]);
        let chart: AnyElement = match self.0 {
            0 => SankeyChart::new("sankey", ["a", "b"])
                .link(0, 1, LIMIT)
                .into_any_element(),
            1 => ChordDiagram::new("chord", ["a", "b"], vec![vec![0.0, LIMIT], vec![1.0, 0.0]])
                .into_any_element(),
            2 => Histogram::new("histogram", [-LIMIT, LIMIT]).into_any_element(),
            3 => ViolinPlot::new("violin")
                .group("s", [LIMIT, LIMIT])
                .into_any_element(),
            4 => AreaChart::new("stack", ["a", "b"])
                .series(top.clone())
                .series(Series::new("t", [LIMIT, LIMIT]))
                .stacked()
                .into_any_element(),
            5 => WaterfallChart::new("waterfall")
                .step("a", LIMIT)
                .step("b", LIMIT)
                .total("all")
                .into_any_element(),
            6 => Sunburst::new(
                "sunburst",
                [Slice::new("root", 0.0)
                    .children([Slice::new("a", LIMIT), Slice::new("b", LIMIT)])],
            )
            .into_any_element(),
            7 => Treemap::new("treemap")
                .tile("a", LIMIT)
                .tile("b", LIMIT)
                .into_any_element(),
            8 => RadarChart::new("radar", ["a", "b", "c"])
                .series(Series::new("s", [LIMIT; 3]))
                .into_any_element(),
            9 => LineChart::new("rule", ["a", "b"])
                .series(Series::new("s", [1.0, 2.0]))
                .rule(LIMIT, "target")
                .into_any_element(),
            10 => BoxPlot::new("box")
                .group("s", [-LIMIT, LIMIT])
                .into_any_element(),
            11 => CalendarHeatmap::new("calendar", date(9999, 12, 31), [1.0]).into_any_element(),
            12 => ChordDiagram::new("tiny", ["a", "b"], vec![vec![0.0, 1e-40], vec![1e-40, 0.0]])
                .into_any_element(),
            13 => LineChart::new("far", ["a", "b"])
                .series(Series::new("s", [1e-20, 2e-20]))
                .rule(LIMIT, "target")
                .into_any_element(),
            14 => ChordDiagram::new(
                "least",
                ["a", "b"],
                vec![vec![0.0, 1e-310], vec![1e-310, 0.0]],
            )
            .into_any_element(),
            15 => Treemap::new("least-tiles")
                .tile("a", 1e-310)
                .tile("b", 1e-310)
                .into_any_element(),
            16 => SankeyChart::new("least-flow", ["a", "b", "c"])
                .link(0, 1, 1e-310)
                .link(1, 2, 1e-310)
                .into_any_element(),
            _ => LineChart::new("top", ["a", "b"])
                .series(top)
                .into_any_element(),
        };
        div().w(px(400.0)).h(px(240.0)).child(chart)
    }
}

#[gpui::test]
fn every_chart_draws_values_at_its_limit(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    for case in 0..=17 {
        let (_, cx) = cx.add_window_view(move |_, _| Edge(case));
        settle(cx);
    }
}

/// A line that turns from the top limit to the bottom one, and a goal from far past its target back to twice it.
struct Turning(Rc<Cell<bool>>);

impl Render for Turning {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let turned = self.0.get();
        let value = if turned { -LIMIT } else { LIMIT };
        let (goal, target) = if turned { (2.0, 1.0) } else { (LIMIT, 1e-30) };
        div()
            .w(px(400.0))
            .child(LineChart::new("line", ["a", "b"]).series(Series::new("s", [value, value])))
            .child(ProgressChart::new("progress").goal("g", goal, target))
    }
}

#[gpui::test]
fn a_chart_glides_between_its_limits(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let turned = Rc::new(Cell::new(false));
    let seen = turned.clone();
    let (view, cx) = cx.add_window_view(|_, _| Turning(seen));
    settle(cx);
    turned.set(true);
    view.update(cx, |_, cx| cx.notify());
    settle(cx);
}

#[test]
#[should_panic(expected = "within charts::LIMIT")]
fn a_value_past_the_limit_is_refused() {
    let _ = Series::new("s", [LIMIT * 10.0]);
}

#[test]
#[should_panic(expected = "within charts::LIMIT")]
fn a_band_past_the_limit_is_refused() {
    let _ = Bullet::new("b", 1.0, 2.0).bands([LIMIT * 10.0]);
}

/// A smooth line in a box barely wider than its gutter.
struct Sliver;

impl Render for Sliver {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let chart = cx.theme().chart();
        let rem = window.rem_size();
        let width = chart.gutter.to_pixels(rem) + chart.inset.to_pixels(rem) + px(0.000015);
        div().w(width).h(px(240.0)).child(
            LineChart::new("sliver", ["a", "b", "c", "d", "e"])
                .series(Series::new("s", [1.0, 2.0, 3.0, 4.0, 5.0]))
                .smooth(),
        )
    }
}

#[gpui::test]
fn a_smooth_line_in_a_sliver_draws(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| Sliver);
    settle(cx);
}

/// A centered sankey that shrinks under a resting pointer.
struct Shrinking(Rc<Cell<bool>>);

impl Render for Shrinking {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let width = if self.0.get() { 20.0 } else { 400.0 };
        div()
            .w(px(400.0))
            .h(px(240.0))
            .flex()
            .flex_col()
            .items_center()
            .child(
                SankeyChart::new("shrinking", ["a", "b"])
                    .link(0, 1, 5.0)
                    .w(px(width))
                    .h(px(240.0)),
            )
    }
}

#[gpui::test]
fn a_hovered_sankey_that_shrinks_drops_its_hover(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let shrunk = Rc::new(Cell::new(false));
    let seen = shrunk.clone();
    let (view, cx) = cx.add_window_view(|_, _| Shrinking(seen));
    settle(cx);
    cx.simulate_mouse_move(point(px(200.0), px(120.0)), None, Modifiers::none());
    settle(cx);
    shrunk.set(true);
    view.update(cx, |_, cx| cx.notify());
    settle(cx);
}

const LEAST: f64 = 1e-310;

/// Every other chart fed the smallest values it takes.
struct Least(u8);

impl Render for Least {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let tiny = || Series::new("s", [LEAST, LEAST * 2.0, LEAST]);
        let chart: AnyElement = match self.0 {
            0 => PieChart::new("pie")
                .slice("a", LEAST)
                .slice("b", LEAST)
                .into_any_element(),
            1 => FunnelChart::new("funnel")
                .stage("a", LEAST)
                .stage("b", LEAST)
                .into_any_element(),
            2 => ParallelCoordinates::new("parallel", ["a", "b"])
                .record("r", [LEAST, LEAST * 2.0])
                .record("q", [LEAST * 2.0, LEAST])
                .into_any_element(),
            3 => BulletChart::new("bullets")
                .bullet(Bullet::new("b", LEAST, LEAST * 2.0).bands([LEAST, LEAST * 3.0]))
                .into_any_element(),
            4 => ProgressChart::new("goals")
                .goal("g", LEAST, LEAST)
                .into_any_element(),
            5 => HeatmapChart::new("heat", ["a", "b"])
                .row("r", [LEAST, LEAST * 2.0])
                .into_any_element(),
            6 => Histogram::new("histogram", [LEAST, LEAST * 2.0, LEAST * 3.0]).into_any_element(),
            7 => BoxPlot::new("box")
                .group("s", [LEAST, LEAST * 2.0, LEAST * 4.0])
                .into_any_element(),
            8 => ViolinPlot::new("violin")
                .group("s", [LEAST, LEAST * 2.0])
                .into_any_element(),
            9 => WaterfallChart::new("waterfall")
                .step("a", LEAST)
                .total("all")
                .into_any_element(),
            10 => RadarChart::new("radar", ["a", "b", "c"])
                .series(tiny())
                .into_any_element(),
            11 => Sunburst::new(
                "sunburst",
                [Slice::new("root", LEAST).children([Slice::new("a", LEAST)])],
            )
            .into_any_element(),
            12 => AreaChart::new("area", ["a", "b", "c"])
                .series(tiny())
                .stacked()
                .into_any_element(),
            _ => BarChart::new("bars", ["a", "b", "c"])
                .series(tiny())
                .horizontal()
                .into_any_element(),
        };
        div().w(px(400.0)).h(px(240.0)).child(chart)
    }
}

#[gpui::test]
fn every_chart_draws_its_smallest_values(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    for case in 0..=13 {
        let (_, cx) = cx.add_window_view(move |_, _| Least(case));
        settle(cx);
    }
}
