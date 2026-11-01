//! Colors a background style names.

use anyhow::{Context as _, bail, ensure};
use gpui::{Hsla, Rgba};

use super::function;

/// A color: `#rgb`, `#rrggbb`, `rgb()`, `rgba()`, `hsl()`, `hsla()`, `black`, `white` or `transparent`.
pub fn color(text: &str) -> anyhow::Result<Hsla> {
    let text = text.trim();
    if text.eq_ignore_ascii_case("transparent") {
        return Ok(Rgba {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }
        .into());
    }
    if text.eq_ignore_ascii_case("black") {
        return Ok(Rgba {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        }
        .into());
    }
    if text.eq_ignore_ascii_case("white") {
        return Ok(Rgba {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }
        .into());
    }
    if let Some(digits) = text.strip_prefix('#') {
        return hex(digits).with_context(|| format!("`{text}`"));
    }
    for (name, alpha) in [("rgb", false), ("rgba", true)] {
        if let Some(arguments) = function(text, name) {
            return rgb(arguments, alpha).with_context(|| format!("`{text}`"));
        }
    }
    for (name, alpha) in [("hsl", false), ("hsla", true)] {
        if let Some(arguments) = function(text, name) {
            return hsl(arguments, alpha).with_context(|| format!("`{text}`"));
        }
    }
    bail!("`{text}` is no color")
}

/// `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa`.
fn hex(digits: &str) -> anyhow::Result<Hsla> {
    let digits = digits.trim();
    let channels: Vec<u8> = match digits.len() {
        3 | 4 => digits
            .chars()
            .map(|digit| {
                let value = digit
                    .to_digit(16)
                    .with_context(|| format!("`{digit}` is no hex digit"))?;
                Ok((value as u8) * 17)
            })
            .collect::<anyhow::Result<_>>()?,
        6 | 8 => digits
            .as_bytes()
            .chunks(2)
            .map(|pair| {
                let pair = std::str::from_utf8(pair).expect("hex digits are ASCII");
                u8::from_str_radix(pair, 16).with_context(|| format!("`{pair}` is no hex pair"))
            })
            .collect::<anyhow::Result<_>>()?,
        other => bail!("{other} hex digits, not 3, 4, 6 or 8"),
    };
    let alpha = channels.get(3).map_or(1.0, |byte| f32::from(*byte) / 255.0);
    let channel = |at: usize| f32::from(channels[at]) / 255.0;
    Ok(Rgba {
        r: channel(0),
        g: channel(1),
        b: channel(2),
        a: alpha,
    }
    .into())
}

/// `rgb(r, g, b)` or `rgba(r, g, b, a)`.
fn rgb(arguments: &str, alpha: bool) -> anyhow::Result<Hsla> {
    let parts: Vec<&str> = arguments.split(',').map(str::trim).collect();
    ensure!(
        parts.len() == if alpha { 4 } else { 3 },
        "rgb takes 3 values, or 4 with an alpha"
    );
    let channel = |text: &str| -> anyhow::Result<f32> {
        match text.strip_suffix('%') {
            Some(number) => Ok(number.trim().parse::<f32>()? / 100.0),
            None => Ok(text.parse::<f32>()? / 255.0),
        }
    };
    let opacity = if alpha {
        let text = parts[3];
        match text.strip_suffix('%') {
            Some(number) => number.trim().parse::<f32>()? / 100.0,
            None => text.parse::<f32>()?,
        }
    } else {
        1.0
    };
    ensure!((0.0..=1.0).contains(&opacity), "an alpha is 0 to 1");
    Ok(Rgba {
        r: channel(parts[0])?,
        g: channel(parts[1])?,
        b: channel(parts[2])?,
        a: opacity,
    }
    .into())
}

/// `hsl(h, s%, l%)` or `hsla(h, s%, l%, a)`.
fn hsl(arguments: &str, alpha: bool) -> anyhow::Result<Hsla> {
    let parts: Vec<&str> = arguments.split(',').map(str::trim).collect();
    ensure!(
        parts.len() == if alpha { 4 } else { 3 },
        "hsl takes 3 values, or 4 with an alpha"
    );
    let degrees = parts[0].trim_end_matches("deg").trim();
    let hue: f32 = degrees.parse()?;
    let percent = |text: &str| -> anyhow::Result<f32> {
        let number = text.strip_suffix('%').unwrap_or(text);
        let value: f32 = number.trim().parse()?;
        Ok(if text.ends_with('%') {
            value / 100.0
        } else {
            value
        })
    };
    let saturation = percent(parts[1])?;
    let lightness = percent(parts[2])?;
    let opacity = if alpha {
        let text = parts[3];
        match text.strip_suffix('%') {
            Some(number) => number.trim().parse::<f32>()? / 100.0,
            None => text.parse::<f32>()?,
        }
    } else {
        1.0
    };
    ensure!(
        (0.0..=1.0).contains(&saturation) && (0.0..=1.0).contains(&lightness),
        "saturation and lightness are 0 to 100%"
    );
    ensure!((0.0..=1.0).contains(&opacity), "an alpha is 0 to 1");
    let (r, g, b) = hsl_to_rgb(hue.rem_euclid(360.0), saturation, lightness);
    Ok(Rgba {
        r,
        g,
        b,
        a: opacity,
    }
    .into())
}

/// `hue` in degrees, `saturation` and `lightness` from 0 to 1, as RGB.
fn hsl_to_rgb(hue: f32, saturation: f32, lightness: f32) -> (f32, f32, f32) {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let sector = hue / 60.0;
    let second = chroma * (1.0 - (sector % 2.0 - 1.0).abs());
    let (r, g, b) = match sector as u32 {
        0 => (chroma, second, 0.0),
        1 => (second, chroma, 0.0),
        2 => (0.0, chroma, second),
        3 => (0.0, second, chroma),
        4 => (second, 0.0, chroma),
        _ => (chroma, 0.0, second),
    };
    let lift = lightness - chroma / 2.0;
    (r + lift, g + lift, b + lift)
}
