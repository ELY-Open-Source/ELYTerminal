//! The background's settings, one JSON field each.

use std::path::Path;

use anyhow::{Context as _, ensure};
use serde::{Deserialize, Serialize};

use crate::background as style;

/// How a picture repeats, as the settings name it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Repeat {
    #[default]
    NoRepeat,
    Repeat,
    RepeatX,
    RepeatY,
}

impl Repeat {
    pub fn css(self) -> &'static str {
        match self {
            Self::NoRepeat => "no-repeat",
            Self::Repeat => "repeat",
            Self::RepeatX => "repeat-x",
            Self::RepeatY => "repeat-y",
        }
    }
}

/// A gradient over the picture, from one color to another.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Gradient {
    /// Degrees clockwise from up.
    pub angle: f32,
    /// A color for the start, and one for the end.
    pub from: String,
    pub to: String,
}

impl Default for Gradient {
    fn default() -> Self {
        Self {
            angle: 180.0,
            from: "#000000cc".into(),
            to: "#00000000".into(),
        }
    }
}

/// The gradient that fades the pictures, as CSS's `mask-image`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Mask {
    pub angle: f32,
    /// Colors of the mask's stops, at least two; their alpha is the mask.
    pub stops: Vec<String>,
    /// `mask-size`: a keyword, a share or pixels.
    pub size: String,
    /// `mask-position`: sides, shares or pixels.
    pub position: String,
}

impl Default for Mask {
    fn default() -> Self {
        Self {
            angle: 90.0,
            stops: vec!["transparent".into(), "black".into(), "black".into()],
            size: "40% 100%".into(),
            position: "right top".into(),
        }
    }
}

/// Everything the settings window edits about the background.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Background {
    /// `#rrggbb` or `#rrggbbaa`; empty for none.
    pub color: String,
    /// A full path to a picture; empty for none.
    pub image: String,
    /// `cover`, `contain`, `fill`, `auto`, or a width and a height.
    pub size: String,
    /// `center`, `top left`, `100% 100%`, or pixels.
    pub position: String,
    pub repeat: Repeat,
    /// Over the picture, under the window's content.
    pub gradient: Option<Gradient>,
    /// `0` to `1`, times the window's background opacity.
    pub opacity: f32,
    /// The gradient the pictures fade through; none leaves them sharp.
    pub mask: Option<Mask>,
}

impl Default for Background {
    fn default() -> Self {
        Self {
            color: String::new(),
            image: String::new(),
            size: "cover".into(),
            position: "center".into(),
            repeat: Repeat::NoRepeat,
            gradient: None,
            opacity: 1.0,
            mask: None,
        }
    }
}

impl Background {
    /// Whether it paints anything.
    pub fn is_empty(&self) -> bool {
        self.color.trim().is_empty() && self.image.trim().is_empty() && self.gradient.is_none()
    }

    /// Its style, as the painter reads it; an error names the field.
    pub fn style(&self) -> anyhow::Result<style::Background> {
        style::parse(&self.css()).context("background")
    }

    /// The declarations the settings name, for the painter's one parser.
    fn css(&self) -> String {
        let mut css = String::new();
        if !self.color.trim().is_empty() {
            css.push_str(&format!("background-color: {}; ", self.color.trim()));
        }
        let mut layers = Vec::new();
        if let Some(gradient) = &self.gradient {
            layers.push(format!(
                "linear-gradient({}deg, {}, {})",
                gradient.angle, gradient.from, gradient.to
            ));
        }
        if !self.image.trim().is_empty() {
            layers.push(format!("url(\"{}\")", self.image.trim()));
        }
        if !layers.is_empty() {
            css.push_str(&format!("background-image: {}; ", layers.join(", ")));
            css.push_str(&format!("background-size: {}; ", self.size.trim()));
            css.push_str(&format!("background-position: {}; ", self.position.trim()));
            css.push_str(&format!("background-repeat: {}; ", self.repeat.css()));
        }
        css.push_str(&format!("opacity: {}; ", self.opacity));
        if let Some(mask) = &self.mask {
            css.push_str(&format!(
                "mask-image: linear-gradient({}deg, {}); ",
                mask.angle,
                mask.stops.join(", ")
            ));
            css.push_str(&format!("mask-size: {}; ", mask.size.trim()));
            css.push_str(&format!("mask-position: {}; ", mask.position.trim()));
            css.push_str("mask-repeat: no-repeat; ");
        }
        css
    }

    /// Checks every field; the error names the field.
    pub fn validate(&self) -> anyhow::Result<()> {
        ensure!(
            (0.0..=1.0).contains(&self.opacity),
            "background.opacity is {}, outside 0 to 1",
            self.opacity
        );
        ensure!(
            self.size.split_whitespace().count() <= 2 && !self.size.trim().is_empty(),
            "background.size is {:?}",
            self.size
        );
        self.style()?;
        if !self.image.trim().is_empty() {
            let path = Path::new(self.image.trim());
            ensure!(
                path.is_file(),
                "background.image is {:?}, which names no file",
                self.image
            );
        }
        if let Some(gradient) = &self.gradient {
            ensure!(
                (0.0..=360.0).contains(&gradient.angle),
                "background.gradient.angle is {}, outside 0 to 360",
                gradient.angle
            );
        }
        if let Some(mask) = &self.mask {
            ensure!(
                (0.0..=360.0).contains(&mask.angle),
                "background.mask.angle is {}, outside 0 to 360",
                mask.angle
            );
            ensure!(
                mask.stops.len() >= 2 && mask.stops.len() <= 16,
                "background.mask.stops holds {}, not 2 to 16",
                mask.stops.len()
            );
        }
        Ok(())
    }

    /// The background a parsed style describes, for a file from before these fields.
    pub fn from_style(parsed: &style::Background) -> Self {
        let mut settings = Self {
            color: parsed.color.map(hex).unwrap_or_default(),
            opacity: parsed.opacity.unwrap_or(1.0),
            ..Self::default()
        };
        for layer in &parsed.layers {
            match &layer.paint {
                style::Paint::Image(path) => {
                    settings.image = path.display().to_string();
                    settings.size = size_of(layer.size);
                    settings.position = position_of(layer.position);
                    settings.repeat = repeat_of(layer.repeat);
                }
                style::Paint::Gradient(gradient) => {
                    let [from, to] = &gradient.stops[..] else {
                        continue;
                    };
                    settings.gradient = Some(Gradient {
                        angle: gradient.angle,
                        from: hex(from.color),
                        to: hex(to.color),
                    });
                }
            }
        }
        settings.mask = parsed.mask.as_ref().map(|mask| Mask {
            angle: mask.gradient.angle,
            stops: mask
                .gradient
                .stops
                .iter()
                .map(|stop| hex(stop.color))
                .collect(),
            size: size_of(mask.size),
            position: position_of(mask.position),
        });
        settings
    }
}

/// A color as `#rrggbbaa`.
fn hex(color: gpui::Hsla) -> String {
    let rgba = gpui::Rgba::from(color);
    let byte = |channel: f32| (channel.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "#{:02x}{:02x}{:02x}{:02x}",
        byte(rgba.r),
        byte(rgba.g),
        byte(rgba.b),
        byte(rgba.a)
    )
}

/// A size as CSS names it.
fn size_of(size: style::Size) -> String {
    match size {
        style::Size::Cover => "cover".into(),
        style::Size::Contain => "contain".into(),
        style::Size::Fill => "fill".into(),
        style::Size::Auto => "auto".into(),
        style::Size::Explicit(width, height) => {
            format!("{} {}", length_of(width), length_of(height))
        }
    }
}

fn length_of(length: style::Length) -> String {
    match length {
        style::Length::Auto => "auto".into(),
        style::Length::Px(points) => format!("{points}px"),
        style::Length::Percent(share) => format!("{}%", share * 100.0),
    }
}

/// A position as CSS names it.
fn position_of(position: style::Position) -> String {
    format!(
        "{} {}",
        axis_of(position.x, "left", "right"),
        axis_of(position.y, "top", "bottom")
    )
}

fn axis_of(axis: style::Axis, start: &str, end: &str) -> String {
    match axis {
        style::Axis::Start => start.into(),
        style::Axis::End => end.into(),
        style::Axis::Center => "center".into(),
        style::Axis::Percent(share) => format!("{}%", share * 100.0),
        style::Axis::Px(points) => format!("{points}px"),
    }
}

fn repeat_of(repeat: style::Repeat) -> Repeat {
    match repeat {
        style::Repeat::NoRepeat => Repeat::NoRepeat,
        style::Repeat::Repeat => Repeat::Repeat,
        style::Repeat::RepeatX => Repeat::RepeatX,
        style::Repeat::RepeatY => Repeat::RepeatY,
    }
}
