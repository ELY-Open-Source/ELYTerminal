/// A crop of a picture: its box in shares of the picture's width and height, each from 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crop {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Crop {
    /// The whole picture.
    pub const WHOLE: Self = Self {
        x: 0.0,
        y: 0.0,
        w: 1.0,
        h: 1.0,
    };

    /// The largest box of `aspect`, its width over height in pixels, inside this one and centered on it, on a picture of `ratio`.
    pub fn fitted(self, aspect: f32, ratio: f32) -> Self {
        fitted(self, aspect / ratio)
    }

    /// Fails loud on a box with no size or one that leaves its picture.
    pub(crate) fn checked(self) -> Self {
        let inside =
            |start: f32, side: f32| start >= 0.0 && side > 0.0 && start + side <= 1.0 + 1e-4;
        assert!(
            inside(self.x, self.w) && inside(self.y, self.h),
            "a crop past its picture: {self:?}"
        );
        self
    }
}

/// The sides a drag moves: none moves the whole box, one an edge, two a corner.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct Grip {
    pub left: bool,
    pub right: bool,
    pub top: bool,
    pub bottom: bool,
}

impl Grip {
    pub(crate) fn corner(self) -> bool {
        (self.left || self.right) && (self.top || self.bottom)
    }
}

/// What a press at `at`, in shares of the picture, holds: a side or corner within `reach` shares across and down, else the body when inside, else nothing. A shaped crop sizes by corners only, so its edges hold the body.
pub(crate) fn grip_at(crop: Crop, at: (f32, f32), reach: (f32, f32), shaped: bool) -> Option<Grip> {
    let (x2, y2) = (crop.x + crop.w, crop.y + crop.h);
    let near = |a: f32, b: f32, r: f32| (a - b).abs() <= r;
    let around = at.0 >= crop.x - reach.0
        && at.0 <= x2 + reach.0
        && at.1 >= crop.y - reach.1
        && at.1 <= y2 + reach.1;
    let inside = at.0 >= crop.x && at.0 <= x2 && at.1 >= crop.y && at.1 <= y2;
    let (left, top) = (near(at.0, crop.x, reach.0), near(at.1, crop.y, reach.1));
    let grip = Grip {
        left,
        right: !left && near(at.0, x2, reach.0),
        top,
        bottom: !top && near(at.1, y2, reach.1),
    };
    let sizes = grip != Grip::default() && (grip.corner() || !shaped);
    match (around, sizes, inside) {
        (true, true, _) => Some(grip),
        (_, _, true) => Some(Grip::default()),
        _ => None,
    }
}

/// The crop after `grip` moves by `by`, in shares, kept inside the picture with sides of at least `least`. With `shape`, a width over height in shares, only corners resize and the box keeps that shape, anchored at the opposite corner, sized by whichever side the pointer moved further.
pub(crate) fn dragged(
    crop: Crop,
    grip: Grip,
    by: (f32, f32),
    least: f32,
    shape: Option<f32>,
) -> Crop {
    let (mut x1, mut y1, mut x2, mut y2) = (crop.x, crop.y, crop.x + crop.w, crop.y + crop.h);
    if grip == Grip::default() {
        let dx = by.0.clamp(-x1, 1.0 - x2);
        let dy = by.1.clamp(-y1, 1.0 - y2);
        return Crop {
            x: x1 + dx,
            y: y1 + dy,
            ..crop
        };
    }
    if shape.is_some() && !grip.corner() {
        return crop;
    }
    if grip.left {
        x1 = (x1 + by.0).clamp(0.0, x2 - least);
    }
    if grip.right {
        x2 = (x2 + by.0).clamp(x1 + least, 1.0);
    }
    if grip.top {
        y1 = (y1 + by.1).clamp(0.0, y2 - least);
    }
    if grip.bottom {
        y2 = (y2 + by.1).clamp(y1 + least, 1.0);
    }
    if let Some(shape) = shape {
        let tall = if grip.top { y2 } else { 1.0 - y1 };
        let wide = if grip.left { x2 } else { 1.0 - x1 };
        let (across, down) = (x2 - x1, (y2 - y1) * shape);
        let asked = if (across - crop.w).abs() >= (down - crop.w).abs() {
            across
        } else {
            down
        };
        let w = asked
            .max(least * shape.max(1.0))
            .min(tall * shape)
            .min(wide);
        let h = w / shape;
        if grip.left {
            x1 = x2 - w;
        } else {
            x2 = x1 + w;
        }
        if grip.top {
            y1 = y2 - h;
        } else {
            y2 = y1 + h;
        }
    }
    Crop {
        x: x1,
        y: y1,
        w: x2 - x1,
        h: y2 - y1,
    }
}

/// The largest box of `shape`, a width over height in shares, inside `crop` and centered on it.
pub(crate) fn fitted(crop: Crop, shape: f32) -> Crop {
    let (w, h) = if crop.w / crop.h > shape {
        (crop.h * shape, crop.h)
    } else {
        (crop.w, crop.w / shape)
    };
    Crop {
        x: crop.x + (crop.w - w) / 2.0,
        y: crop.y + (crop.h - h) / 2.0,
        w,
        h,
    }
}

#[cfg(test)]
mod tests {
    use super::{Crop, Grip, dragged, fitted, grip_at};

    const HALF: Crop = Crop {
        x: 0.25,
        y: 0.25,
        w: 0.5,
        h: 0.5,
    };

    fn near(a: Crop, b: Crop) -> bool {
        [(a.x, b.x), (a.y, b.y), (a.w, b.w), (a.h, b.h)]
            .iter()
            .all(|(a, b)| (a - b).abs() < 1e-5)
    }

    #[test]
    fn the_body_moves_whole_and_stops_at_the_picture_edge() {
        let moved = dragged(HALF, Grip::default(), (0.1, -0.4), 0.05, None);
        assert!(
            near(
                moved,
                Crop {
                    x: 0.35,
                    y: 0.0,
                    ..HALF
                }
            ),
            "{moved:?}"
        );
    }

    #[test]
    fn an_edge_or_corner_moves_its_sides_and_keeps_the_least_size() {
        let right = Grip {
            right: true,
            ..Grip::default()
        };
        assert!(near(
            dragged(HALF, right, (0.1, 0.3), 0.05, None),
            Crop { w: 0.6, ..HALF }
        ));
        assert!(near(
            dragged(HALF, right, (-0.9, 0.0), 0.05, None),
            Crop { w: 0.05, ..HALF }
        ));
        let top_left = Grip {
            left: true,
            top: true,
            ..Grip::default()
        };
        let grown = dragged(HALF, top_left, (-0.5, -0.1), 0.05, None);
        assert!(
            near(
                grown,
                Crop {
                    x: 0.0,
                    y: 0.15,
                    w: 0.75,
                    h: 0.6
                }
            ),
            "{grown:?}"
        );
    }

    #[test]
    fn a_shaped_crop_resizes_by_corners_only_and_keeps_its_shape_inside_the_picture() {
        let bottom_right = Grip {
            right: true,
            bottom: true,
            ..Grip::default()
        };
        let square = dragged(HALF, bottom_right, (0.1, 0.0), 0.05, Some(1.0));
        assert!(
            near(
                square,
                Crop {
                    w: 0.6,
                    h: 0.6,
                    ..HALF
                }
            ),
            "{square:?}"
        );
        let stopped = dragged(HALF, bottom_right, (0.25, 0.0), 0.05, Some(2.0));
        assert!(
            near(
                stopped,
                Crop {
                    w: 0.75,
                    h: 0.375,
                    ..HALF
                }
            ),
            "{stopped:?}"
        );
        let edge = Grip {
            top: true,
            ..Grip::default()
        };
        assert_eq!(dragged(HALF, edge, (0.0, -0.1), 0.05, Some(1.0)), HALF);
    }

    #[test]
    fn a_shaped_corner_follows_the_pointer_up_and_down_too() {
        let bottom_right = Grip {
            right: true,
            bottom: true,
            ..Grip::default()
        };
        let down = dragged(HALF, bottom_right, (0.0, 0.2), 0.05, Some(1.0));
        assert!(
            near(
                down,
                Crop {
                    w: 0.7,
                    h: 0.7,
                    ..HALF
                }
            ),
            "straight down grows it: {down:?}"
        );
        let up = dragged(HALF, bottom_right, (0.0, -0.2), 0.05, Some(1.0));
        assert!(
            near(
                up,
                Crop {
                    w: 0.3,
                    h: 0.3,
                    ..HALF
                }
            ),
            "straight up shrinks it: {up:?}"
        );
    }

    #[test]
    fn a_new_shape_takes_the_largest_centered_box_inside() {
        let wide = fitted(HALF, 2.0);
        assert!(
            near(
                wide,
                Crop {
                    x: 0.25,
                    y: 0.375,
                    w: 0.5,
                    h: 0.25
                }
            ),
            "{wide:?}"
        );
        let tall = fitted(Crop::WHOLE, 0.5);
        assert!(
            near(
                tall,
                Crop {
                    x: 0.25,
                    y: 0.0,
                    w: 0.5,
                    h: 1.0
                }
            ),
            "{tall:?}"
        );
    }

    #[test]
    fn a_press_holds_the_nearest_corner_or_side_else_the_body_else_nothing() {
        let reach = (0.02, 0.02);
        let corner = grip_at(HALF, (0.26, 0.74), reach, false);
        assert_eq!(
            corner,
            Some(Grip {
                left: true,
                bottom: true,
                ..Grip::default()
            })
        );
        let side = grip_at(HALF, (0.5, 0.24), reach, false);
        assert_eq!(
            side,
            Some(Grip {
                top: true,
                ..Grip::default()
            })
        );
        assert_eq!(
            grip_at(HALF, (0.5, 0.5), reach, false),
            Some(Grip::default())
        );
        assert_eq!(grip_at(HALF, (0.1, 0.5), reach, false), None);
        assert_eq!(
            grip_at(HALF, (0.5, 0.26), reach, true),
            Some(Grip::default()),
            "a shaped crop's edge holds its body"
        );
        assert_eq!(
            grip_at(HALF, (0.5, 0.24), reach, true),
            None,
            "and just outside it holds nothing"
        );
    }

    #[test]
    #[should_panic(expected = "a crop past its picture")]
    fn a_crop_past_its_picture_fails_loud() {
        Crop {
            x: 0.8,
            y: 0.0,
            w: 0.5,
            h: 0.5,
        }
        .checked();
    }
}
