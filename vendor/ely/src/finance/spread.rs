use gpui::{App, IntoElement, ParentElement, RenderOnce, Styled, Window, div, relative};

use super::quotes::{moves, price};
use crate::{
    theme::{ActiveTheme, Radius, TextSize},
    typography::{format, tabular},
};

/// The best bid and ask, and the spread between them in price and in basis points.
#[derive(IntoElement)]
pub struct SpreadIndicator {
    bid: f64,
    ask: f64,
    places: usize,
    red_up: bool,
}

impl SpreadIndicator {
    pub fn new(bid: f64, ask: f64) -> Self {
        assert!(
            bid.is_finite() && ask.is_finite(),
            "a spread needs finite prices"
        );
        Self {
            bid,
            ask,
            places: 2,
            red_up: false,
        }
    }

    pub fn places(mut self, places: usize) -> Self {
        self.places = places;
        self
    }

    pub fn red_up(mut self) -> Self {
        self.red_up = true;
        self
    }
}

impl RenderOnce for SpreadIndicator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (rise, fall) = moves(self.red_up, cx);
        let theme = cx.theme();
        let colors = &theme.colors;
        let side = |name: &'static str, value: f64, ink| {
            div()
                .flex()
                .items_baseline()
                .gap_1p5()
                .child(div().text_color(colors.fg_subtle).child(name))
                .child(
                    tabular(div())
                        .text_color(ink)
                        .child(price(value, self.places)),
                )
        };
        let spread = self.ask - self.bid;
        let points = spread / ((self.ask + self.bid) / 2.0) * 10_000.0;
        div()
            .flex()
            .items_center()
            .gap_4()
            .text_size(theme.text_size(TextSize::Sm))
            .child(side("Bid", self.bid, rise))
            .child(
                tabular(div())
                    .px_2()
                    .py_0p5()
                    .rounded(theme.radius(Radius::Sm))
                    .bg(colors.hover)
                    .text_color(colors.fg_muted)
                    .child(format!("{} · {points:.1} bps", price(spread, self.places))),
            )
            .child(side("Ask", self.ask, fall))
    }
}

/// The weight of waiting orders on each side as one bar split where they balance.
#[derive(IntoElement)]
pub struct BidAskBar {
    bids: f64,
    asks: f64,
    red_up: bool,
}

impl BidAskBar {
    /// The size waiting on each side.
    pub fn new(bids: f64, asks: f64) -> Self {
        assert!(bids >= 0.0 && asks >= 0.0, "sizes are zero or more");
        Self {
            bids,
            asks,
            red_up: false,
        }
    }

    pub fn red_up(mut self) -> Self {
        self.red_up = true;
        self
    }
}

impl RenderOnce for BidAskBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (rise, fall) = moves(self.red_up, cx);
        let theme = cx.theme();
        let total = self.bids + self.asks;
        let share = if total > 0.0 { self.bids / total } else { 0.5 };
        let words = |name: &'static str, value: f64| {
            let shown = match total > 0.0 {
                true => format::percent(value, 0, false),
                false => "—".to_string(),
            };
            tabular(div())
                .text_color(theme.colors.fg_muted)
                .child(format!("{name} {shown}"))
        };
        let part = |grow: f64, ink| {
            let mut part = div().h_full().flex_basis(relative(0.0)).bg(ink);
            part.style().flex_grow = Some(grow as f32);
            part
        };
        div()
            .flex()
            .flex_col()
            .gap_1()
            .text_size(theme.text_size(TextSize::Xs))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .child(words("Bids", share))
                    .child(words("Asks", 1.0 - share)),
            )
            .child(
                div()
                    .flex()
                    .gap_0p5()
                    .h(theme.meter_track())
                    .rounded_full()
                    .overflow_hidden()
                    .child(part(share, rise).rounded_l_full())
                    .child(part(1.0 - share, fall).rounded_r_full()),
            )
    }
}
