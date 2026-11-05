//! How a background picture fills its box: its size, its position and its repeat.

use std::{fmt, str::FromStr};

use anyhow::{Context as _, bail, ensure};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

/// How a picture fills its box, as CSS's `background-size`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum PictureSize {
    /// Scaled to cover the box, the parts past it cut off.
    #[default]
    Cover,
    /// Scaled to fit inside the box.
    Contain,
    /// Stretched to the box.
    Fill,
    /// Its own size in points.
    Auto,
    /// A width and a height, each a length or `auto`.
    Explicit(PictureLength, PictureLength),
}

impl PictureSize {
    /// The box `intrinsic` points of picture take in `container`, in points.
    pub fn points(self, intrinsic: (f32, f32), container: (f32, f32)) -> (f32, f32) {
        let (width, height) = intrinsic;
        match self {
            Self::Cover => {
                let scale = (container.0 / width).max(container.1 / height);
                (width * scale, height * scale)
            }
            Self::Contain => {
                let scale = (container.0 / width).min(container.1 / height);
                (width * scale, height * scale)
            }
            Self::Fill => container,
            Self::Auto => intrinsic,
            Self::Explicit(width_length, height_length) => {
                match (
                    width_length.points(container.0),
                    height_length.points(container.1),
                ) {
                    (Some(width), Some(height)) => (width, height),
                    (Some(width), None) => (width, height * width / intrinsic.0),
                    (None, Some(height)) => (width * height / intrinsic.1, height),
                    (None, None) => intrinsic,
                }
            }
        }
    }
}

impl fmt::Display for PictureSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cover => f.write_str("cover"),
            Self::Contain => f.write_str("contain"),
            Self::Fill => f.write_str("fill"),
            Self::Auto => f.write_str("auto"),
            Self::Explicit(width, height) => write!(f, "{width} {height}"),
        }
    }
}

impl FromStr for PictureSize {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let words: Vec<&str> = text.split_whitespace().collect();
        ensure!(!words.is_empty(), "size is empty");
        ensure!(words.len() <= 2, "size is {text:?}, more than two lengths");
        Ok(match words.as_slice() {
            ["cover"] => Self::Cover,
            ["contain"] => Self::Contain,
            ["fill"] => Self::Fill,
            ["auto"] => Self::Auto,
            [width, height] => Self::Explicit(width.parse()?, height.parse()?),
            [width] => Self::Explicit(width.parse()?, PictureLength::Auto),
            _ => unreachable!("at most two words"),
        })
    }
}

impl Serialize for PictureSize {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for PictureSize {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}

/// One side of a picture's size: points, a percentage of the box, or `auto`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PictureLength {
    /// The picture's own side.
    Auto,
    /// Points.
    Px(f32),
    /// A percentage of the box.
    Percent(f32),
}

impl PictureLength {
    /// Its points in `extent`; `None` for `auto`.
    pub fn points(self, extent: f32) -> Option<f32> {
        match self {
            Self::Auto => None,
            Self::Px(points) => Some(points),
            Self::Percent(percent) => Some(extent * percent / 100.0),
        }
    }
}

impl fmt::Display for PictureLength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auto => f.write_str("auto"),
            Self::Px(points) => write!(f, "{points}px"),
            Self::Percent(percent) => write!(f, "{percent}%"),
        }
    }
}

impl FromStr for PictureLength {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        if text == "auto" {
            return Ok(Self::Auto);
        }
        let (number, percent) = match text.strip_suffix('%') {
            Some(number) => (number, true),
            None => (text.strip_suffix("px").unwrap_or(text), false),
        };
        let value: f32 = number
            .parse()
            .with_context(|| format!("length is {text:?}"))?;
        ensure!(
            value.is_finite() && value >= 0.0,
            "length is {text:?}, not a size"
        );
        Ok(if percent {
            Self::Percent(value)
        } else {
            Self::Px(value)
        })
    }
}

/// One side of a picture's place in its box.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum PictureAxis {
    /// Centered in the free room.
    #[default]
    Center,
    /// The box's start: its left or top.
    Start,
    /// The box's end: its right or bottom.
    End,
    /// A percentage of the free room.
    Percent(f32),
    /// Points from the box's start.
    Px(f32),
}

impl PictureAxis {
    /// Its offset into `free` points of room.
    pub fn points(self, free: f32) -> f32 {
        match self {
            Self::Center => free / 2.0,
            Self::Start => 0.0,
            Self::End => free,
            Self::Percent(percent) => free * percent / 100.0,
            Self::Px(points) => points,
        }
    }
}

/// Where a picture sits in its box, as CSS's `background-position`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PicturePosition {
    /// Its horizontal place.
    pub x: PictureAxis,
    /// Its vertical place.
    pub y: PictureAxis,
}

impl PicturePosition {
    /// Its offsets into `free` points of room, across and down.
    pub fn points(self, free: (f32, f32)) -> (f32, f32) {
        (self.x.points(free.0), self.y.points(free.1))
    }
}

impl fmt::Display for PicturePosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if (self.x, self.y) == (PictureAxis::Center, PictureAxis::Center) {
            return f.write_str("center");
        }
        write!(
            f,
            "{} {}",
            side(self.x, "left", "right"),
            side(self.y, "top", "bottom")
        )
    }
}

/// `axis` spelled as CSS names a side.
fn side(axis: PictureAxis, start: &str, end: &str) -> String {
    match axis {
        PictureAxis::Center => "center".to_owned(),
        PictureAxis::Start => start.to_owned(),
        PictureAxis::End => end.to_owned(),
        PictureAxis::Percent(percent) => format!("{percent}%"),
        PictureAxis::Px(points) => format!("{points}px"),
    }
}

/// One word of a place: a side's keyword, `center`, or an offset.
enum Word {
    X(PictureAxis),
    Y(PictureAxis),
    Center,
    Offset(PictureAxis),
}

fn word(word: &str, text: &str) -> anyhow::Result<Word> {
    Ok(match word {
        "left" => Word::X(PictureAxis::Start),
        "right" => Word::X(PictureAxis::End),
        "top" => Word::Y(PictureAxis::Start),
        "bottom" => Word::Y(PictureAxis::End),
        "center" => Word::Center,
        offset => {
            let (number, percent) = match offset.strip_suffix('%') {
                Some(number) => (number, true),
                None => (offset.strip_suffix("px").unwrap_or(offset), false),
            };
            let value: f32 = number
                .parse()
                .with_context(|| format!("position is {text:?}, {offset:?} places nothing"))?;
            ensure!(
                value.is_finite(),
                "position is {text:?}, {offset:?} is no number"
            );
            Word::Offset(if percent {
                PictureAxis::Percent(value)
            } else {
                PictureAxis::Px(value)
            })
        }
    })
}

impl FromStr for PicturePosition {
    type Err = anyhow::Error;

    /// CSS's one and two values: an offset first is across, second down.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        use PictureAxis::Center as Mid;
        let words: Vec<&str> = text.split_whitespace().collect();
        let (x, y) = match words.as_slice() {
            [] => bail!("position is empty"),
            [one] => match word(one, text)? {
                Word::X(x) | Word::Offset(x) => (x, Mid),
                Word::Y(y) => (Mid, y),
                Word::Center => (Mid, Mid),
            },
            [first, second] => match (word(first, text)?, word(second, text)?) {
                (Word::X(x) | Word::Offset(x), Word::Y(y) | Word::Offset(y)) => (x, y),
                (Word::X(x) | Word::Offset(x), Word::Center) => (x, Mid),
                (Word::Center, Word::Y(y) | Word::Offset(y)) => (Mid, y),
                (Word::Y(y), Word::X(x)) => (x, y),
                (Word::Y(y), Word::Center) => (Mid, y),
                (Word::Center, Word::X(x)) => (x, Mid),
                (Word::Center, Word::Center) => (Mid, Mid),
                _ => bail!("position is {text:?}, two places on one side"),
            },
            _ => bail!("position is {text:?}, more than two places"),
        };
        Ok(Self { x, y })
    }
}

impl Serialize for PicturePosition {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for PicturePosition {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}

/// Whether a picture repeats to fill its box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PictureRepeat {
    /// Drawn once.
    #[default]
    NoRepeat,
    /// Tiled across and down.
    Repeat,
    /// Tiled across.
    RepeatX,
    /// Tiled down.
    RepeatY,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_size_keeps_each_side_as_written() {
        for text in [
            "cover",
            "contain",
            "fill",
            "auto",
            "100% 100%",
            "auto 30px",
            "12px auto",
        ] {
            let size: PictureSize = text.parse().expect(text);
            assert_eq!(size.to_string(), text);
        }
        assert_eq!(
            "51%".parse::<PictureSize>().expect("a size").to_string(),
            "51% auto"
        );
        assert_eq!(
            "51%".parse::<PictureSize>().expect("a size"),
            PictureSize::Explicit(PictureLength::Percent(51.0), PictureLength::Auto)
        );
    }

    #[test]
    fn a_size_refuses_what_is_not_one() {
        for text in ["", "cover cover", "-5px", "wide", "10px 20px 30px"] {
            assert!(text.parse::<PictureSize>().is_err(), "{text:?}");
        }
    }

    #[test]
    fn a_side_keeps_its_points_and_percentages() {
        assert_eq!(
            PictureSize::Cover.points((200.0, 100.0), (100.0, 100.0)),
            (200.0, 100.0)
        );
        assert_eq!(
            PictureSize::Contain.points((200.0, 100.0), (100.0, 100.0)),
            (100.0, 50.0)
        );
        assert_eq!(
            PictureSize::Fill.points((200.0, 100.0), (100.0, 50.0)),
            (100.0, 50.0)
        );
        assert_eq!(
            PictureSize::Auto.points((200.0, 100.0), (100.0, 100.0)),
            (200.0, 100.0)
        );
        let half: PictureSize = "50%".parse().expect("a size");
        assert_eq!(half.points((200.0, 100.0), (100.0, 100.0)), (50.0, 25.0));
    }

    #[test]
    fn a_position_keeps_each_side_as_written() {
        for text in [
            "center",
            "left top",
            "right bottom",
            "125% 100%",
            "10px center",
            "center 10px",
            "-10px 20px",
            "-5% center",
        ] {
            let position: PicturePosition = text.parse().expect(text);
            assert_eq!(position.to_string(), text);
        }
        assert_eq!(
            "right top".parse::<PicturePosition>().expect("a place"),
            PicturePosition {
                x: PictureAxis::End,
                y: PictureAxis::Start
            }
        );
        let down: PicturePosition = "center 10px".parse().expect("a place");
        assert_eq!(
            (down.x, down.y),
            (PictureAxis::Center, PictureAxis::Px(10.0))
        );
        let across: PicturePosition = "top left".parse().expect("a place");
        assert_eq!(
            (across.x, across.y),
            (PictureAxis::Start, PictureAxis::Start)
        );
    }

    #[test]
    fn a_position_refuses_what_is_not_one() {
        for text in [
            "",
            "auto",
            "left right top",
            "left right",
            "top bottom",
            "10px left",
            "top 10px",
            "NaNpx",
        ] {
            assert!(text.parse::<PicturePosition>().is_err(), "{text:?}");
        }
    }

    #[test]
    fn a_place_keeps_its_offset() {
        let center = PicturePosition::default();
        assert_eq!(center.points((100.0, 50.0)), (50.0, 25.0));
        let end: PicturePosition = "100% 100%".parse().expect("a place");
        assert_eq!(end.points((100.0, 50.0)), (100.0, 50.0));
        let start: PicturePosition = "left top".parse().expect("a place");
        assert_eq!(start.points((100.0, 50.0)), (0.0, 0.0));
    }
}
