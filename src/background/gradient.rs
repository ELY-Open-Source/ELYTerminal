//! Linear gradients a background style names.

use anyhow::{Context as _, bail, ensure};

use super::{Gradient, MAX_STOPS, Stop, color, split};

/// A linear gradient, the number of stops gpui draws.
pub(super) fn linear(arguments: &str) -> anyhow::Result<Gradient> {
    linear_with(arguments, MAX_STOPS).with_context(|| "linear-gradient")
}

/// A linear gradient with at most `most` stops.
pub(super) fn linear_with(arguments: &str, most: usize) -> anyhow::Result<Gradient> {
    let parts = split(arguments, ',')?;
    let (angle, stops) = match direction(parts[0].trim())? {
        Some(angle) => (angle, &parts[1..]),
        None => (180.0, &parts[..]),
    };
    ensure!(stops.len() >= 2, "a gradient needs two stops");
    ensure!(
        stops.len() <= most,
        "a gradient takes {most} stops at most; it has {}",
        stops.len()
    );
    let stops = stops
        .iter()
        .map(|stop| stop_of(stop))
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(Gradient {
        angle,
        stops: placed(stops),
    })
}

/// A gradient's first argument's angle, when it names one rather than a color.
fn direction(text: &str) -> anyhow::Result<Option<f32>> {
    let lower = text.to_ascii_lowercase();
    if let Some(degrees) = lower.strip_suffix("deg") {
        let angle: f32 = degrees
            .trim()
            .parse()
            .with_context(|| format!("`{text}` is no angle"))?;
        return Ok(Some(angle));
    }
    if let Some(rest) = lower.strip_prefix("to ") {
        let side = rest.trim();
        let angle = match side {
            "top" => 0.0,
            "right" => 90.0,
            "bottom" => 180.0,
            "left" => 270.0,
            "top right" | "right top" => 45.0,
            "bottom right" | "right bottom" => 135.0,
            "bottom left" | "left bottom" => 225.0,
            "top left" | "left top" => 315.0,
            other => bail!("`to {other}` is no side"),
        };
        return Ok(Some(angle));
    }
    Ok(None)
}

/// One stop: a color, and where it sits when it says.
fn stop_of(text: &str) -> anyhow::Result<Stop> {
    let text = text.trim();
    let (color_text, at) = match position_tail(text) {
        Some((color_text, at)) => (color_text, Some(at)),
        None => (text, None),
    };
    Ok(Stop {
        color: color(color_text).with_context(|| format!("stop `{text}`"))?,
        at: at.unwrap_or(f32::NAN),
    })
}

/// `text`'s last value, when it ends in a position; the color before it.
fn position_tail(text: &str) -> Option<(&str, f32)> {
    let index = last_top_level_space(text)?;
    let (head, tail) = text.split_at(index);
    let at = position(tail.trim()).ok()?;
    Some((head.trim(), at))
}

/// The last space in `text` outside parentheses and quotes.
fn last_top_level_space(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    let mut found = None;
    for (index, character) in text.char_indices() {
        match quote {
            Some(open) => {
                if character == open {
                    quote = None;
                }
            }
            None => match character {
                '"' | '\'' => quote = Some(character),
                '(' => depth += 1,
                ')' => depth = depth.saturating_sub(1),
                ' ' if depth == 0 => found = Some(index),
                _ => {}
            },
        }
    }
    found
}

/// A stop's position, as `50%` or `0.5`.
fn position(text: &str) -> anyhow::Result<f32> {
    let (number, scale) = match text.strip_suffix('%') {
        Some(number) => (number, 100.0),
        None => (text, 1.0),
    };
    let value: f32 = number
        .trim()
        .parse()
        .with_context(|| format!("`{text}` is no position"))?;
    let at = value / scale;
    ensure!((0.0..=1.0).contains(&at), "`{text}` is outside 0 to 100%");
    Ok(at)
}

/// `stops` with every gap filled and no position going back.
fn placed(mut stops: Vec<Stop>) -> Vec<Stop> {
    let last = stops.len() - 1;
    if stops[0].at.is_nan() {
        stops[0].at = 0.0;
    }
    if stops[last].at.is_nan() {
        stops[last].at = 1.0;
    }
    let mut index = 0;
    while index < stops.len() {
        if !stops[index].at.is_nan() {
            index += 1;
            continue;
        }
        let next = (index..stops.len())
            .find(|&other| !stops[other].at.is_nan())
            .expect("the last stop has a position");
        let (from, to) = (stops[index - 1].at, stops[next].at);
        let span = (next - index + 1) as f32;
        for step in 0..(next - index) {
            stops[index + step].at = from + (to - from) * (step + 1) as f32 / span;
        }
        index = next;
    }
    for index in 1..stops.len() {
        if stops[index].at < stops[index - 1].at {
            stops[index].at = stops[index - 1].at;
        }
    }
    stops
}
