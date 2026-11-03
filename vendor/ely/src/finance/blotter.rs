use std::cmp::Reverse;

use gpui::{
    App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div,
};
use jiff::{Timestamp, tz::TimeZone};

use super::{
    book::Side,
    quotes::moves,
    trade::{Order, OrderKind},
};
use crate::{
    data_display::Tone,
    motion::Flash,
    tables::{Aggregate, Cell, Column, DataTable, Row},
    theme::{ActiveTheme, TextSize},
    typography::{
        format::{self, system_zone},
        tabular,
    },
};

/// A held position: its symbol, how many, what each cost on average, and the last price.
#[derive(Clone, Debug, PartialEq)]
pub struct Position {
    pub symbol: SharedString,
    pub quantity: f64,
    pub cost: f64,
    pub last: f64,
}

/// A signed share as a tag, green up and red down.
fn signed(share: f64) -> Cell {
    let tone = if share > 0.0 {
        Tone::Success
    } else if share < 0.0 {
        Tone::Danger
    } else {
        Tone::Neutral
    };
    Cell::Tag(format::percent(share, 2, true).into(), tone)
}

/// Positions held, with what each is worth now and what it has made or lost; totals below.
#[derive(IntoElement)]
pub struct PositionTable {
    id: ElementId,
    positions: Vec<Position>,
}

impl PositionTable {
    pub fn new(id: impl Into<ElementId>, positions: impl IntoIterator<Item = Position>) -> Self {
        Self {
            id: id.into(),
            positions: positions.into_iter().collect(),
        }
    }
}

impl RenderOnce for PositionTable {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let columns = [
            Column::new("symbol", "Symbol"),
            Column::new("quantity", "Quantity").end(),
            Column::new("cost", "Average cost").end().decimals(2),
            Column::new("last", "Last").end().decimals(2),
            Column::new("value", "Value")
                .end()
                .decimals(2)
                .aggregate(Aggregate::Sum),
            Column::new("gain", "Gain")
                .end()
                .decimals(2)
                .aggregate(Aggregate::Sum),
            Column::new("share", "Return").end(),
        ];
        let rows: Vec<Row> = self
            .positions
            .iter()
            .map(|held| {
                let gain = (held.last - held.cost) * held.quantity;
                let basis = (held.cost * held.quantity).abs();
                let share = if basis > 0.0 {
                    signed(gain / basis)
                } else {
                    Cell::Empty
                };
                Row::new(
                    held.symbol.clone(),
                    [
                        Cell::Text(held.symbol.clone()),
                        held.quantity.into(),
                        held.cost.into(),
                        held.last.into(),
                        (held.last * held.quantity).into(),
                        gain.into(),
                        share,
                    ],
                )
            })
            .collect();
        DataTable::new(self.id, columns).rows(rows)
    }
}

/// An order still working: when it was placed, its order, and how many of its quantity have filled.
#[derive(Clone, Debug, PartialEq)]
pub struct Working {
    pub time: Timestamp,
    pub order: Order,
    pub filled: f64,
}

/// Orders still working, newest first: when, what, which way, how, how much at what price, and how far each has filled.
#[derive(IntoElement)]
pub struct OrderTable {
    id: ElementId,
    orders: Vec<Working>,
    zone: Option<TimeZone>,
}

impl OrderTable {
    pub fn new(id: impl Into<ElementId>, orders: impl IntoIterator<Item = Working>) -> Self {
        let mut orders: Vec<Working> = orders.into_iter().collect();
        assert!(
            orders.iter().all(|working| {
                working.order.quantity > 0.0
                    && (0.0..=working.order.quantity).contains(&working.filled)
            }),
            "a working order has a quantity and fills from none up to it"
        );
        orders.sort_by_key(|working| Reverse(working.time));
        Self {
            id: id.into(),
            orders,
            zone: None,
        }
    }

    /// The time zone its times read in; the system's unless set.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }
}

/// A side as a tag.
fn side_tag(side: Side) -> Cell {
    match side {
        Side::Buy => Cell::Tag("Buy".into(), Tone::Success),
        Side::Sell => Cell::Tag("Sell".into(), Tone::Danger),
    }
}

impl RenderOnce for OrderTable {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let zone = self
            .zone
            .clone()
            .unwrap_or_else(|| system_zone("OrderTable"));
        let columns = [
            Column::new("time", "Placed"),
            Column::new("symbol", "Symbol"),
            Column::new("side", "Side"),
            Column::new("kind", "Type"),
            Column::new("quantity", "Quantity").end(),
            Column::new("price", "Price").end().decimals(2),
            Column::new("filled", "Filled"),
        ];
        let rows: Vec<Row> = self
            .orders
            .iter()
            .enumerate()
            .map(|(ix, working)| {
                let order = &working.order;
                let price = if order.kind == OrderKind::Market {
                    Cell::Text("Market".into())
                } else {
                    order.price.into()
                };
                let time: SharedString = format::datetime(working.time, &zone, "%H:%M:%S")
                    .expect("a fixed pattern")
                    .into();
                let filled = working.filled / order.quantity;
                Row::new(
                    format!("order-{ix}"),
                    [
                        Cell::Text(time),
                        Cell::Text(order.symbol.clone()),
                        side_tag(order.side),
                        Cell::Text(order.kind.word().into()),
                        order.quantity.into(),
                        price,
                        Cell::Progress(filled as f32),
                    ],
                )
            })
            .collect();
        DataTable::new(self.id, columns).rows(rows)
    }
}

/// A filled trade: when, what, which way, how much at what price, and its fee.
#[derive(Clone, Debug, PartialEq)]
pub struct Fill {
    pub time: Timestamp,
    pub symbol: SharedString,
    pub side: Side,
    pub quantity: f64,
    pub price: f64,
    pub fee: f64,
}

/// Trades filled, newest first, with what each came to after its fee; totals below.
#[derive(IntoElement)]
pub struct TradeHistoryTable {
    id: ElementId,
    fills: Vec<Fill>,
    zone: Option<TimeZone>,
}

impl TradeHistoryTable {
    pub fn new(id: impl Into<ElementId>, fills: impl IntoIterator<Item = Fill>) -> Self {
        let mut fills: Vec<Fill> = fills.into_iter().collect();
        fills.sort_by_key(|fill| Reverse(fill.time));
        Self {
            id: id.into(),
            fills,
            zone: None,
        }
    }

    /// The time zone its times read in; the system's unless set.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }
}

impl RenderOnce for TradeHistoryTable {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let zone = self
            .zone
            .clone()
            .unwrap_or_else(|| system_zone("TradeHistoryTable"));
        let columns = [
            Column::new("time", "Time"),
            Column::new("symbol", "Symbol"),
            Column::new("side", "Side"),
            Column::new("quantity", "Quantity").end(),
            Column::new("price", "Price").end().decimals(2),
            Column::new("fee", "Fee")
                .end()
                .decimals(2)
                .aggregate(Aggregate::Sum),
            Column::new("total", "Total")
                .end()
                .decimals(2)
                .aggregate(Aggregate::Sum),
        ];
        let rows: Vec<Row> = self
            .fills
            .iter()
            .enumerate()
            .map(|(ix, fill)| {
                let signed = if fill.side == Side::Buy { -1.0 } else { 1.0 };
                let total = signed * fill.quantity * fill.price - fill.fee;
                let time: SharedString = format::datetime(fill.time, &zone, "%b %-d %H:%M")
                    .expect("a fixed pattern")
                    .into();
                Row::new(
                    format!("fill-{ix}"),
                    [
                        Cell::Text(time),
                        Cell::Text(fill.symbol.clone()),
                        side_tag(fill.side),
                        fill.quantity.into(),
                        fill.price.into(),
                        fill.fee.into(),
                        total.into(),
                    ],
                )
            })
            .collect();
        DataTable::new(self.id, columns).rows(rows)
    }
}

/// A profit or loss as it reads at a glance: signed and tinted in its currency, its share of cost below, flashing when it moves.
#[derive(IntoElement)]
pub struct PnLDisplay {
    id: ElementId,
    label: SharedString,
    amount: f64,
    share: f64,
    currency: SharedString,
    red_up: bool,
}

impl PnLDisplay {
    /// `share` is the amount as a share of what it cost; `currency` an ISO 4217 code.
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        (amount, share): (f64, f64),
        currency: &str,
    ) -> Self {
        assert!(
            amount.is_finite() && share.is_finite(),
            "a result needs finite numbers"
        );
        Self {
            id: id.into(),
            label: label.into(),
            amount,
            share,
            currency: currency.to_string().into(),
            red_up: false,
        }
    }

    pub fn red_up(mut self) -> Self {
        self.red_up = true;
        self
    }
}

impl RenderOnce for PnLDisplay {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (rise, fall) = moves(self.red_up, cx);
        let theme = cx.theme();
        let ink = if self.amount > 0.0 {
            rise
        } else if self.amount < 0.0 {
            fall
        } else {
            theme.colors.fg
        };
        let sign = if self.amount > 0.0 { "+" } else { "" };
        div()
            .flex()
            .flex_col()
            .gap_0p5()
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(theme.colors.fg_muted)
                    .child(self.label),
            )
            .child(
                Flash::new(self.id, self.amount.to_bits())
                    .rounded(theme.radius(crate::theme::Radius::Sm))
                    .child(
                        tabular(div())
                            .text_size(theme.text_size(TextSize::Xl))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(ink)
                            .child(format!(
                                "{sign}{}",
                                format::currency(self.amount, &self.currency)
                            )),
                    ),
            )
            .child(
                tabular(div())
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(ink)
                    .child(format::percent(self.share, 2, true)),
            )
    }
}
