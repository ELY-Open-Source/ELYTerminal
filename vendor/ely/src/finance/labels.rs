use std::ops::Range;

use gpui::{
    App, Div, FontWeight, Hsla, ParentElement, Pixels, SharedString, Styled, Window, div,
    prelude::*,
};
use jiff::tz::TimeZone;

use super::{candles::Candle, series::Drawn, stage::Visible, tools::Drawing};
use crate::{
    charts::{Linear, Rect, tint},
    theme::{ActiveTheme, Radius, TextSize},
    typography::{
        LEADING,
        format::{self, decimals},
        tabular,
    },
};

/// Where a market chart's words go: its panes and their scales, what is in view, the candle and row under the pointer, the zone its times read in, its title, and a compared symbol's name.
pub(crate) struct Axes {
    pub panes: Vec<(Rect, Linear, Vec<f64>)>,
    pub visible: Visible,
    pub range: Range<usize>,
    pub hover: Option<i64>,
    pub row: Option<(usize, f32)>,
    pub zone: TimeZone,
    pub title: Option<SharedString>,
    pub compare: Option<SharedString>,
    pub drawings: Vec<Drawing>,
}

/// Places a price shows: its ticks' own, and at least two.
fn places(ticks: &[f64]) -> usize {
    let step = ticks
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .next()
        .unwrap_or(1.0);
    decimals(step).max(2)
}

/// How a candle's time reads: the month and year for monthly candles, the day for daily, the hour within a day.
fn stamp(candle: &Candle, interval: i64, zone: &TimeZone, full: bool) -> String {
    let pattern = match (interval, full) {
        (seconds, _) if seconds >= 86_400 * 28 => "%b %Y",
        (seconds, true) if seconds >= 86_400 => "%a %b %-d, %Y",
        (seconds, false) if seconds >= 86_400 => "%b %-d",
        (_, true) => "%b %-d %H:%M",
        (_, false) => "%H:%M",
    };
    format::datetime(candle.time, zone, pattern).expect("a fixed pattern")
}

/// Price alerts with their words, and the last price with whether it rose.
pub(crate) type Marks<'a> = (&'a [(f64, SharedString)], Option<(f64, bool)>);

/// Every word a market chart shows: price and time ticks, the readout of the candle under the pointer or the last, each study's readout, and the pills at the price scale for the last price, alerts and the crosshair.
pub(crate) fn labels(
    axes: &Axes,
    (shown, candles): (&[Candle], &[Candle]),
    (overlays, studies): (&[Drawn], &[Drawn]),
    (alerts, last): Marks,
    (rise, fall): (Hsla, Hsla),
    window: &Window,
    cx: &App,
) -> Vec<Div> {
    let theme = cx.theme();
    let (colors, sizes, rem) = (&theme.colors, theme.chart(), window.rem_size());
    let small = theme.text_size(TextSize::Xs);
    let text = |words: String, color: Hsla| {
        div()
            .flex_none()
            .whitespace_nowrap()
            .text_size(small)
            .text_color(color)
            .child(words)
    };
    let (main, price, price_ticks) = (&axes.panes[0].0, &axes.panes[0].1, &axes.panes[0].2);
    let digits = places(price_ticks);
    let edge = main.x + main.w;
    let beside = |y: f32, label: Div| {
        div()
            .absolute()
            .left(Pixels::from(edge))
            .top(Pixels::from(y))
            .h_0()
            .flex()
            .items_center()
            .pl_2()
            .child(label)
    };
    let mut out: Vec<Div> = Vec::new();
    for (ix, (_, scale, ticks)) in axes.panes.iter().enumerate() {
        let digits = if ix == 0 {
            digits
        } else {
            places(ticks).min(2)
        };
        out.extend(ticks.iter().map(|tick| {
            beside(
                scale.at(*tick),
                tabular(text(
                    format::number(*tick, digits, format::Separators::EN),
                    colors.fg_subtle,
                )),
            )
        }));
    }
    let interval = candles.windows(2).next().map_or(86_400, |pair| {
        pair[1].time.as_second() - pair[0].time.as_second()
    });
    let room = f32::from(sizes.label.to_pixels(rem)) * 1.25;
    let every = ((room / axes.visible.slot(main.w)).ceil() as usize).max(1);
    let bottom = axes
        .panes
        .last()
        .map_or(main.y + main.h, |(pane, _, _)| pane.y + pane.h);
    for ix in axes.range.clone().filter(|ix| ix % every == 0) {
        let x = axes.visible.x(ix as f64, (main.x, main.w));
        if x < main.x || x > edge {
            continue;
        }
        let label = text(
            stamp(&candles[ix], interval, &axes.zone, false),
            colors.fg_subtle,
        );
        out.push(
            div()
                .absolute()
                .left(Pixels::from(x))
                .top(Pixels::from(bottom))
                .w_0()
                .flex()
                .justify_center()
                .pt_1p5()
                .child(label),
        );
    }
    let at = axes
        .hover
        .map(|ix| ix as usize)
        .or(candles.len().checked_sub(1));
    if let Some(ix) = at {
        let candle = &shown[ix];
        let change = candle.close - candle.open;
        let ink = if change >= 0.0 { rise } else { fall };
        let number = |value: f64| format::number(value, digits, format::Separators::EN);
        let mut readout = div()
            .absolute()
            .left(Pixels::from(main.x))
            .top(Pixels::from(main.y))
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x_3()
            .gap_y_0p5()
            .px_1();
        if let Some(title) = &axes.title {
            readout = readout.child(
                div()
                    .text_size(theme.text_size(TextSize::Sm))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.fg)
                    .child(title.clone()),
            );
        }
        for (name, value) in [
            ("O", candle.open),
            ("H", candle.high),
            ("L", candle.low),
            ("C", candle.close),
        ] {
            readout = readout.child(
                div()
                    .flex()
                    .gap_1()
                    .child(text(name.into(), colors.fg_subtle))
                    .child(tabular(text(number(value), ink))),
            );
        }
        let share = if candle.open != 0.0 {
            change / candle.open
        } else {
            0.0
        };
        readout = readout.child(tabular(text(format::percent(share, 2, true), ink)));
        for drawn in overlays {
            let reading = drawn.lines.first().and_then(|(_, values)| values[ix]);
            let color = tint(colors, drawn.lines.first().map_or(0, |(color, _)| *color));
            readout = readout.child(
                div()
                    .flex()
                    .gap_1()
                    .child(text(drawn.name.to_string(), color))
                    .children(reading.map(|value| tabular(text(number(value), color)))),
            );
        }
        if let Some(name) = &axes.compare {
            readout = readout.child(
                text(format!("vs {name}"), colors.fg_muted)
                    .debug_selector(|| "chart-compared".into()),
            );
        }
        out.push(readout);
        for ((pane, _, _), drawn) in axes.panes.iter().skip(1).zip(studies) {
            let values = drawn
                .lines
                .iter()
                .filter_map(|(color, values)| values[ix].map(|value| (*color, value)));
            let line = div()
                .absolute()
                .left(Pixels::from(pane.x))
                .top(Pixels::from(pane.y))
                .flex()
                .gap_2()
                .px_1()
                .child(text(drawn.name.to_string(), colors.fg_muted));
            out.push(line.children(values.map(|(color, value)| {
                tabular(text(
                    format::number(value, 2, format::Separators::EN),
                    tint(colors, color),
                ))
            })));
        }
    }
    let tall = f32::from(small.to_pixels(rem)) * LEADING;
    let mut levels = Vec::new();
    for drawing in &axes.drawings {
        let Drawing::Fib(from, to) = *drawing else {
            continue;
        };
        let x = axes.visible.x(from.0.min(to.0), (main.x, main.w));
        if x > main.x + main.w {
            continue;
        }
        let left = (x - main.x).max(0.0);
        for (share, value) in Drawing::levels(from, to) {
            let y = price.at(value) - main.y;
            if !(0.0..=main.h).contains(&y) {
                continue;
            }
            let words = format!(
                "{} {}",
                format::percent(share, 1, false),
                format::number(value, digits, format::Separators::EN)
            );
            let above = y >= tall;
            levels.push(
                div()
                    .absolute()
                    .left(Pixels::from(left))
                    .top(Pixels::from(y))
                    .h_0()
                    .flex()
                    .pl_1()
                    .when(above, |label| label.items_end())
                    .when(!above, |label| label.items_start())
                    .child(
                        tabular(text(words, colors.fg_muted))
                            .when(above, |words| words.pb_0p5())
                            .when(!above, |words| words.pt_0p5()),
                    ),
            );
        }
    }
    if !levels.is_empty() {
        out.push(
            div()
                .absolute()
                .left(Pixels::from(main.x))
                .top(Pixels::from(main.y))
                .w(Pixels::from(main.w))
                .h(Pixels::from(main.h))
                .overflow_hidden()
                .children(levels),
        );
    }
    let pill = |y: f32, words: String, (fill, ink): (Hsla, Hsla)| {
        let body = tabular(div())
            .px_1p5()
            .py_0p5()
            .rounded(theme.radius(Radius::Sm))
            .bg(fill)
            .text_size(small)
            .text_color(ink)
            .whitespace_nowrap()
            .child(words);
        div()
            .absolute()
            .left(Pixels::from(edge))
            .top(Pixels::from(y))
            .h_0()
            .flex()
            .items_center()
            .pl_1()
            .child(body)
    };
    let number = |value: f64| format::number(value, digits, format::Separators::EN);
    let shown = |value: f64| (price.domain.0..=price.domain.1).contains(&value);
    for (value, label) in alerts.iter().filter(|(value, _)| shown(*value)) {
        out.push(pill(
            price.at(*value),
            format!("{label} {}", number(*value)),
            (colors.warning_subtle, colors.fg),
        ));
    }
    if let Some((value, rose)) = last.filter(|(value, _)| shown(*value)) {
        out.push(pill(
            price.at(value),
            number(value),
            (if rose { rise } else { fall }, colors.bg),
        ));
    }
    if let Some((pane, y)) = axes.row {
        let (_, scale, ticks) = &axes.panes[pane];
        let digits = if pane == 0 {
            digits
        } else {
            places(ticks).min(2)
        };
        out.push(pill(
            y,
            format::number(scale.value(y), digits, format::Separators::EN),
            (colors.fg, colors.bg),
        ));
    }
    if let Some(ix) = axes.hover.filter(|ix| (*ix as usize) < candles.len()) {
        let x = axes.visible.x(ix as f64, (main.x, main.w));
        let body = div()
            .px_1p5()
            .py_0p5()
            .rounded(theme.radius(Radius::Sm))
            .bg(colors.fg)
            .text_size(small)
            .text_color(colors.bg)
            .whitespace_nowrap();
        let label = body.child(stamp(&candles[ix as usize], interval, &axes.zone, true));
        out.push(
            div()
                .absolute()
                .left(Pixels::from(x))
                .top(Pixels::from(bottom))
                .w_0()
                .flex()
                .justify_center()
                .pt_1()
                .child(label),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use jiff::{Timestamp, tz::TimeZone};

    use super::*;

    #[test]
    fn times_read_by_the_candles_interval() {
        let candle = Candle::new(
            Timestamp::from_second(1_780_000_000).expect("a time"),
            (1.0, 1.0, 1.0, 1.0),
            0.0,
        );
        let utc = TimeZone::UTC;
        assert_eq!(stamp(&candle, 86_400, &utc, false), "May 28");
        assert_eq!(stamp(&candle, 3_600, &utc, false), "20:26");
        assert_eq!(stamp(&candle, 86_400 * 30, &utc, false), "May 2026");
        let unknown = TimeZone::unknown();
        assert_eq!(stamp(&candle, 3_600, &unknown, false), "20:26 UTC");
        assert_eq!(places(&[100.0, 105.0]), 2, "prices keep cents");
        assert_eq!(places(&[0.001, 0.002]), 3);
    }
}
