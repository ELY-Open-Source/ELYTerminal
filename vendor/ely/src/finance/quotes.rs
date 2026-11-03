use gpui::{
    App, ElementId, FontWeight, Hsla, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, relative,
};

use crate::{
    data_display::{Badge, Sparkline, Tone},
    motion::{Flash, Marquee},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
    typography::{format, tabular},
};

/// The rise and fall colors, green up unless red leads.
pub(crate) fn moves(red_up: bool, cx: &App) -> (Hsla, Hsla) {
    let colors = &cx.theme().colors;
    if red_up {
        (colors.danger, colors.success)
    } else {
        (colors.success, colors.danger)
    }
}

/// A price as it reads: grouped thousands and its places.
pub(crate) fn price(value: f64, places: usize) -> String {
    format::number(value, places, format::Separators::EN)
}

/// A change and its share as a signed pair, such as "+1.24 (+0.66%)".
pub(crate) fn change(amount: f64, share: f64, places: usize) -> String {
    let sign = if amount > 0.0 { "+" } else { "" };
    format!(
        "{sign}{} ({})",
        price(amount, places),
        format::percent(share, 2, true)
    )
}

/// A price read to its places that tints green or red for a moment each time it moves, with an arrow the way it last went.
#[derive(IntoElement)]
pub struct PriceText {
    id: ElementId,
    value: f64,
    before: Option<f64>,
    places: usize,
    size: TextSize,
    red_up: bool,
}

impl PriceText {
    pub fn new(id: impl Into<ElementId>, value: f64) -> Self {
        assert!(value.is_finite(), "a price needs a finite value");
        Self {
            id: id.into(),
            value,
            before: None,
            places: 2,
            size: TextSize::Base,
            red_up: false,
        }
    }

    /// The price it moved from, which sets its arrow and tint.
    pub fn from(mut self, before: f64) -> Self {
        self.before = Some(before);
        self
    }

    pub fn places(mut self, places: usize) -> Self {
        self.places = places;
        self
    }

    pub fn size(mut self, size: TextSize) -> Self {
        self.size = size;
        self
    }

    pub fn red_up(mut self) -> Self {
        self.red_up = true;
        self
    }
}

impl RenderOnce for PriceText {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (rise, fall) = moves(self.red_up, cx);
        let theme = cx.theme();
        let way = self.before.map(|before| self.value.total_cmp(&before));
        let ink = match way {
            Some(std::cmp::Ordering::Greater) => rise,
            Some(std::cmp::Ordering::Less) => fall,
            _ => theme.colors.fg,
        };
        let arrow = way.filter(|way| way.is_ne()).map(|way| {
            let icon = if way.is_gt() {
                IconName::ArrowUp
            } else {
                IconName::ArrowDown
            };
            Icon::new(icon).size(IconSize::Xs).color(ink)
        });
        Flash::new(self.id, self.value.to_bits())
            .tint(ink.opacity(0.18))
            .flex()
            .items_center()
            .gap_1()
            .px_1()
            .rounded(theme.radius(Radius::Sm))
            .children(arrow)
            .child(
                tabular(div())
                    .text_size(theme.text_size(self.size))
                    .text_color(ink)
                    .child(price(self.value, self.places)),
            )
    }
}

/// A move since a reference, its amount and share, tinted by which way it went.
#[derive(IntoElement)]
pub struct PriceChangeBadge {
    amount: f64,
    share: f64,
    places: usize,
    red_up: bool,
}

impl PriceChangeBadge {
    /// The move from `before` to `now`.
    pub fn new(before: f64, now: f64) -> Self {
        assert!(
            before.is_finite() && now.is_finite() && before != 0.0,
            "a change needs finite prices and a start other than zero"
        );
        Self {
            amount: now - before,
            share: (now - before) / before,
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

impl RenderOnce for PriceChangeBadge {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let tone = match (self.amount.total_cmp(&0.0), self.red_up) {
            (std::cmp::Ordering::Equal, _) => Tone::Neutral,
            (std::cmp::Ordering::Greater, false) | (std::cmp::Ordering::Less, true) => {
                Tone::Success
            }
            _ => Tone::Danger,
        };
        Badge::new(change(self.amount, self.share, self.places)).tone(tone)
    }
}

/// A symbol at a glance: its name, price and move since the last close, the day's line, and where the price sits in the day's range.
#[derive(IntoElement)]
pub struct QuoteCard {
    id: ElementId,
    symbol: SharedString,
    name: SharedString,
    now: f64,
    close: f64,
    day: Vec<f32>,
    red_up: bool,
}

impl QuoteCard {
    /// `close` is the last session's; `day` the prices since, oldest first.
    pub fn new(
        id: impl Into<ElementId>,
        (symbol, name): (impl Into<SharedString>, impl Into<SharedString>),
        (now, close): (f64, f64),
        day: impl IntoIterator<Item = f32>,
    ) -> Self {
        let day: Vec<f32> = day.into_iter().collect();
        assert!(day.len() >= 2, "a day's line needs two prices");
        Self {
            id: id.into(),
            symbol: symbol.into(),
            name: name.into(),
            now,
            close,
            day,
            red_up: false,
        }
    }

    pub fn red_up(mut self) -> Self {
        self.red_up = true;
        self
    }
}

impl RenderOnce for QuoteCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (rise, fall) = moves(self.red_up, cx);
        let theme = cx.theme();
        let colors = &theme.colors;
        let (low, high) = self
            .day
            .iter()
            .fold((f32::MAX, f32::MIN), |(low, high), value| {
                (low.min(*value), high.max(*value))
            });
        let at = if high > low {
            (self.now as f32 - low) / (high - low)
        } else {
            0.5
        };
        let ink = if self.now >= self.close { rise } else { fall };
        let badge = if self.red_up {
            PriceChangeBadge::new(self.close, self.now).red_up()
        } else {
            PriceChangeBadge::new(self.close, self.now)
        };
        let quote = if self.red_up {
            PriceText::new(self.id.clone(), self.now).red_up()
        } else {
            PriceText::new(self.id.clone(), self.now)
        };
        let small = |words: String| {
            tabular(div())
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(colors.fg_subtle)
                .child(words)
        };
        let track = div()
            .relative()
            .flex_1()
            .h(theme.meter_track())
            .rounded_full()
            .bg(colors.border)
            .child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(relative(at.clamp(0.0, 1.0)))
                    .w(theme.meter_track())
                    .rounded_full()
                    .bg(colors.fg),
            );
        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap_4()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(theme.text_size(TextSize::Md))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.fg)
                                    .child(self.symbol),
                            )
                            .child(
                                div()
                                    .text_size(theme.text_size(TextSize::Sm))
                                    .text_color(colors.fg_muted)
                                    .child(self.name),
                            ),
                    )
                    .child(badge),
            )
            .child(quote.size(TextSize::Xl).from(self.close))
            .child(
                Sparkline::new(self.day)
                    .area()
                    .color(ink)
                    .w_full()
                    .h(theme.chart().height * 0.2),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(small(price(f64::from(low), 2)))
                    .child(track)
                    .child(small(price(f64::from(high), 2))),
            )
    }
}

/// Quotes running past in a band, each symbol with its price and move.
#[derive(IntoElement)]
pub struct TickerTape {
    id: ElementId,
    quotes: Vec<(SharedString, f64, f64)>,
    red_up: bool,
}

impl TickerTape {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            quotes: Vec::new(),
            red_up: false,
        }
    }

    /// A symbol, its price, and its move as a share since the last close.
    pub fn quote(mut self, symbol: impl Into<SharedString>, now: f64, share: f64) -> Self {
        self.quotes.push((symbol.into(), now, share));
        self
    }

    pub fn red_up(mut self) -> Self {
        self.red_up = true;
        self
    }
}

impl RenderOnce for TickerTape {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (rise, fall) = moves(self.red_up, cx);
        let theme = cx.theme();
        let (colors, size) = (theme.colors.clone(), theme.text_size(TextSize::Sm));
        let quotes = self.quotes;
        let tape = Marquee::new(self.id, move || {
            div()
                .flex()
                .items_center()
                .children(quotes.iter().map(|(symbol, now, share)| {
                    let ink = if *share >= 0.0 { rise } else { fall };
                    let icon = if *share >= 0.0 {
                        IconName::TrendingUp
                    } else {
                        IconName::TrendingDown
                    };
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_4()
                        .text_size(size)
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(colors.fg)
                                .child(symbol.clone()),
                        )
                        .child(
                            tabular(div())
                                .text_color(colors.fg_muted)
                                .child(price(*now, 2)),
                        )
                        .child(Icon::new(icon).size(IconSize::Xs).color(ink))
                        .child(
                            tabular(div())
                                .text_color(ink)
                                .child(format::percent(*share, 2, true)),
                        )
                }))
        });
        div()
            .py_2()
            .border_t_1()
            .border_b_1()
            .border_color(theme.colors.border)
            .child(tape)
    }
}

#[cfg(test)]
mod tests {
    use super::{change, price};

    #[test]
    fn prices_and_changes_read_signed() {
        assert_eq!(price(1234.5, 2), "1,234.50");
        assert_eq!(change(1.24, 0.0066, 2), "+1.24 (+0.66%)");
        assert_eq!(
            change(-0.5, -0.01, 2),
            "\u{2212}0.50 (\u{2212}1.00%)",
            "a true minus"
        );
    }
}
