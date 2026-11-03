use gpui::{
    App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, relative,
};

use super::quotes::{PriceChangeBadge, QuoteCard, price};
use crate::{
    charts::tint,
    theme::{ActiveTheme, TextSize},
    typography::tabular,
};

/// Where a market's day stands: before hours, open, after hours, or closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Session {
    Before,
    Open,
    After,
    Closed,
}

/// Whether a market trades now, with a dot, and when that next changes.
#[derive(IntoElement)]
pub struct MarketStatus {
    market: SharedString,
    session: Session,
    next: Option<SharedString>,
}

impl MarketStatus {
    pub fn new(market: impl Into<SharedString>, session: Session) -> Self {
        Self {
            market: market.into(),
            session,
            next: None,
        }
    }

    /// When the session changes, such as "Closes in 2 h 14 min".
    pub fn next(mut self, next: impl Into<SharedString>) -> Self {
        self.next = Some(next.into());
        self
    }
}

impl RenderOnce for MarketStatus {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let (words, dot) = match self.session {
            Session::Before => ("Before hours", colors.warning),
            Session::Open => ("Open", colors.success),
            Session::After => ("After hours", colors.warning),
            Session::Closed => ("Closed", colors.fg_subtle),
        };
        div()
            .flex()
            .items_center()
            .gap_2()
            .px_2p5()
            .py_1()
            .rounded_full()
            .border_1()
            .border_color(colors.border)
            .text_size(theme.text_size(TextSize::Sm))
            .child(div().size(theme.status_dot()).rounded_full().bg(dot))
            .child(
                div()
                    .text_color(colors.fg)
                    .child(format!("{} · {words}", self.market)),
            )
            .children(
                self.next
                    .map(|next| div().text_color(colors.fg_muted).child(next)),
            )
    }
}

/// The world's trading day on one scale: each market's hours as a band across a day in UTC, those open now lit, and a line at now.
#[derive(IntoElement)]
pub struct TradingSessionClock {
    markets: Vec<(SharedString, f32, f32)>,
    now: f32,
}

impl TradingSessionClock {
    /// `now` in hours since midnight UTC.
    pub fn new(now: f32) -> Self {
        assert!((0.0..24.0).contains(&now), "now is an hour of the day");
        Self {
            markets: Vec::new(),
            now,
        }
    }

    /// A market and its hours in UTC; a session past midnight runs from `open` to `close` the next day.
    pub fn market(mut self, name: impl Into<SharedString>, open: f32, close: f32) -> Self {
        assert!(
            (0.0..24.0).contains(&open) && (0.0..=24.0).contains(&close),
            "hours run from 0 to 24"
        );
        self.markets.push((name.into(), open, close));
        self
    }
}

/// A session's bands across the day, split at midnight when it runs past it, as shares of the day.
pub(crate) fn bands(open: f32, close: f32) -> Vec<(f32, f32)> {
    if open <= close {
        vec![(open / 24.0, close / 24.0)]
    } else {
        vec![(open / 24.0, 1.0), (0.0, close / 24.0)]
    }
}

impl RenderOnce for TradingSessionClock {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let now = self.now / 24.0;
        let rows = self
            .markets
            .iter()
            .enumerate()
            .map(|(ix, (name, open, close))| {
                let spans = bands(*open, *close);
                let lit = spans.iter().any(|(from, to)| (*from..*to).contains(&now));
                let ink = tint(&colors, ix);
                let track = div()
                    .relative()
                    .flex_1()
                    .h(theme.meter_track() * 2.0)
                    .rounded_full()
                    .bg(colors.hover)
                    .children(spans.into_iter().map(|(from, to)| {
                        div()
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .left(relative(from))
                            .w(relative(to - from))
                            .rounded_full()
                            .bg(if lit { ink } else { ink.opacity(0.35) })
                    }));
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .w(theme.label_width() * 0.6)
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(if lit { colors.fg } else { colors.fg_muted })
                            .child(name.clone()),
                    )
                    .child(track)
            });
        let hours = [0, 6, 12, 18, 24].map(|hour| {
            let at = hour as f32 / 24.0;
            div()
                .absolute()
                .top_0()
                .left(relative(at))
                .w_0()
                .flex()
                .justify_center()
                .child(tabular(div()).child(format!("{hour:02}:00")))
        });
        let line = div()
            .absolute()
            .top_0()
            .bottom_0()
            .left(relative(now))
            .w(theme.chart().hairline)
            .bg(colors.fg);
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .children(rows)
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .left(theme.label_width() * 0.6)
                            .right_0()
                            .ml_3()
                            .child(line),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(div().w(theme.label_width() * 0.6))
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .h(theme.text_size(TextSize::Xs) * 1.5)
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg_subtle)
                            .children(hours),
                    ),
            )
    }
}

/// A market at a glance: its session, its indexes as cards in a row, and the day's movers up and down.
#[derive(IntoElement)]
pub struct MarketOverview {
    id: ElementId,
    status: Option<MarketStatus>,
    indexes: Vec<(SharedString, SharedString, f64, f64, Vec<f32>)>,
    movers: Vec<(SharedString, f64, f64)>,
    red_up: bool,
}

impl MarketOverview {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            status: None,
            indexes: Vec::new(),
            movers: Vec::new(),
            red_up: false,
        }
    }

    pub fn status(mut self, status: MarketStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// An index: its symbol, name, last value, last close, and today's values.
    pub fn index(
        mut self,
        (symbol, name): (impl Into<SharedString>, impl Into<SharedString>),
        (last, close): (f64, f64),
        day: impl IntoIterator<Item = f32>,
    ) -> Self {
        self.indexes.push((
            symbol.into(),
            name.into(),
            last,
            close,
            day.into_iter().collect(),
        ));
        self
    }

    /// A symbol that moved today: its last price and the last close.
    pub fn mover(mut self, symbol: impl Into<SharedString>, last: f64, close: f64) -> Self {
        self.movers.push((symbol.into(), last, close));
        self
    }

    pub fn red_up(mut self) -> Self {
        self.red_up = true;
        self
    }
}

impl RenderOnce for MarketOverview {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let red_up = self.red_up;
        let cards =
            self.indexes
                .into_iter()
                .enumerate()
                .map(|(ix, (symbol, name, last, close, day))| {
                    let card = QuoteCard::new(
                        (self.id.clone(), format!("index-{ix}")),
                        (symbol, name),
                        (last, close),
                        day,
                    );
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(if red_up { card.red_up() } else { card })
                });
        let mut movers = self.movers;
        movers.sort_by(|a, b| (b.1 / b.2).total_cmp(&(a.1 / a.2)));
        let column = |title: &'static str, list: Vec<(SharedString, f64, f64)>| {
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(colors.fg_muted)
                        .child(title),
                )
                .children(list.into_iter().map(|(symbol, last, close)| {
                    let badge = if red_up {
                        PriceChangeBadge::new(close, last).red_up()
                    } else {
                        PriceChangeBadge::new(close, last)
                    };
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .py_1()
                        .text_size(theme.text_size(TextSize::Sm))
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(colors.fg)
                                .child(symbol),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_3()
                                .child(
                                    tabular(div())
                                        .text_color(colors.fg_muted)
                                        .child(price(last, 2)),
                                )
                                .child(badge),
                        )
                }))
        };
        let half = movers.len().div_ceil(2);
        let losers: Vec<_> = movers.split_off(half).into_iter().rev().collect();
        div()
            .flex()
            .flex_col()
            .gap_4()
            .children(self.status)
            .child(div().flex().gap_4().children(cards))
            .child(
                div()
                    .flex()
                    .gap_8()
                    .child(column("Up the most", movers))
                    .child(column("Down the most", losers)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::bands;

    #[test]
    fn sessions_split_at_midnight() {
        assert_eq!(bands(8.0, 16.5), [(8.0 / 24.0, 16.5 / 24.0)]);
        assert_eq!(
            bands(22.0, 6.0),
            [(22.0 / 24.0, 1.0), (0.0, 0.25)],
            "a session past midnight takes two bands"
        );
    }
}
