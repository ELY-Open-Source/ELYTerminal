use gpui::SharedString;

use super::view::Frame;
use crate::primitives::IconName;

/// What a shape is.
#[derive(Clone, Debug, PartialEq)]
pub enum ShapeKind {
    Rect,
    Ellipse,
    /// A regular polygon of so many sides, three or more.
    Polygon(u8),
    /// From the opposite corner of the frame to this one.
    Line(Corner),
    /// A line with a head at the corner it ends at.
    Arrow(Corner),
    Text(SharedString),
    /// Points in canvas units from the frame's top left, a line so wide.
    Path {
        points: Vec<(f32, f32)>,
        width: f32,
    },
    /// A square of paper with words on it.
    Note(SharedString),
}

impl ShapeKind {
    /// Its icon in lists and grids.
    pub(crate) fn icon(&self) -> IconName {
        match self {
            ShapeKind::Rect => IconName::Square,
            ShapeKind::Ellipse => IconName::Circle,
            ShapeKind::Polygon(_) => IconName::Hexagon,
            ShapeKind::Line(_) => IconName::Slash,
            ShapeKind::Arrow(_) => IconName::MoveUpRight,
            ShapeKind::Text(_) => IconName::Type,
            ShapeKind::Path { .. } => IconName::PenTool,
            ShapeKind::Note(_) => IconName::StickyNote,
        }
    }

    /// What it is, in a word.
    pub(crate) fn word(&self) -> &'static str {
        match self {
            ShapeKind::Rect => "Rectangle",
            ShapeKind::Ellipse => "Ellipse",
            ShapeKind::Polygon(_) => "Polygon",
            ShapeKind::Line(_) => "Line",
            ShapeKind::Arrow(_) => "Arrow",
            ShapeKind::Text(_) => "Text",
            ShapeKind::Path { .. } => "Path",
            ShapeKind::Note(_) => "Note",
        }
    }
}

/// A frame's corner, where a line or an arrow ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
}

impl Corner {
    /// Its point on `frame`.
    pub(crate) fn of(self, frame: &Frame) -> (f32, f32) {
        match self {
            Corner::TopLeft => (frame.x, frame.y),
            Corner::TopRight => (frame.right(), frame.y),
            Corner::BottomRight => (frame.right(), frame.bottom()),
            Corner::BottomLeft => (frame.x, frame.bottom()),
        }
    }

    pub(crate) fn opposite(self) -> Corner {
        match self {
            Corner::TopLeft => Corner::BottomRight,
            Corner::TopRight => Corner::BottomLeft,
            Corner::BottomRight => Corner::TopLeft,
            Corner::BottomLeft => Corner::TopRight,
        }
    }
}

/// Something on a canvas: its key, its name, what it is, its frame, its hue among the chart colors, and whether it hides or is locked.
#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    pub key: SharedString,
    pub name: SharedString,
    pub kind: ShapeKind,
    pub frame: Frame,
    pub hue: usize,
    pub hidden: bool,
    pub locked: bool,
    /// Words shown in its middle.
    pub label: Option<SharedString>,
}

impl Shape {
    pub fn new(
        key: impl Into<SharedString>,
        name: impl Into<SharedString>,
        kind: ShapeKind,
        frame: Frame,
    ) -> Self {
        if let ShapeKind::Polygon(sides) = kind {
            assert!(sides >= 3, "a polygon of {sides} sides");
        }
        Self {
            key: key.into(),
            name: name.into(),
            kind,
            frame,
            hue: 0,
            hidden: false,
            locked: false,
            label: None,
        }
    }

    pub fn hue(mut self, hue: usize) -> Self {
        self.hue = hue;
        self
    }

    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    pub fn locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    pub fn label(mut self, text: impl Into<SharedString>) -> Self {
        self.label = Some(text.into());
        self
    }

    /// The words it shows: a text's or a note's own, or its label.
    pub fn words(&self) -> Option<&SharedString> {
        match &self.kind {
            ShapeKind::Text(text) | ShapeKind::Note(text) => Some(text),
            _ => self.label.as_ref(),
        }
    }

    /// The shape showing `text`: a text or a note says it, and anything else wears it as its label, or none when it is empty.
    pub fn written(&self, text: impl Into<SharedString>) -> Shape {
        let text = text.into();
        let mut next = self.clone();
        match &mut next.kind {
            ShapeKind::Text(words) | ShapeKind::Note(words) => *words = text,
            _ => next.label = (!text.is_empty()).then_some(text),
        }
        next
    }
}

/// A named board on the canvas that designs sit on.
#[derive(Clone, Debug, PartialEq)]
pub struct Artboard {
    pub key: SharedString,
    pub name: SharedString,
    pub frame: Frame,
}

impl Artboard {
    pub fn new(key: impl Into<SharedString>, name: impl Into<SharedString>, frame: Frame) -> Self {
        Self {
            key: key.into(),
            name: name.into(),
            frame,
        }
    }
}

/// A line across the canvas to line things up with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Guide {
    /// At this canvas x.
    Vertical(f32),
    /// At this canvas y.
    Horizontal(f32),
}

/// The corners of a regular polygon of `sides` in `frame`, the first at the top.
pub(crate) fn polygon(frame: &Frame, sides: u8) -> Vec<(f32, f32)> {
    let (cx, cy) = frame.center();
    (0..sides)
        .map(|ix| {
            let turn = std::f32::consts::TAU * f32::from(ix) / f32::from(sides)
                - std::f32::consts::FRAC_PI_2;
            (
                cx + frame.w / 2.0 * turn.cos(),
                cy + frame.h / 2.0 * turn.sin(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Frame, polygon};

    #[test]
    fn a_polygon_starts_at_the_top_and_goes_round() {
        let corners = polygon(&Frame::new(0.0, 0.0, 100.0, 100.0), 4);
        let round = |(x, y): (f32, f32)| (x.round(), y.round());
        assert_eq!(
            corners.into_iter().map(round).collect::<Vec<_>>(),
            [(50.0, 0.0), (100.0, 50.0), (50.0, 100.0), (0.0, 50.0)]
        );
    }
}
