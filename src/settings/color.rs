//! The colors a picture's gradient and mask take: CSS hex, rgb and rgba, and the three names.

use anyhow::{Context as _, bail, ensure};
use gpui::{Hsla, Rgba};

/// Reads `#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb(r, g, b)`, `rgba(r, g, b, a)`, or `transparent`, `black` or `white`.
pub fn color(text: &str) -> anyhow::Result<Hsla> {
    let source = text.trim();
    let lower = source.to_ascii_lowercase();
    match lower.as_str() {
        "transparent" => return Ok(Hsla::transparent_black()),
        "black" => {
            return Ok(Rgba {
                r: 0.,
                g: 0.,
                b: 0.,
                a: 1.,
            }
            .into());
        }
        "white" => {
            return Ok(Rgba {
                r: 1.,
                g: 1.,
                b: 1.,
                a: 1.,
            }
            .into());
        }
        _ => {}
    }
    if let Some(hex) = lower.strip_prefix('#') {
        return hex_color(hex).with_context(|| format!("{source:?} is not a color"));
    }
    for (prefix, alpha) in [("rgba(", true), ("rgb(", false)] {
        if let Some(inside) = lower
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_suffix(')'))
        {
            return channels(inside, alpha).with_context(|| format!("{source:?} is not a color"));
        }
    }
    bail!("{source:?} is not a color")
}

/// A hex color's digits: three, six, or eight of them.
fn hex_color(hex: &str) -> anyhow::Result<Hsla> {
    ensure!(
        hex.chars().all(|digit| digit.is_ascii_hexdigit()),
        "{hex:?} holds a digit that is not hexadecimal"
    );
    let digits: Vec<u8> = hex
        .chars()
        .map(|digit| digit.to_digit(16).expect("a hexadecimal digit") as u8)
        .collect();
    let (red, green, blue, alpha) = match digits.len() {
        3 => (digits[0] * 17, digits[1] * 17, digits[2] * 17, 255),
        6 => (
            digits[0] * 16 + digits[1],
            digits[2] * 16 + digits[3],
            digits[4] * 16 + digits[5],
            255,
        ),
        8 => (
            digits[0] * 16 + digits[1],
            digits[2] * 16 + digits[3],
            digits[4] * 16 + digits[5],
            digits[6] * 16 + digits[7],
        ),
        other => bail!("{hex:?} holds {other} digits, not three, six or eight"),
    };
    Ok(Rgba {
        r: f32::from(red) / 255.,
        g: f32::from(green) / 255.,
        b: f32::from(blue) / 255.,
        a: f32::from(alpha) / 255.,
    }
    .into())
}

/// `rgb`'s three channels, or `rgba`'s four: 0 to 255 each, the alpha a fraction.
fn channels(inside: &str, alpha: bool) -> anyhow::Result<Hsla> {
    let parts: Vec<&str> = inside.split(',').map(str::trim).collect();
    let wanted = if alpha { 4 } else { 3 };
    ensure!(
        parts.len() == wanted,
        "it holds {} values, not {wanted}",
        parts.len()
    );
    let mut values = [0.; 4];
    for (value, part) in values.iter_mut().zip(&parts) {
        *value = part
            .parse::<f32>()
            .with_context(|| format!("{part:?} is not a number"))?;
    }
    ensure!(
        values[..3]
            .iter()
            .all(|channel| (0.0..=255.0).contains(channel)),
        "a channel lies outside 0 to 255"
    );
    ensure!(
        !alpha || (0.0..=1.0).contains(&values[3]),
        "the alpha lies outside 0 to 1"
    );
    Ok(Rgba {
        r: values[0] / 255.,
        g: values[1] / 255.,
        b: values[2] / 255.,
        a: if alpha { values[3] } else { 1. },
    }
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_hex_form_reads_its_channels() {
        let red: Hsla = Rgba {
            r: 1.,
            g: 0.,
            b: 0.,
            a: 1.,
        }
        .into();
        let green: Hsla = Rgba {
            r: 0.,
            g: 1.,
            b: 0.,
            a: 1.,
        }
        .into();
        let white: Hsla = Rgba {
            r: 1.,
            g: 1.,
            b: 1.,
            a: 1.,
        }
        .into();
        let black: Hsla = Rgba {
            r: 0.,
            g: 0.,
            b: 0.,
            a: 1.,
        }
        .into();
        let faded = color("#000000cc").expect("a color");
        assert_eq!(faded.a, 204. / 255.);
        assert_eq!(color("#fff").expect("a color"), white);
        assert_eq!(color("#f00").expect("a color"), red);
        assert_eq!(color("#00ff00").expect("a color"), green);
        assert_eq!(color(" white ").expect("a name"), white);
        assert_eq!(color("black").expect("a name"), black);
        assert_eq!(color("transparent").expect("a name").a, 0.);
    }

    #[test]
    fn rgb_and_rgba_read_their_channels() {
        let red: Hsla = Rgba {
            r: 1.,
            g: 0.,
            b: 0.,
            a: 1.,
        }
        .into();
        assert_eq!(color("rgb(255, 0, 0)").expect("a color"), red);
        let half = color("rgba(0, 0, 0, 0.5)").expect("a color").to_rgb();
        assert_eq!((half.r, half.g, half.b, half.a), (0., 0., 0., 0.5));
    }

    #[test]
    fn what_is_not_a_color_says_why() {
        for refused in [
            "",
            "#12",
            "#12345",
            "#ggg",
            "rgb(1, 2)",
            "rgba(0, 0, 0, 2)",
            "reddish",
        ] {
            assert!(color(refused).is_err(), "{refused:?} reads as a color");
        }
    }
}
