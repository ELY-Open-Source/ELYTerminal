use std::rc::Rc;

use gpui::{
    App, Div, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
};

use super::{book::Side, quotes::price};
use crate::{
    buttons::{Button, ButtonVariant, SegmentedControl},
    forms::NumberInput,
    overlays::ConfirmDialog,
    theme::{ActiveTheme, ControlSize, TextSize},
    typography::{format, tabular},
};

/// How an order fills: now at the market, at its price or better, or once the market reaches its stop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OrderKind {
    #[default]
    Market,
    Limit,
    Stop,
}

impl OrderKind {
    pub(crate) fn word(self) -> &'static str {
        match self {
            OrderKind::Market => "Market",
            OrderKind::Limit => "Limit",
            OrderKind::Stop => "Stop",
        }
    }
}

/// An order before it is placed: its symbol, which way, how it fills, how much, and the price it waits for when it waits.
#[derive(Clone, Debug, PartialEq)]
pub struct Order {
    pub symbol: SharedString,
    pub side: Side,
    pub kind: OrderKind,
    pub quantity: f64,
    pub price: f64,
}

impl Order {
    /// What it comes to at `last` for a market order, or at its own price.
    pub fn value(&self, last: f64) -> f64 {
        self.quantity
            * if self.kind == OrderKind::Market {
                last
            } else {
                self.price
            }
    }

    /// The order in a line, such as "Buy 100 ELY at the market".
    pub fn words(&self, places: usize) -> String {
        let side = if self.side == Side::Buy {
            "Buy"
        } else {
            "Sell"
        };
        let how = match self.kind {
            OrderKind::Market => "at the market".to_string(),
            OrderKind::Limit => format!("at {} or better", price(self.price, places)),
            OrderKind::Stop => format!("once it reaches {}", price(self.price, places)),
        };
        format!(
            "{side} {} {} {how}",
            format::number(self.quantity, 0, format::Separators::EN),
            self.symbol
        )
    }
}

type OnOrder = Rc<dyn Fn(Order, &mut Window, &mut App)>;
type Run = Rc<dyn Fn(&mut Window, &mut App)>;

/// An order to fill in: its side, how it fills, its price when it waits for one, its quantity and what it comes to; the button places it.
#[derive(IntoElement)]
pub struct OrderEntry {
    id: ElementId,
    order: Order,
    last: f64,
    on_change: Option<OnOrder>,
    on_submit: Option<OnOrder>,
}

impl OrderEntry {
    pub fn new(id: impl Into<ElementId>, order: Order, last: f64) -> Self {
        Self {
            id: id.into(),
            order,
            last,
            on_change: None,
            on_submit: None,
        }
    }

    /// Gets the order after each change to it.
    pub fn on_change(mut self, handler: impl Fn(Order, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    pub fn on_submit(mut self, handler: impl Fn(Order, &mut Window, &mut App) + 'static) -> Self {
        self.on_submit = Some(Rc::new(handler));
        self
    }
}

/// A labelled row of a form.
fn field(label: &'static str, body: impl IntoElement, cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(theme.colors.fg_muted)
                .child(label),
        )
        .child(body)
}

impl RenderOnce for OrderEntry {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let order = self.order.clone();
        let change = |edit: fn(&mut Order, f64), order: Order, on_change: Option<OnOrder>| {
            move |value: f64, window: &mut Window, cx: &mut App| {
                let mut next = order.clone();
                edit(&mut next, value);
                if let Some(on_change) = &on_change {
                    on_change(next, window, cx);
                }
            }
        };
        let pick = |order: Order, on_change: Option<OnOrder>| {
            move |key: &SharedString, window: &mut Window, cx: &mut App| {
                let mut next = order.clone();
                match key.as_ref() {
                    "buy" => next.side = Side::Buy,
                    "sell" => next.side = Side::Sell,
                    "limit" => next.kind = OrderKind::Limit,
                    "stop" => next.kind = OrderKind::Stop,
                    _ => next.kind = OrderKind::Market,
                }
                if let Some(on_change) = &on_change {
                    on_change(next, window, cx);
                }
            }
        };
        let side = SegmentedControl::new(
            (self.id.clone(), "side"),
            if order.side == Side::Buy {
                "buy"
            } else {
                "sell"
            },
        )
        .segment("buy", "Buy", None)
        .segment("sell", "Sell", None)
        .on_change(pick(order.clone(), self.on_change.clone()));
        let kind = [OrderKind::Market, OrderKind::Limit, OrderKind::Stop]
            .iter()
            .fold(
                SegmentedControl::new((self.id.clone(), "kind"), order.kind.word().to_lowercase())
                    .size(ControlSize::Sm),
                |control, kind| control.segment(kind.word().to_lowercase(), kind.word(), None),
            );
        let kind = kind.on_change(pick(order.clone(), self.on_change.clone()));
        let price_field = if order.kind == OrderKind::Market {
            div()
                .flex()
                .justify_between()
                .px_3()
                .py_1p5()
                .rounded(theme.radius(crate::theme::Radius::Md))
                .bg(colors.sunken)
                .text_color(colors.fg_muted)
                .child("Market")
                .child(tabular(div()).child(price(self.last, 2)))
                .into_any_element()
        } else {
            NumberInput::new((self.id.clone(), "price"), order.price)
                .step(0.01)
                .precision(2)
                .on_change(change(
                    |order, value| order.price = value,
                    order.clone(),
                    self.on_change.clone(),
                ))
                .into_any_element()
        };
        let quantity = NumberInput::new((self.id.clone(), "quantity"), order.quantity)
            .range(0.0, f64::MAX)
            .step(1.0)
            .precision(0)
            .on_change(change(
                |order, value| order.quantity = value,
                order.clone(),
                self.on_change.clone(),
            ));
        let variant = if order.side == Side::Buy {
            ButtonVariant::Success
        } else {
            ButtonVariant::Danger
        };
        let submit = Button::new((self.id.clone(), "place"), order.words(2))
            .variant(variant)
            .full_width();
        let placed = (order.clone(), self.on_submit.clone());
        let submit = submit
            .disabled(order.quantity <= 0.0)
            .on_click(move |_, window, cx| {
                log::info!("order entry: {}", placed.0.words(2));
                if let Some(on_submit) = &placed.1 {
                    on_submit(placed.0.clone(), window, cx);
                }
            });
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(side)
            .child(kind)
            .child(field("Price", price_field, cx))
            .child(field("Quantity", quantity, cx))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .text_size(theme.text_size(TextSize::Sm))
                    .child(div().text_color(colors.fg_muted).child("Value"))
                    .child(
                        tabular(div())
                            .text_color(colors.fg)
                            .child(format!("${}", price(order.value(self.last), 2))),
                    ),
            )
            .child(submit)
    }
}

type OnSide = Rc<dyn Fn(Side, &mut Window, &mut App)>;

/// Buy or sell at a press: the bid on the sell button, the ask on the buy button, and the spread between them.
#[derive(IntoElement)]
pub struct QuickTradeButtons {
    id: ElementId,
    bid: f64,
    ask: f64,
    on_trade: Option<OnSide>,
}

impl QuickTradeButtons {
    pub fn new(id: impl Into<ElementId>, bid: f64, ask: f64) -> Self {
        assert!(bid.is_finite() && ask.is_finite(), "quotes are finite");
        Self {
            id: id.into(),
            bid,
            ask,
            on_trade: None,
        }
    }

    pub fn on_trade(mut self, handler: impl Fn(Side, &mut Window, &mut App) + 'static) -> Self {
        self.on_trade = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for QuickTradeButtons {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let button = |side: Side, at: f64| {
            let (name, variant) = if side == Side::Buy {
                ("Buy", ButtonVariant::Success)
            } else {
                ("Sell", ButtonVariant::Danger)
            };
            let trade = self.on_trade.clone();
            div().flex_1().child(
                Button::new((self.id.clone(), name), format!("{name} {}", price(at, 2)))
                    .variant(variant)
                    .size(ControlSize::Lg)
                    .full_width()
                    .on_click(move |_, window, cx| {
                        log::info!("quick trade: {side:?} at {at}");
                        if let Some(trade) = &trade {
                            trade(side, window, cx);
                        }
                    }),
            )
        };
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(button(Side::Sell, self.bid))
            .child(
                tabular(div())
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(theme.colors.fg_muted)
                    .child(price(self.ask - self.bid, 2)),
            )
            .child(button(Side::Buy, self.ask))
    }
}

/// Asks before an order goes, reading back what it will do and what it comes to.
#[derive(IntoElement)]
pub struct OrderConfirmDialog {
    id: ElementId,
    order: Order,
    last: f64,
    on_confirm: Option<OnOrder>,
    on_close: Run,
}

impl OrderConfirmDialog {
    /// `on_close` runs on every way out, placing the order included.
    pub fn new(
        id: impl Into<ElementId>,
        order: Order,
        last: f64,
        on_close: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            order,
            last,
            on_confirm: None,
            on_close: Rc::new(on_close),
        }
    }

    pub fn on_confirm(mut self, handler: impl Fn(Order, &mut Window, &mut App) + 'static) -> Self {
        self.on_confirm = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for OrderConfirmDialog {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let message = format!(
            "{}, about ${} in all.",
            self.order.words(2),
            price(self.order.value(self.last), 2)
        );
        let close = self.on_close.clone();
        let dialog =
            ConfirmDialog::new(self.id, "Place this order?", message, move |window, cx| {
                close(window, cx)
            })
            .confirm(if self.order.side == Side::Buy {
                "Buy"
            } else {
                "Sell"
            });
        let dialog = if self.order.side == Side::Sell {
            dialog.destructive()
        } else {
            dialog
        };
        match self.on_confirm {
            Some(on_confirm) => {
                let order = self.order;
                dialog.on_confirm(move |window, cx| on_confirm(order.clone(), window, cx))
            }
            None => dialog,
        }
    }
}
