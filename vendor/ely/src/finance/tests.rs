use std::rc::Rc;

use gpui::{
    AnyElement, AppContext as _, Context, Entity, IntoElement, ParentElement, Render, Styled,
    TestAppContext, VisualTestContext, Window, div, px,
};
use jiff::Timestamp;

use crate::finance::{
    Arrangement, BidAskBar, CandlestickChart, ChartSync, CryptoWalletCard, CurrencyConverter,
    DepthChart, DomLadder, IntervalSelector, Leg, LeverageSlider, MarginIndicator, MarketHeatmap,
    MultiChartLayout, PayoffDiagram, PerformanceChart, PointFigureChart, QuickTradeButtons,
    RenkoChart, SpreadIndicator, TimeRangeSelector, candles::Candle, stage::Visible,
};
use crate::{layout::tests::narrow_width, theme::Theme};

fn candles(count: usize) -> Rc<Vec<Candle>> {
    Rc::new(
        (0..count)
            .map(|ix| {
                let time = Timestamp::from_second(ix as i64 * 86_400).expect("a day");
                Candle::new(time, (10.0, 11.0, 9.0, 10.0 + ix as f64 * 0.01), 100.0)
            })
            .collect(),
    )
}

struct Pair {
    first: Rc<Vec<Candle>>,
    second: Rc<Vec<Candle>>,
    sync: Entity<ChartSync>,
}

impl Render for Pair {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(800.))
            .child(CandlestickChart::new("first", self.first.clone()).sync(&self.sync))
            .child(CandlestickChart::new("second", self.second.clone()).sync(&self.sync))
    }
}

/// Two synced charts of 40 candles, their shared window held on the newest 20.
fn held(cx: &mut TestAppContext) -> (Entity<Pair>, Entity<ChartSync>, &mut VisualTestContext) {
    cx.update(Theme::init);
    let sync = cx.new(|_| ChartSync::default());
    let shared = sync.clone();
    let (view, cx) = cx.add_window_view(|_, _| Pair {
        first: candles(40),
        second: candles(40),
        sync: shared,
    });
    cx.run_until_parked();
    sync.update(cx, |sync, cx| {
        sync.visible = Some(Visible {
            start: 20.0,
            count: 20.0,
        });
        cx.notify();
    });
    view.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
    (view, sync, cx)
}

fn start(sync: &Entity<ChartSync>, cx: &mut VisualTestContext) -> f64 {
    sync.read_with(cx, |sync, _| sync.visible.expect("held").start)
}

#[gpui::test]
fn synced_charts_move_their_shared_window_once_per_append(cx: &mut TestAppContext) {
    let (view, sync, cx) = held(cx);
    view.update(cx, |pair, cx| {
        pair.first = candles(45);
        pair.second = candles(45);
        cx.notify();
    });
    cx.run_until_parked();
    assert_eq!(
        start(&sync, cx),
        25.0,
        "five new candles move the shared window five, not ten"
    );
}

#[gpui::test]
fn a_lagging_chart_leaves_the_shared_window_alone(cx: &mut TestAppContext) {
    let (view, sync, cx) = held(cx);
    view.update(cx, |pair, cx| {
        pair.second = candles(45);
        cx.notify();
    });
    cx.run_until_parked();
    for _ in 0..2 {
        view.update(cx, |_, cx| cx.notify());
        cx.run_until_parked();
    }
    assert_eq!(
        (start(&sync, cx), sync.read_with(cx, |sync, _| sync.seen)),
        (25.0, 45),
        "redraws while the first chart lags move nothing"
    );
    view.update(cx, |pair, cx| {
        pair.first = candles(45);
        cx.notify();
    });
    cx.run_until_parked();
    assert_eq!(
        start(&sync, cx),
        25.0,
        "the lagging chart's append is already shown"
    );
}

#[gpui::test]
fn every_chart_fills_a_column_its_block_measures_by_content(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let leg = Leg {
        call: true,
        strike: 100.0,
        quantity: 1.0,
        premium: 2.0,
    };
    let widths = [
        narrow_width(cx, "chart-root", |_, _| {
            DepthChart::new("depth", [(99.0, 2.0)], [(101.0, 3.0)]).into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            MarketHeatmap::new("heatmap")
                .tile("A", 1.0, 0.5)
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            CandlestickChart::new("candles", candles(30)).into_any_element()
        }),
        narrow_width(cx, "chart-root", move |_, _| {
            PayoffDiagram::new("payoff", [leg], (80.0, 120.0), 100.0).into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            RenkoChart::new("renko", candles(30), 0.1).into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            PointFigureChart::new("figure", candles(30), 0.1, 3).into_any_element()
        }),
    ];
    assert_eq!(
        widths,
        [px(240.0); 6],
        "each chart spans the card inside its padding"
    );
}

/// A converter squeezed beside a card in a 280px row.
struct Squeezed;

impl Render for Squeezed {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let converter = CurrencyConverter::new("converter", 1000.0, ("USD", "EUR"), 0.9174);
        div().w(px(280.)).child(
            div()
                .flex()
                .gap_8()
                .child(div().w(px(460.)).child(converter))
                .child(div().flex_1().child(div().w(px(260.)).h(px(100.)))),
        )
    }
}

#[gpui::test]
fn a_squeezed_converter_keeps_room_for_its_currencies(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| Squeezed);
    cx.run_until_parked();
    let text = cx
        .debug_bounds("input-text")
        .expect("the second currency's text");
    assert_eq!(text.size.width, px(26.), "the frame's inset for its name");
}

fn leg() -> Leg {
    Leg {
        call: true,
        strike: 1e-200,
        quantity: 1.0,
        premium: 0.0,
    }
}

/// Candles whose prices are each of `closes`.
fn scaled(closes: &[f64]) -> Rc<Vec<Candle>> {
    Rc::new(
        closes
            .iter()
            .enumerate()
            .map(|(ix, close)| {
                let time = Timestamp::from_second(ix as i64 * 86_400).expect("a day");
                Candle::new(time, (*close, *close, *close, *close), 1.0)
            })
            .collect(),
    )
}

/// Market states a live feed reaches: a crossed book, no size, margin past equity, a lagging comparison.
struct Live(u8);

impl Render for Live {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let chart: AnyElement = match self.0 {
            0 => DepthChart::new("crossed", [(101.0, 2.0)], [(100.0, 3.0)]).into_any_element(),
            1 => SpreadIndicator::new(101.0, 100.0).into_any_element(),
            2 => QuickTradeButtons::new("locked", 100.0, 100.0).into_any_element(),
            3 => BidAskBar::new(0.0, 0.0).into_any_element(),
            4 => LeverageSlider::new("leverage", 40.0, 20.0).into_any_element(),
            5 => MarginIndicator::new("margin", 500.0, -200.0).into_any_element(),
            6 => TimeRangeSelector::new("range", "2Y5D").into_any_element(),
            7 => IntervalSelector::new("interval", "7s").into_any_element(),
            8 => MultiChartLayout::new("layout", Arrangement::Four)
                .cell(div())
                .into_any_element(),
            9 => CandlestickChart::new("compare", candles(20))
                .compare("other", candles(12))
                .into_any_element(),
            10 => {
                PerformanceChart::new("zero", ["a", "b", "c"], [0.0, 10.0, 12.0]).into_any_element()
            }
            11 => PerformanceChart::new("lagging", ["a", "b", "c"], [10.0, 11.0, 12.0])
                .benchmark("index", [1.0, 2.0])
                .into_any_element(),
            12 => DepthChart::new(
                "crossed-deep",
                [(101.0, 1.0), (100.75, 1.0)],
                [(100.0, 1.0), (100.25, 1.0)],
            )
            .into_any_element(),
            13 => PerformanceChart::new("negative", ["a", "b", "c"], [-1.0, 1e-310, -1.0])
                .into_any_element(),
            14 => CandlestickChart::new("far", candles(2))
                .compare("other", scaled(&[1e-299, 1e30]))
                .into_any_element(),
            15 => PayoffDiagram::new("narrow", [leg()], (1e-200, 1.000000000000001e-200), 1e-200)
                .into_any_element(),
            16 => PointFigureChart::new("fine", candles(3), 1e-18, 3).into_any_element(),
            17 => RenkoChart::new("fine-bricks", candles(3), 1e-18).into_any_element(),
            18 => DomLadder::new("fine-ladder", [(10.0, 1.0)], [(10.1, 1.0)], 10.0, 1e-18)
                .into_any_element(),
            _ => CryptoWalletCard::new(
                "wallet",
                ("Coin", "Chain"),
                (1.0, "BTC"),
                (1.0, "USD"),
                "x".repeat(8_000),
            )
            .into_any_element(),
        };
        div().w(px(480.0)).h(px(320.0)).child(chart)
    }
}

#[gpui::test]
fn live_market_states_draw(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    for case in 0..=19 {
        let (_, cx) = cx.add_window_view(move |_, _| Live(case));
        cx.run_until_parked();
    }
}

/// A box too thin for a pixel, and a comparison its scale cannot hold.
struct Thin(bool);

impl Render for Thin {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let chart: AnyElement = match self.0 {
            true => PointFigureChart::new("thin", scaled(&[100_000.0]), 1e-5, 3).into_any_element(),
            false => CandlestickChart::new("unheld", candles(2))
                .compare("retired", scaled(&[1e-299, 1e30]))
                .into_any_element(),
        };
        div().w(px(480.0)).h(px(320.0)).child(chart)
    }
}

#[gpui::test]
fn boxes_thinner_than_a_pixel_draw_as_a_block(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| Thin(true));
    for _ in 0..3 {
        cx.run_until_parked();
        cx.update(|window, _| window.refresh());
    }
    cx.run_until_parked();
}

#[gpui::test]
fn a_comparison_left_out_names_nothing(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| Thin(false));
    cx.run_until_parked();
    assert!(cx.debug_bounds("chart-compared").is_none());
}

/// A ladder at a tick, last price and book; or a depth book over a subnormal span.
struct Fine(Option<(f64, f64)>);

impl Render for Fine {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let chart: AnyElement = match self.0 {
            Some((last, tick)) => {
                DomLadder::new("ladder", [(last, 1.0)], [(last + 1.0, 1.0)], last, tick)
                    .into_any_element()
            }
            None => DepthChart::new("subnormal", [(0.0, 1.0)], [(3e-323, 1.0)]).into_any_element(),
        };
        div().w(px(480.0)).h(px(320.0)).child(chart)
    }
}

#[gpui::test]
fn a_ladder_marks_its_last_price_at_any_tick(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    for (last, tick) in [
        (10.0, 0.05),
        (10.0, 1e-18),
        (10.0, 1e-310),
        (0.0, f64::from_bits(1)),
    ] {
        let (_, cx) = cx.add_window_view(move |_, _| Fine(Some((last, tick))));
        cx.run_until_parked();
        assert!(cx.debug_bounds("ladder-last").is_some(), "{last} at {tick}");
    }
}

#[gpui::test]
fn a_depth_book_over_a_subnormal_span_draws(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| Fine(None));
    cx.run_until_parked();
}

/// A ladder that keeps each trade it hears.
struct Trading(Rc<std::cell::RefCell<Vec<f64>>>, f64);

impl Render for Trading {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let heard = self.0.clone();
        div().w(px(480.0)).child(
            DomLadder::new("ladder", [(10.0, 1.0)], [(10.1, 1.0)], 10.0, self.1)
                .on_trade(move |_, at, _, _| heard.borrow_mut().push(at)),
        )
    }
}

#[gpui::test]
fn every_rung_trades_even_where_prices_repeat(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    for tick in [0.05, 1e-18] {
        let heard = Rc::new(std::cell::RefCell::new(Vec::new()));
        let seen = heard.clone();
        let (_, cx) = cx.add_window_view(move |_, _| Trading(seen, tick));
        cx.run_until_parked();
        for row in [
            "ladder-cell-0-Buy",
            "ladder-cell-7-Buy",
            "ladder-cell-14-Buy",
        ] {
            heard.borrow_mut().clear();
            let at = cx.debug_bounds(row).expect("a cell").center();
            cx.simulate_mouse_move(at, None, gpui::Modifiers::none());
            cx.simulate_click(at, gpui::Modifiers::none());
            cx.run_until_parked();
            assert_eq!(heard.borrow().len(), 1, "tick {tick}, row {row}");
        }
    }
}
