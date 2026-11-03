use gpui::{FontWeight, Hsla};

/// Styles a span of the text: its color, an optional wash behind it, and its weight, slant and strike.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Highlight {
    pub color: Hsla,
    pub background: Option<Hsla>,
    pub weight: Option<FontWeight>,
    pub italic: bool,
    pub strike: bool,
}

impl Highlight {
    /// A span in `color`, its font as the field's.
    pub fn new(color: Hsla) -> Self {
        Self {
            color,
            background: None,
            weight: None,
            italic: false,
            strike: false,
        }
    }
}
