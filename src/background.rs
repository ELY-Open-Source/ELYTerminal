//! The terminal's background, as the settings describe it in CSS-like declarations.

use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail, ensure};
use gpui::Hsla;

mod colors;
mod gradient;
mod mask;
mod paint;

pub use colors::color;
use gradient::linear;
pub use mask::Mask;
pub use paint::render;

/// The most layers one background paints.
pub const MAX_LAYERS: usize = 8;

/// The most stops one gradient holds, as gpui draws them.
pub const MAX_STOPS: usize = 2;

/// What the window paints behind its content.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Background {
    /// Under every layer, as CSS paints `background-color`.
    pub color: Option<Hsla>,
    /// Topmost first, as CSS lists them.
    pub layers: Vec<Layer>,
    /// The whole background's opacity, as CSS's `opacity`.
    pub opacity: Option<f32>,
    /// The gradient every picture fades through, as CSS's `mask-image`.
    pub mask: Option<Mask>,
}

impl Background {
    /// Whether it paints anything.
    pub fn is_empty(&self) -> bool {
        self.color.is_none() && self.layers.is_empty()
    }
}

/// One thing a background paints, with how it fills the window.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub paint: Paint,
    pub size: Size,
    pub position: Position,
    pub repeat: Repeat,
}

impl Layer {
    fn new(paint: Paint) -> Self {
        Self {
            paint,
            size: Size::default(),
            position: Position::default(),
            repeat: Repeat::default(),
        }
    }
}

/// What a layer paints: a picture or a gradient.
#[derive(Clone, Debug, PartialEq)]
pub enum Paint {
    Image(PathBuf),
    Gradient(Gradient),
}

/// A linear gradient, in CSS's terms.
#[derive(Clone, Debug, PartialEq)]
pub struct Gradient {
    /// Degrees clockwise from up, as CSS turns.
    pub angle: f32,
    pub stops: Vec<Stop>,
}

/// One color of a gradient, and where it sits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stop {
    pub color: Hsla,
    /// From 0 at the start to 1 at the end.
    pub at: f32,
}

/// How a picture fills its box, as CSS's `background-size`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Size {
    #[default]
    Cover,
    Contain,
    /// Stretched to the window.
    Fill,
    /// Its own size.
    Auto,
    /// A width and a height, each a length or `auto`.
    Explicit(Length, Length),
}

/// A width or a height of an explicit size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Length {
    Auto,
    Px(f32),
    /// A share of the window, 0 to 1.
    Percent(f32),
}

/// Where a picture sits, as CSS's `background-position`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Position {
    pub x: Axis,
    pub y: Axis,
}

/// One side of a position.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Axis {
    #[default]
    Center,
    /// The left or the top.
    Start,
    /// The right or the bottom.
    End,
    Percent(f32),
    Px(f32),
}

/// How a picture repeats, as CSS's `background-repeat`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Repeat {
    #[default]
    NoRepeat,
    Repeat,
    RepeatX,
    RepeatY,
}

/// Reads the declarations `text` holds; an error names the declaration it could not read.
pub fn parse(text: &str) -> anyhow::Result<Background> {
    let text = uncomment(text)?;
    let mut background = Background::default();
    let mut paints: Vec<Paint> = Vec::new();
    let (mut sizes, mut positions, mut repeats) = (Vec::new(), Vec::new(), Vec::new());
    let (mut mask_image, mut mask_size, mut mask_position) = (None, None, None);
    let mut opacity = None;
    for declaration in declarations(&text)? {
        let declaration = declaration.trim();
        if declaration.is_empty() {
            continue;
        }
        let (property, value) = declaration
            .split_once(':')
            .with_context(|| format!("`{declaration}` is no `property: value`"))?;
        let property = property.trim().to_ascii_lowercase();
        let value = value.trim();
        match property.as_str() {
            "background-color" => {
                background.color =
                    Some(color(value).with_context(|| format!("background-color: `{value}`"))?);
            }
            "background" => {
                if let Some(arguments) = function(value, "linear-gradient") {
                    paints = vec![Paint::Gradient(linear(arguments)?)];
                } else if function(value, "url").is_some() {
                    paints = vec![layer_paint(value)?];
                } else {
                    background.color =
                        Some(color(value).with_context(|| format!("background: `{value}`"))?);
                }
            }
            "background-image" => paints = paints_of(value)?,
            "background-size" => sizes = sizes_of(value)?,
            "background-position" => positions = positions_of(value)?,
            "background-repeat" => repeats = repeats_of(value)?,
            "opacity" => opacity = Some(share(value)?),
            "mask-image" => {
                if value.eq_ignore_ascii_case("none") {
                    mask_image = None;
                } else if let Some(arguments) = function(value, "linear-gradient") {
                    mask_image = Some(mask::gradient(arguments)?);
                } else {
                    bail!("`{value}` is no `linear-gradient()` mask");
                }
            }
            "mask-size" => mask_size = sizes_of(value)?.into_iter().next(),
            "mask-position" => mask_position = positions_of(value)?.into_iter().next(),
            "mask-repeat" => {
                ensure!(
                    value.eq_ignore_ascii_case("no-repeat"),
                    "`{value}` is no mask repeat; only `no-repeat` is supported"
                );
            }
            other => bail!("`{other}` is no background property"),
        }
    }
    background.layers = paints
        .into_iter()
        .enumerate()
        .map(|(at, paint)| {
            let mut layer = Layer::new(paint);
            if let Some(size) = sizes.get(at % sizes.len().max(1)) {
                layer.size = *size;
            }
            if let Some(position) = positions.get(at % positions.len().max(1)) {
                layer.position = *position;
            }
            if let Some(repeat) = repeats.get(at % repeats.len().max(1)) {
                layer.repeat = *repeat;
            }
            layer
        })
        .collect();
    background.opacity = opacity;
    background.mask = mask_image.map(|gradient| Mask {
        gradient,
        size: mask_size.unwrap_or(Size::Auto),
        position: mask_position.unwrap_or(Position {
            x: Axis::Start,
            y: Axis::Start,
        }),
    });
    Ok(background)
}

/// The pictures `text` names, for a caller that reads them afresh.
pub fn images(text: &str) -> Vec<PathBuf> {
    let Ok(background) = parse(text) else {
        return Vec::new();
    };
    background
        .layers
        .into_iter()
        .filter_map(|layer| match layer.paint {
            Paint::Image(path) => Some(path),
            Paint::Gradient(_) => None,
        })
        .collect()
}

/// The declarations between `;`s, comments gone.
fn declarations(text: &str) -> anyhow::Result<Vec<&str>> {
    split(text, ';')
}

/// `text` split at `separator` outside parentheses and quotes.
pub(super) fn split(text: &str, separator: char) -> anyhow::Result<Vec<&str>> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
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
                each if each == separator && depth == 0 => {
                    parts.push(&text[start..index]);
                    start = index + 1;
                }
                _ => {}
            },
        }
    }
    ensure!(quote.is_none(), "a quote never ends");
    ensure!(depth == 0, "a parenthesis never ends");
    parts.push(&text[start..]);
    Ok(parts)
}

/// `text` without its `/* ... */` comments.
fn uncomment(text: &str) -> anyhow::Result<String> {
    let mut kept = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(begin) = rest.find("/*") {
        kept.push_str(&rest[..begin]);
        let after = &rest[begin + 2..];
        let end = after.find("*/").context("a `/*` comment never ends")?;
        rest = &after[end + 2..];
    }
    kept.push_str(rest);
    Ok(kept)
}

/// The pictures `background-image` lists, topmost first.
fn paints_of(value: &str) -> anyhow::Result<Vec<Paint>> {
    if value.trim().eq_ignore_ascii_case("none") {
        return Ok(Vec::new());
    }
    let parts = split(value, ',')?;
    ensure!(
        parts.len() <= MAX_LAYERS,
        "{} layers, past {MAX_LAYERS}",
        parts.len()
    );
    parts.iter().map(|part| layer_paint(part)).collect()
}

/// One picture: a file or a gradient.
fn layer_paint(text: &str) -> anyhow::Result<Paint> {
    let text = text.trim();
    if let Some(arguments) = function(text, "url") {
        let path = unquote(arguments.trim());
        ensure!(!path.is_empty(), "`url()` names no file");
        let path = Path::new(path);
        ensure!(
            path.is_absolute(),
            "`{}` is not a full path",
            path.display()
        );
        return Ok(Paint::Image(path.to_path_buf()));
    }
    if let Some(arguments) = function(text, "linear-gradient") {
        return Ok(Paint::Gradient(linear(arguments)?));
    }
    if function(text, "radial-gradient").is_some() {
        bail!("`radial-gradient` is not supported; `linear-gradient` is");
    }
    bail!("`{text}` is no `url()` or `linear-gradient()` layer")
}

/// `name(arguments)`'s arguments, when `text` is that call in any case.
pub(super) fn function<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let head = text.get(..name.len())?;
    if !head.eq_ignore_ascii_case(name) {
        return None;
    }
    text[name.len()..]
        .trim_start()
        .strip_prefix('(')?
        .strip_suffix(')')
}

/// `text` without the quotes around it, when it wears a pair.
fn unquote(text: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(inner) = text
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_suffix(quote))
        {
            return inner;
        }
    }
    text
}

/// The sizes `background-size` lists, cycling over the layers.
fn sizes_of(value: &str) -> anyhow::Result<Vec<Size>> {
    let parts = split(value, ',')?;
    ensure!(!parts.is_empty(), "`background-size` names none");
    parts.iter().map(|part| size(part)).collect()
}

/// One layer's size.
fn size(text: &str) -> anyhow::Result<Size> {
    let text = text.trim();
    let lower = text.to_ascii_lowercase();
    match lower.as_str() {
        "cover" => return Ok(Size::Cover),
        "contain" => return Ok(Size::Contain),
        "fill" | "100% 100%" => return Ok(Size::Fill),
        "auto" => return Ok(Size::Auto),
        _ => {}
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    match words.len() {
        1 => Ok(Size::Explicit(length(words[0])?, Length::Auto)),
        2 => Ok(Size::Explicit(length(words[0])?, length(words[1])?)),
        other => bail!("`{text}` holds {other} values, not one or two"),
    }
}

/// A length: `auto`, pixels or a percentage.
fn length(text: &str) -> anyhow::Result<Length> {
    let text = text.trim();
    if text.eq_ignore_ascii_case("auto") {
        return Ok(Length::Auto);
    }
    let value = if let Some(number) = text.strip_suffix("px") {
        Length::Px(number_of(number, text)?)
    } else if let Some(number) = text.strip_suffix('%') {
        Length::Percent(number_of(number, text)? / 100.0)
    } else {
        Length::Px(number_of(text, text)?)
    };
    match value {
        Length::Px(px) => ensure!(px >= 0.0, "`{text}` is negative"),
        Length::Percent(share) => ensure!(share >= 0.0, "`{text}` is negative"),
        Length::Auto => {}
    }
    Ok(value)
}

/// The positions `background-position` lists.
fn positions_of(value: &str) -> anyhow::Result<Vec<Position>> {
    let parts = split(value, ',')?;
    ensure!(!parts.is_empty(), "`background-position` names none");
    parts.iter().map(|part| position(part)).collect()
}

/// One layer's position.
fn position(text: &str) -> anyhow::Result<Position> {
    let words: Vec<&str> = text.split_whitespace().collect();
    ensure!(
        (1..=2).contains(&words.len()),
        "`{text}` holds {} values, not one or two",
        words.len()
    );
    let mut x = None;
    let mut y = None;
    for word in &words {
        let lower = word.to_ascii_lowercase();
        match lower.as_str() {
            "left" => x = Some(Axis::Start),
            "right" => x = Some(Axis::End),
            "top" => y = Some(Axis::Start),
            "bottom" => y = Some(Axis::End),
            "center" => {}
            _ => match length(word)? {
                Length::Px(px) => {
                    if x.is_none() {
                        x = Some(Axis::Px(px));
                    } else {
                        y = Some(Axis::Px(px));
                    }
                }
                Length::Percent(share) => {
                    if x.is_none() {
                        x = Some(Axis::Percent(share));
                    } else {
                        y = Some(Axis::Percent(share));
                    }
                }
                Length::Auto => bail!("`{word}` is no position"),
            },
        }
    }
    let position = Position {
        x: x.unwrap_or(Axis::Center),
        y: y.unwrap_or(Axis::Center),
    };
    Ok(position)
}

/// The repeats `background-repeat` lists.
fn repeats_of(value: &str) -> anyhow::Result<Vec<Repeat>> {
    let parts = split(value, ',')?;
    ensure!(!parts.is_empty(), "`background-repeat` names none");
    parts.iter().map(|part| repeat(part)).collect()
}

/// One layer's repeat.
fn repeat(text: &str) -> anyhow::Result<Repeat> {
    match text.trim().to_ascii_lowercase().as_str() {
        "no-repeat" => Ok(Repeat::NoRepeat),
        "repeat" => Ok(Repeat::Repeat),
        "repeat-x" => Ok(Repeat::RepeatX),
        "repeat-y" => Ok(Repeat::RepeatY),
        other => bail!("`{other}` is not no-repeat, repeat, repeat-x or repeat-y"),
    }
}

/// A share from 0 to 1, as `opacity: 0.25` or `25%`.
fn share(text: &str) -> anyhow::Result<f32> {
    let text = text.trim();
    let (number, scale) = match text.strip_suffix('%') {
        Some(number) => (number, 100.0),
        None => (text, 1.0),
    };
    let value: f32 = number
        .trim()
        .parse()
        .with_context(|| format!("`{text}` is no number"))?;
    let value = value / scale;
    ensure!((0.0..=1.0).contains(&value), "`{text}` is outside 0 to 1");
    Ok(value)
}

/// A number of a length or a position.
fn number_of(number: &str, whole: &str) -> anyhow::Result<f32> {
    number
        .trim()
        .parse()
        .with_context(|| format!("`{whole}` is no number"))
}

#[cfg(test)]
mod tests;
