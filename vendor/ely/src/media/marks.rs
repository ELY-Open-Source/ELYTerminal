use gpui::{Bounds, Hsla, PathBuilder, Pixels, Window, point};

use crate::forms::ink;

/// A mark on a picture, its places in shares of the picture's width and height, its color an index into the chart palette.
#[derive(Clone, Debug, PartialEq)]
pub enum Mark {
    /// A box from one corner to the other.
    Box {
        from: (f32, f32),
        to: (f32, f32),
        color: usize,
    },
    /// An arrow from its tail to its head.
    Arrow {
        from: (f32, f32),
        to: (f32, f32),
        color: usize,
    },
    /// A line drawn freehand.
    Pen {
        points: Vec<(f32, f32)>,
        color: usize,
    },
    /// A numbered pin, counted in the order pins were placed.
    Pin { at: (f32, f32), color: usize },
}

/// How marks look: the line's width, an arrow head's length, the chart colors, and the chosen mark's halo.
#[derive(Clone, Copy)]
pub(crate) struct Look {
    pub stroke: Pixels,
    pub head: Pixels,
    pub palette: [Hsla; 8],
    pub halo: Hsla,
}

/// Paints `marks` and the one being drawn on `bounds`; the chosen one sits on the halo.
pub(crate) fn paint(
    marks: &[Mark],
    sketch: Option<Mark>,
    chosen: Option<usize>,
    look: Look,
    bounds: Bounds<Pixels>,
    window: &mut Window,
) {
    let Look {
        stroke,
        head,
        palette,
        halo,
    } = look;
    let place = |p: (f32, f32)| {
        point(
            bounds.left() + bounds.size.width * p.0,
            bounds.top() + bounds.size.height * p.1,
        )
    };
    let mut draw = |mark: &Mark, width: Pixels, color: Hsla| {
        let path = match mark {
            Mark::Box { from, to, .. } => {
                let mut path = PathBuilder::stroke(width);
                path.move_to(place(*from));
                for corner in [(to.0, from.1), *to, (from.0, to.1), *from] {
                    path.line_to(place(corner));
                }
                path.build().ok()
            }
            Mark::Arrow { from, to, .. } => {
                let (tail, tip) = (place(*from), place(*to));
                let angle = f32::from(tip.y - tail.y).atan2(f32::from(tip.x - tail.x));
                let barb = |turn: f32| {
                    point(
                        tip.x - head * (angle + turn).cos(),
                        tip.y - head * (angle + turn).sin(),
                    )
                };
                let mut path = PathBuilder::stroke(width);
                path.move_to(tail);
                path.line_to(tip);
                path.move_to(barb(0.45));
                path.line_to(tip);
                path.line_to(barb(-0.45));
                path.build().ok()
            }
            Mark::Pen { points, .. } => {
                let points: Vec<_> = points.iter().map(|p| place(*p) - bounds.origin).collect();
                ink(&points, bounds.origin, width)
            }
            Mark::Pin { .. } => None,
        };
        if let Some(path) = path {
            window.paint_path(path, color);
        }
    };
    if let Some(ix) = chosen {
        draw(&marks[ix], stroke * 3.0, halo);
    }
    for mark in marks.iter().chain(&sketch) {
        draw(mark, stroke, palette[mark.color() % palette.len()]);
    }
}

/// What a drag on the picture makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    /// Chooses a mark instead of making one.
    Select,
    Box,
    Arrow,
    Pen,
    Pin,
}

impl Mark {
    /// The mark a tool makes from a stroke of points: none from Select, and none but a pin from a tap in place.
    pub(crate) fn made(tool: Tool, stroke: &[(f32, f32)], color: usize) -> Option<Self> {
        let (&first, &last) = (stroke.first()?, stroke.last()?);
        let moved = stroke.iter().any(|point| *point != first);
        match tool {
            Tool::Select => None,
            Tool::Box | Tool::Arrow if last == first => None,
            Tool::Pen if !moved => None,
            Tool::Box => Some(Self::Box {
                from: first,
                to: last,
                color,
            }),
            Tool::Arrow => Some(Self::Arrow {
                from: first,
                to: last,
                color,
            }),
            Tool::Pen => Some(Self::Pen {
                points: stroke.to_vec(),
                color,
            }),
            Tool::Pin => Some(Self::Pin { at: last, color }),
        }
    }

    pub(crate) fn color(&self) -> usize {
        match self {
            Self::Box { color, .. }
            | Self::Arrow { color, .. }
            | Self::Pen { color, .. }
            | Self::Pin { color, .. } => *color,
        }
    }

    /// How far `at` sits from the mark, measured in pixels of a picture `size` across and down: a box by its edges, a line by its nearest stretch, a pin by its middle.
    pub(crate) fn distance(&self, at: (f32, f32), size: (f32, f32)) -> f32 {
        let scale = |p: (f32, f32)| (p.0 * size.0, p.1 * size.1);
        let at = scale(at);
        let along = |points: &[(f32, f32)]| {
            let points: Vec<_> = points.iter().map(|p| scale(*p)).collect();
            match points.as_slice() {
                [only] => (only.0 - at.0).hypot(only.1 - at.1),
                _ => points
                    .windows(2)
                    .map(|pair| segment(at, pair[0], pair[1]))
                    .fold(f32::INFINITY, f32::min),
            }
        };
        match self {
            Self::Box { from, to, .. } => {
                let (a, b) = (*from, *to);
                along(&[a, (b.0, a.1), b, (a.0, b.1), a])
            }
            Self::Arrow { from, to, .. } => along(&[*from, *to]),
            Self::Pen { points, .. } => along(points),
            Self::Pin { at: pin, .. } => along(&[*pin]),
        }
    }
}

/// How far `p` sits from the stretch between `a` and `b`.
fn segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = dx * dx + dy * dy;
    let t = if length == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length).clamp(0.0, 1.0)
    };
    (p.0 - (a.0 + t * dx)).hypot(p.1 - (a.1 + t * dy))
}

/// The topmost mark within `reach` pixels of `at`, a place in shares of a picture `size` across and down.
pub(crate) fn mark_at(
    marks: &[Mark],
    at: (f32, f32),
    size: (f32, f32),
    reach: f32,
) -> Option<usize> {
    marks
        .iter()
        .enumerate()
        .rev()
        .map(|(ix, mark)| (ix, mark.distance(at, size)))
        .filter(|(_, far)| *far <= reach)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(ix, _)| ix)
}

/// Each pin's number, counting from one in the order the pins were placed.
pub(crate) fn pin_numbers(marks: &[Mark]) -> Vec<Option<usize>> {
    let mut count = 0;
    marks
        .iter()
        .map(|mark| {
            matches!(mark, Mark::Pin { .. }).then(|| {
                count += 1;
                count
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Mark, Tool, mark_at, pin_numbers};

    const SIZE: (f32, f32) = (400.0, 200.0);

    #[test]
    fn a_stroke_becomes_the_tools_mark() {
        let stroke = [(0.1, 0.1), (0.2, 0.3), (0.5, 0.4)];
        assert_eq!(
            Mark::made(Tool::Box, &stroke, 2),
            Some(Mark::Box {
                from: (0.1, 0.1),
                to: (0.5, 0.4),
                color: 2
            })
        );
        assert_eq!(
            Mark::made(Tool::Pin, &stroke, 0),
            Some(Mark::Pin {
                at: (0.5, 0.4),
                color: 0
            })
        );
        assert_eq!(Mark::made(Tool::Select, &stroke, 0), None);
        assert_eq!(Mark::made(Tool::Pen, &[], 0), None);
        let tap = [(0.4, 0.4)];
        assert_eq!(Mark::made(Tool::Box, &tap, 0), None, "a tap makes no box");
        assert_eq!(Mark::made(Tool::Arrow, &tap, 0), None);
        assert_eq!(Mark::made(Tool::Pen, &tap, 0), None, "nor a line");
        let back = [(0.4, 0.4), (0.6, 0.5), (0.4, 0.4)];
        assert_eq!(
            Mark::made(Tool::Box, &back, 0),
            None,
            "a drag back to its start makes no box"
        );
        assert!(
            Mark::made(Tool::Pen, &back, 0).is_some(),
            "but draws a line"
        );
        assert_eq!(
            Mark::made(Tool::Pin, &tap, 0),
            Some(Mark::Pin {
                at: (0.4, 0.4),
                color: 0
            }),
            "but a pin"
        );
    }

    #[test]
    fn a_press_finds_the_nearest_mark_by_its_edge_or_line_and_prefers_the_top() {
        let boxed = Mark::Box {
            from: (0.25, 0.25),
            to: (0.75, 0.75),
            color: 0,
        };
        let arrow = Mark::Arrow {
            from: (0.0, 0.5),
            to: (1.0, 0.5),
            color: 1,
        };
        let marks = [boxed.clone(), arrow];
        assert_eq!(
            boxed.distance((0.5, 0.5), SIZE),
            50.0,
            "inside a box is its edge's distance away"
        );
        assert_eq!(
            mark_at(&marks, (0.25, 0.4), SIZE, 6.0),
            Some(0),
            "on the box's left edge"
        );
        assert_eq!(
            mark_at(&marks, (0.1, 0.51), SIZE, 6.0),
            Some(1),
            "beside the arrow's line"
        );
        assert_eq!(mark_at(&marks, (0.1, 0.1), SIZE, 6.0), None);
        let twins = [
            Mark::Pin {
                at: (0.5, 0.5),
                color: 0,
            },
            Mark::Pin {
                at: (0.5, 0.5),
                color: 1,
            },
        ];
        assert_eq!(
            mark_at(&twins, (0.5, 0.5), SIZE, 6.0),
            Some(1),
            "the later one lies on top"
        );
    }

    #[test]
    fn pins_count_in_order_past_other_marks() {
        let pin = |x| Mark::Pin {
            at: (x, 0.5),
            color: 0,
        };
        let pen = Mark::Pen {
            points: vec![(0.0, 0.0)],
            color: 0,
        };
        assert_eq!(
            pin_numbers(&[pin(0.1), pen, pin(0.2)]),
            [Some(1), None, Some(2)]
        );
    }
}
