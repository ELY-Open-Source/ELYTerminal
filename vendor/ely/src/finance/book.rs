use gpui::{
    App, Div, ElementId, FontWeight, Hsla, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div, relative,
};
use jiff::Timestamp;

use super::{
    depth::depth,
    quotes::{moves, price},
};
use crate::{
    charts::compact,
    motion::Flash,
    theme::{ActiveTheme, Density, TextSize},
    typography::{format, tabular},
};

/// Which side of the book took a trade, or wants to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

/// One trade as it printed: when, at what price, how much, and which side took it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trade {
    pub time: Timestamp,
    pub price: f64,
    pub size: f64,
    pub side: Side,
}

/// A row of a book: a bar behind it as wide as its share, and its cells.
pub(super) fn level(share: f64, ink: Hsla, from_right: bool, cx: &App) -> Div {
    let theme = cx.theme();
    let bar = div()
        .absolute()
        .top_0()
        .bottom_0()
        .w(relative(share.clamp(0.0, 1.0) as f32))
        .bg(ink.opacity(0.1));
    div()
        .relative()
        .flex()
        .items_center()
        .gap_3()
        .px_2()
        .h(theme.table_row(Density::Compact))
        .text_size(theme.text_size(TextSize::Sm))
        .child(if from_right {
            bar.right_0()
        } else {
            bar.left_0()
        })
}

/// A column of numbers set right.
pub(super) fn cell(words: String, ink: Hsla) -> Div {
    tabular(div())
        .flex_1()
        .text_right()
        .text_color(ink)
        .child(words)
}

/// The heading over a book's columns.
pub(super) fn heading(names: &[&'static str], cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .flex()
        .gap_3()
        .px_2()
        .pb_1()
        .text_size(theme.text_size(TextSize::Xs))
        .text_color(theme.colors.fg_subtle)
        .children(
            names
                .iter()
                .map(|name| div().flex_1().text_right().child(*name)),
        )
}

/// Orders waiting to buy and to sell, best in the middle: asks above falling to the spread, bids below; each level's running total shaded as a share of the deepest shown.
#[derive(IntoElement)]
pub struct OrderBook {
    id: ElementId,
    bids: Vec<(f64, f64)>,
    asks: Vec<(f64, f64)>,
    levels: usize,
    places: usize,
    red_up: bool,
}

impl OrderBook {
    /// Bids and asks, each a price and the size waiting there.
    pub fn new(
        id: impl Into<ElementId>,
        bids: impl IntoIterator<Item = (f64, f64)>,
        asks: impl IntoIterator<Item = (f64, f64)>,
    ) -> Self {
        let (bids, asks): (Vec<_>, Vec<_>) =
            (bids.into_iter().collect(), asks.into_iter().collect());
        assert!(
            !bids.is_empty() && !asks.is_empty(),
            "a book needs both sides"
        );
        Self {
            id: id.into(),
            bids,
            asks,
            levels: 8,
            places: 2,
            red_up: false,
        }
    }

    /// How many levels each side shows.
    pub fn levels(mut self, levels: usize) -> Self {
        assert!(levels > 0, "a book shows a level");
        self.levels = levels;
        self
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

impl RenderOnce for OrderBook {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (rise, fall) = moves(self.red_up, cx);
        let (bids, asks) = depth(&self.bids, &self.asks);
        let (bids, asks) = (
            &bids[..self.levels.min(bids.len())],
            &asks[..self.levels.min(asks.len())],
        );
        let deepest = bids
            .iter()
            .chain(asks)
            .map(|(_, total)| *total)
            .fold(0.0, f64::max);
        let sizes = |side: &[(f64, f64)]| {
            side.iter()
                .scan(0.0, |before, (at, total)| {
                    let size = total - *before;
                    *before = *total;
                    Some((*at, size, *total))
                })
                .collect::<Vec<_>>()
        };
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let row = |id: SharedString, (at, size, total): (f64, f64, f64), ink: Hsla| {
            level(total / deepest.max(f64::EPSILON), ink, true, cx)
                .child(cell(price(at, self.places), ink))
                .child(
                    Flash::new(id, size.to_bits())
                        .flex_1()
                        .child(cell(compact(size), colors.fg)),
                )
                .child(cell(compact(total), colors.fg_muted))
        };
        let (best_bid, best_ask) = (bids[0].0, asks[0].0);
        let spread = best_ask - best_bid;
        let middle = div()
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .py_1p5()
            .border_t_1()
            .border_b_1()
            .border_color(colors.border)
            .child(
                tabular(div())
                    .text_size(theme.text_size(TextSize::Md))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.fg)
                    .child(price((best_bid + best_ask) / 2.0, self.places + 1)),
            )
            .child(
                tabular(div())
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.fg_muted)
                    .child(format!(
                        "Spread {} · {}",
                        price(spread, self.places),
                        format::percent(spread / best_ask, 3, false)
                    )),
            );
        let id = self.id;
        div()
            .flex()
            .flex_col()
            .child(heading(&["Price", "Size", "Total"], cx))
            .children(
                sizes(asks)
                    .into_iter()
                    .rev()
                    .map(|level| row(format!("{id:?}-ask-{}", level.0).into(), level, fall)),
            )
            .child(middle)
            .children(
                sizes(bids)
                    .into_iter()
                    .map(|level| row(format!("{id:?}-bid-{}", level.0).into(), level, rise)),
            )
    }
}

/// Quotes by venue on both sides: who bids how much at what price and who asks, best first, each price level shaded a step lighter than the better one.
#[derive(IntoElement)]
pub struct Level2Quotes {
    bids: Vec<(SharedString, f64, f64)>,
    asks: Vec<(SharedString, f64, f64)>,
    rows: usize,
    places: usize,
    red_up: bool,
}

impl Level2Quotes {
    pub fn new() -> Self {
        Self {
            bids: Vec::new(),
            asks: Vec::new(),
            rows: 8,
            places: 2,
            red_up: false,
        }
    }

    /// A venue's bid: its price and size.
    pub fn bid(mut self, venue: impl Into<SharedString>, at: f64, size: f64) -> Self {
        self.bids.push((venue.into(), at, size));
        self
    }

    pub fn ask(mut self, venue: impl Into<SharedString>, at: f64, size: f64) -> Self {
        self.asks.push((venue.into(), at, size));
        self
    }

    pub fn rows(mut self, rows: usize) -> Self {
        self.rows = rows;
        self
    }

    pub fn red_up(mut self) -> Self {
        self.red_up = true;
        self
    }
}

impl Default for Level2Quotes {
    fn default() -> Self {
        Self::new()
    }
}

/// Each quote's price level, zero for the best price, one for the next, and so on.
fn ranks(prices: &[f64]) -> Vec<usize> {
    let mut rank = 0;
    prices
        .iter()
        .enumerate()
        .map(|(ix, at)| {
            if ix > 0 && *at != prices[ix - 1] {
                rank += 1;
            }
            rank
        })
        .collect()
}

impl RenderOnce for Level2Quotes {
    fn render(mut self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (rise, fall) = moves(self.red_up, cx);
        self.bids.sort_by(|a, b| b.1.total_cmp(&a.1));
        self.asks.sort_by(|a, b| a.1.total_cmp(&b.1));
        let colors = cx.theme().colors.clone();
        let side = |quotes: &[(SharedString, f64, f64)], ink: Hsla, bids: bool| {
            let levels = ranks(&quotes.iter().map(|quote| quote.1).collect::<Vec<_>>());
            let names: &[&'static str] = if bids {
                &["Venue", "Size", "Bid"]
            } else {
                &["Ask", "Size", "Venue"]
            };
            div()
                .flex_1()
                .flex()
                .flex_col()
                .child(heading(names, cx))
                .children(quotes.iter().zip(levels).take(self.rows).map(
                    |((venue, at, size), rank)| {
                        let shade = ink.opacity((0.16 - 0.04 * rank as f32).max(0.03));
                        let venue = div()
                            .flex_1()
                            .text_right()
                            .text_color(colors.fg_muted)
                            .child(venue.clone());
                        let (at, size) = (
                            cell(price(*at, self.places), ink),
                            cell(compact(*size), colors.fg),
                        );
                        let row = div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .px_2()
                            .h(cx.theme().table_row(Density::Compact))
                            .text_size(cx.theme().text_size(TextSize::Sm))
                            .bg(shade);
                        if bids {
                            row.child(venue).child(size).child(at)
                        } else {
                            row.child(at).child(size).child(venue)
                        }
                    },
                ))
        };
        div()
            .flex()
            .gap_2()
            .child(side(&self.bids, rise, true))
            .child(side(&self.asks, fall, false))
    }
}

#[cfg(test)]
mod tests {
    use super::ranks;

    #[test]
    fn levels_rank_by_price() {
        assert_eq!(ranks(&[10.0, 10.0, 9.9, 9.8, 9.8]), [0, 0, 1, 2, 2]);
    }
}
