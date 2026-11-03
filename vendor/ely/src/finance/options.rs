use std::rc::Rc;

use gpui::{
    App, Div, ElementId, FontWeight, Hsla, InteractiveElement, IntoElement, MouseButton,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::*,
};

use super::quotes::price;
use crate::{
    charts::compact,
    theme::{ActiveTheme, Density, Radius, TextSize},
    typography::{format, tabular},
};

/// One side of a strike: its bid, ask, volume, and implied volatility as a share.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OptionQuote {
    pub bid: f64,
    pub ask: f64,
    pub volume: f64,
    pub volatility: f64,
}

/// A strike and its call and put.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Strike {
    pub strike: f64,
    pub call: OptionQuote,
    pub put: OptionQuote,
}

type OnPick = Rc<dyn Fn(f64, bool, &mut Window, &mut App)>;

/// Calls and puts across strikes: calls to the left, puts to the right, strikes between; the side in the money is shaded and a line marks where the price stands. A press on a quote picks its option.
#[derive(IntoElement)]
pub struct OptionChain {
    id: ElementId,
    strikes: Vec<Strike>,
    spot: f64,
    on_pick: Option<OnPick>,
}

impl OptionChain {
    pub fn new(
        id: impl Into<ElementId>,
        strikes: impl IntoIterator<Item = Strike>,
        spot: f64,
    ) -> Self {
        let mut strikes: Vec<Strike> = strikes.into_iter().collect();
        strikes.sort_by(|a, b| a.strike.total_cmp(&b.strike));
        Self {
            id: id.into(),
            strikes,
            spot,
            on_pick: None,
        }
    }

    /// Gets a strike and whether the call, rather than the put, was pressed.
    pub fn on_pick(mut self, handler: impl Fn(f64, bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }
}

/// A side's four numbers as cells.
fn quote_cells(quote: &OptionQuote, ink: Hsla) -> [Div; 4] {
    let number = |words: String, color: Hsla| {
        tabular(div())
            .flex_1()
            .text_right()
            .text_color(color)
            .child(words)
    };
    [
        number(price(quote.bid, 2), ink),
        number(price(quote.ask, 2), ink),
        number(compact(quote.volume), ink),
        number(format::percent(quote.volatility, 1, false), ink),
    ]
}

impl RenderOnce for OptionChain {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let tall = theme.table_row(Density::Compact);
        let small = theme.text_size(TextSize::Xs);
        let names =
            |names: [&'static str; 4]| names.map(|name| div().flex_1().text_right().child(name));
        let header = div()
            .flex()
            .gap_2()
            .px_2()
            .pb_1()
            .text_size(small)
            .text_color(colors.fg_subtle)
            .child(
                div()
                    .flex_1()
                    .flex()
                    .gap_2()
                    .children(names(["Bid", "Ask", "Volume", "IV"])),
            )
            .child(
                div()
                    .w(theme.label_width() * 0.5)
                    .text_center()
                    .child("Strike"),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .gap_2()
                    .children(names(["Bid", "Ask", "Volume", "IV"])),
            );
        let above = self
            .strikes
            .iter()
            .position(|strike| strike.strike > self.spot)
            .unwrap_or(self.strikes.len());
        let mut rows: Vec<gpui::AnyElement> = Vec::new();
        for (ix, strike) in self.strikes.iter().enumerate() {
            if ix == above {
                let pill = tabular(div())
                    .px_1p5()
                    .rounded(theme.radius(Radius::Sm))
                    .bg(colors.fg)
                    .text_color(colors.bg)
                    .text_size(small)
                    .child(price(self.spot, 2));
                let rule = || div().flex_1().h(theme.chart().hairline).bg(colors.fg);
                rows.push(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(rule())
                        .child(pill)
                        .child(rule())
                        .into_any_element(),
                );
            }
            let half = |call: bool| {
                let quote = if call { &strike.call } else { &strike.put };
                let money = if call {
                    strike.strike < self.spot
                } else {
                    strike.strike > self.spot
                };
                let pick = self.on_pick.clone();
                let at = strike.strike;
                div()
                    .id((
                        self.id.clone(),
                        SharedString::from(format!("{at}-{}", if call { "call" } else { "put" })),
                    ))
                    .flex_1()
                    .flex()
                    .gap_2()
                    .px_2()
                    .h_full()
                    .items_center()
                    .when(money, |half| half.bg(colors.hover))
                    .cursor_pointer()
                    .hover(|style| style.bg(colors.active))
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .when_some(pick, |half, pick| {
                        half.on_click(move |_, window, cx| {
                            log::info!("option chain: {} {at}", if call { "call" } else { "put" });
                            pick(at, call, window, cx)
                        })
                    })
                    .children(quote_cells(quote, colors.fg))
            };
            rows.push(
                div()
                    .flex()
                    .h(tall)
                    .text_size(theme.text_size(TextSize::Sm))
                    .border_b_1()
                    .border_color(colors.border.opacity(0.5))
                    .child(half(true))
                    .child(
                        tabular(div())
                            .w(theme.label_width() * 0.5)
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.fg)
                            .child(price(strike.strike, 2)),
                    )
                    .child(half(false))
                    .into_any_element(),
            );
        }
        div().flex().flex_col().child(header).children(rows)
    }
}

/// An option's or a book's greeks: how its price answers to the underlying, time, volatility and rates.
#[derive(IntoElement)]
pub struct GreeksTable {
    greeks: [f64; 5],
}

impl GreeksTable {
    /// Delta, gamma, theta, vega and rho.
    pub fn new(greeks: [f64; 5]) -> Self {
        assert!(
            greeks.iter().all(|greek| greek.is_finite()),
            "greeks are finite"
        );
        Self { greeks }
    }
}

const GREEKS: [(&str, &str, &str); 5] = [
    (
        "Δ",
        "Delta",
        "Price change for each dollar the underlying moves",
    ),
    (
        "Γ",
        "Gamma",
        "Delta change for each dollar the underlying moves",
    ),
    ("Θ", "Theta", "Value lost each day as expiry nears"),
    ("ν", "Vega", "Price change for each point of volatility"),
    ("ρ", "Rho", "Price change for each point of interest rates"),
];

impl RenderOnce for GreeksTable {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        div()
            .flex()
            .flex_col()
            .children(
                GREEKS
                    .iter()
                    .zip(self.greeks)
                    .map(|((symbol, name, meaning), value)| {
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .py_1p5()
                            .border_b_1()
                            .border_color(colors.border.opacity(0.5))
                            .text_size(theme.text_size(TextSize::Sm))
                            .child(
                                div()
                                    .w(theme.icon_size(crate::theme::IconSize::Md))
                                    .text_color(colors.fg_subtle)
                                    .child(*symbol),
                            )
                            .child(
                                div()
                                    .w(theme.label_width() * 0.5)
                                    .text_color(colors.fg)
                                    .child(*name),
                            )
                            .child(
                                tabular(div())
                                    .w(theme.label_width() * 0.5)
                                    .text_right()
                                    .text_color(colors.fg)
                                    .child(format::number(value, 4, format::Separators::EN)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .pl_3()
                                    .text_size(theme.text_size(TextSize::Xs))
                                    .text_color(colors.fg_muted)
                                    .child(*meaning),
                            )
                    }),
            )
    }
}
