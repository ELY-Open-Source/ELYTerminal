use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, Hsla, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};
use jiff::tz::TimeZone;

use super::{
    book::{Side, Trade, cell, heading, level},
    quotes::{moves, price},
};
use crate::{
    charts::compact,
    motion::Flash,
    theme::{ActiveTheme, Density, TextSize},
    typography::{
        format::{datetime, decimals, system_zone},
        tabular,
    },
};

/// Trades as they print, newest on top: the time, price and size tinted by the side that took them; trades of at least `large` stand bold, and the newest flashes in.
#[derive(IntoElement)]
pub struct TimeAndSales {
    id: ElementId,
    trades: Vec<Trade>,
    large: f64,
    rows: usize,
    places: usize,
    zone: Option<TimeZone>,
    red_up: bool,
}

impl TimeAndSales {
    /// Trades oldest first.
    pub fn new(id: impl Into<ElementId>, trades: impl IntoIterator<Item = Trade>) -> Self {
        Self {
            id: id.into(),
            trades: trades.into_iter().collect(),
            large: f64::MAX,
            rows: 12,
            places: 2,
            zone: None,
            red_up: false,
        }
    }

    /// The size from which a trade reads bold.
    pub fn large(mut self, size: f64) -> Self {
        self.large = size;
        self
    }

    pub fn rows(mut self, rows: usize) -> Self {
        self.rows = rows;
        self
    }

    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }

    pub fn red_up(mut self) -> Self {
        self.red_up = true;
        self
    }
}

impl RenderOnce for TimeAndSales {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let zone = self
            .zone
            .clone()
            .unwrap_or_else(|| system_zone("TimeAndSales"));
        let (rise, fall) = moves(self.red_up, cx);
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let rows = self
            .trades
            .iter()
            .rev()
            .take(self.rows)
            .enumerate()
            .map(|(ix, trade)| {
                let ink = if trade.side == Side::Buy { rise } else { fall };
                let time = datetime(trade.time, &zone, "%H:%M:%S").expect("a fixed pattern");
                let row = div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_2()
                    .h(theme.table_row(Density::Compact))
                    .text_size(theme.text_size(TextSize::Sm))
                    .when(trade.size >= self.large, |row| {
                        row.font_weight(FontWeight::SEMIBOLD)
                    })
                    .child(
                        tabular(div())
                            .flex_1()
                            .text_right()
                            .text_color(colors.fg_muted)
                            .child(time),
                    )
                    .child(cell(price(trade.price, self.places), ink))
                    .child(cell(compact(trade.size), colors.fg));
                if ix == 0 {
                    Flash::new((self.id.clone(), "newest"), *trade)
                        .child(row)
                        .into_any_element()
                } else {
                    row.into_any_element()
                }
            });
        div()
            .flex()
            .flex_col()
            .child(heading(&["Time", "Price", "Size"], cx))
            .children(rows)
    }
}

type OnTrade = Rc<dyn Fn(Side, f64, &mut Window, &mut App)>;

/// The book as a ladder of prices around the last trade: size bid at each price on the left, asked on the right, the last price marked in the middle. A press on a size side trades at its price.
#[derive(IntoElement)]
pub struct DomLadder {
    id: ElementId,
    bids: Vec<(f64, f64)>,
    asks: Vec<(f64, f64)>,
    last: f64,
    tick: f64,
    rows: usize,
    places: usize,
    on_trade: Option<OnTrade>,
    red_up: bool,
}

impl DomLadder {
    /// Bids and asks, each a price and the size waiting there; the last trade's price, and the price step between rows.
    pub fn new(
        id: impl Into<ElementId>,
        bids: impl IntoIterator<Item = (f64, f64)>,
        asks: impl IntoIterator<Item = (f64, f64)>,
        last: f64,
        tick: f64,
    ) -> Self {
        assert!(
            tick > 0.0 && last.is_finite(),
            "a ladder needs a step and a last price"
        );
        Self {
            id: id.into(),
            bids: bids.into_iter().collect(),
            asks: asks.into_iter().collect(),
            last,
            tick,
            rows: 15,
            places: 2,
            on_trade: None,
            red_up: false,
        }
    }

    pub fn rows(mut self, rows: usize) -> Self {
        self.rows = rows;
        self
    }

    /// Gets a buy at a bid's price or a sell at an ask's, from a press on its size.
    pub fn on_trade(
        mut self,
        handler: impl Fn(Side, f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_trade = Some(Rc::new(handler));
        self
    }

    pub fn red_up(mut self) -> Self {
        self.red_up = true;
        self
    }
}

/// The prices a ladder shows around `last`, highest first, each on a whole step when the step can say it.
fn rungs(last: f64, tick: f64, rows: usize) -> Vec<f64> {
    let steps = (last / tick).round();
    let middle = if steps.is_finite() {
        steps * tick
    } else {
        last
    };
    let above = (rows / 2) as f64;
    (0..rows)
        .map(|ix| middle + (above - ix as f64) * tick)
        .collect()
}

impl RenderOnce for DomLadder {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let places = self.places.max(decimals(self.tick));
        let (rise, fall) = moves(self.red_up, cx);
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let near = |side: &[(f64, f64)], at: f64| {
            side.iter()
                .find(|(level, _)| (level - at).abs() <= self.tick / 2.0)
                .map(|(_, size)| *size)
        };
        let deepest = self
            .bids
            .iter()
            .chain(&self.asks)
            .map(|(_, size)| *size)
            .fold(0.0, f64::max);
        let middle = self.rows / 2;
        let rows = rungs(self.last, self.tick, self.rows)
            .into_iter()
            .enumerate()
            .map(|(ix, at)| {
                let (bid, ask) = (near(&self.bids, at), near(&self.asks, at));
                let side_cell = |size: Option<f64>, side: Side, ink: Hsla| {
                    let key = format!("{at}-{ix}-{side:?}");
                    let body = level(
                        size.unwrap_or(0.0) / deepest.max(f64::EPSILON),
                        ink,
                        side == Side::Buy,
                        cx,
                    )
                    .flex_1()
                    .child(cell(size.map(compact).unwrap_or_default(), colors.fg));
                    let trade = self.on_trade.clone();
                    div()
                        .id((self.id.clone(), SharedString::from(key)))
                        .debug_selector(move || format!("ladder-cell-{ix}-{side:?}"))
                        .flex_1()
                        .cursor_pointer()
                        .hover(|style| style.bg(colors.hover))
                        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                        .when_some(trade, |cell, trade| {
                            cell.on_click(move |_, window, cx| {
                                log::info!("dom ladder: {side:?} at {at}");
                                trade(side, at, window, cx)
                            })
                        })
                        .child(body)
                };
                let marked = ix == middle;
                div()
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(colors.border.opacity(0.5))
                    .child(side_cell(bid, Side::Buy, rise))
                    .child(
                        tabular(div())
                            .w(theme.label_width() * 0.6)
                            .text_center()
                            .text_size(theme.text_size(TextSize::Sm))
                            .when(marked, |price| {
                                price
                                    .debug_selector(|| "ladder-last".into())
                                    .bg(colors.active)
                                    .text_color(colors.fg)
                                    .font_weight(FontWeight::SEMIBOLD)
                            })
                            .when(!marked, |price| price.text_color(colors.fg_muted))
                            .child(price(at, places)),
                    )
                    .child(side_cell(ask, Side::Sell, fall))
            });
        div()
            .flex()
            .flex_col()
            .rounded(theme.radius(crate::theme::Radius::Md))
            .border_1()
            .border_color(colors.border)
            .overflow_hidden()
            .children(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::rungs;

    #[test]
    fn a_tick_finer_than_the_price_repeats_rungs() {
        for tick in [1e-18, 1e-310] {
            let prices = rungs(10.0, tick, 15);
            assert_eq!(prices.len(), 15);
            assert!(
                prices.iter().all(|price| (price - 10.0).abs() < 1e-9),
                "{prices:?}"
            );
        }
    }

    #[test]
    fn rungs_step_around_the_last() {
        let prices = rungs(100.02, 0.05, 5);
        assert_eq!(prices.len(), 5);
        assert!(
            (prices[2] - 100.0).abs() < 1e-9,
            "the middle rung is the last price on a whole step"
        );
        assert!(prices[0] > prices[4], "highest first");
    }
}
