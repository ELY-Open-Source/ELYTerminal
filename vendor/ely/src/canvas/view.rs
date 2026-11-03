/// A box on the canvas, in canvas units.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Frame {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Frame {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        assert!(w >= 0.0 && h >= 0.0, "a frame of {w} by {h}");
        Self { x, y, w, h }
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub fn center(&self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    /// The least frame that holds both.
    pub fn union(&self, other: &Frame) -> Frame {
        let (x, y) = (self.x.min(other.x), self.y.min(other.y));
        Frame::new(
            x,
            y,
            self.right().max(other.right()) - x,
            self.bottom().max(other.bottom()) - y,
        )
    }

    /// The frame spanned by two corners, in any order.
    pub fn spanning((ax, ay): (f32, f32), (bx, by): (f32, f32)) -> Frame {
        Frame::new(ax.min(bx), ay.min(by), (ax - bx).abs(), (ay - by).abs())
    }

    /// Whether it lies wholly inside `other`.
    pub fn within(&self, other: &Frame) -> bool {
        self.x >= other.x
            && self.y >= other.y
            && self.right() <= other.right()
            && self.bottom() <= other.bottom()
    }

    pub fn contains(&self, (x, y): (f32, f32)) -> bool {
        x >= self.x && x <= self.right() && y >= self.y && y <= self.bottom()
    }
}

/// Where a canvas looks: the canvas point at the view's top left, and its zoom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}

impl Viewport {
    /// The farthest out and the nearest in a canvas zooms.
    pub const ZOOMS: (f32, f32) = (0.1, 8.0);

    pub fn new(x: f32, y: f32, zoom: f32) -> Self {
        let (least, most) = Self::ZOOMS;
        assert!(
            (least..=most).contains(&zoom),
            "zoom {zoom} past {least} to {most}"
        );
        Self { x, y, zoom }
    }

    /// A canvas point in view pixels from the view's top left.
    pub fn to_view(&self, (x, y): (f32, f32)) -> (f32, f32) {
        ((x - self.x) * self.zoom, (y - self.y) * self.zoom)
    }

    /// A point in the view as a canvas point.
    pub fn to_canvas(&self, (x, y): (f32, f32)) -> (f32, f32) {
        (x / self.zoom + self.x, y / self.zoom + self.y)
    }

    /// Zoomed by `factor` about a view point, which keeps its canvas point; held inside the zooms.
    pub fn zoomed(self, factor: f32, about: (f32, f32)) -> Viewport {
        let (least, most) = Self::ZOOMS;
        let zoom = (self.zoom * factor).clamp(least, most);
        let (x, y) = self.to_canvas(about);
        Viewport::new(x - about.0 / zoom, y - about.1 / zoom, zoom)
    }

    /// Moved so the content slides by a view distance.
    pub fn panned(self, (dx, dy): (f32, f32)) -> Viewport {
        Viewport::new(self.x - dx / self.zoom, self.y - dy / self.zoom, self.zoom)
    }

    /// Shows `frame` whole and centered in a view of `size`, `margin` view pixels clear each side, no nearer than a zoom of one.
    pub fn fitting(frame: Frame, (w, h): (f32, f32), margin: f32) -> Viewport {
        let (least, _) = Self::ZOOMS;
        let room = ((w - margin * 2.0).max(1.0), (h - margin * 2.0).max(1.0));
        let zoom = (room.0 / frame.w.max(1.0))
            .min(room.1 / frame.h.max(1.0))
            .clamp(least, 1.0);
        let (cx, cy) = frame.center();
        Viewport::new(cx - w / 2.0 / zoom, cy - h / 2.0 / zoom, zoom)
    }
}

/// The canvas step between marks at `zoom`: the least of one, two or five times a power of ten that sets marks `least` view pixels apart or more.
pub(crate) fn step(zoom: f32, least: f32) -> f32 {
    let raw = least / zoom;
    let power = 10f32.powf(raw.log10().floor());
    [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|each| each * power)
        .find(|each| *each >= raw)
        .expect("ten times the power passes the raw step")
}

/// Canvas marks every `step` from `from` through `to`.
pub(crate) fn marks(from: f32, to: f32, step: f32) -> Vec<f32> {
    let first = (from / step).ceil() as i64;
    let last = (to / step).floor() as i64;
    (first..=last).map(|ix| ix as f32 * step).collect()
}

#[cfg(test)]
mod tests {
    use super::{Frame, Viewport, marks, step};

    #[test]
    fn a_point_keeps_its_place_as_the_view_zooms_about_it() {
        let view = Viewport::new(100.0, 50.0, 1.0);
        let about = (200.0, 120.0);
        let held = view.to_canvas(about);
        let near = view.zoomed(2.0, about);
        assert_eq!(near.zoom, 2.0);
        assert_eq!(near.to_canvas(about), held);
        assert_eq!(
            view.zoomed(1000.0, about).zoom,
            8.0,
            "held at the nearest zoom"
        );
    }

    #[test]
    fn a_pan_slides_the_content_with_the_pointer() {
        let view = Viewport::new(0.0, 0.0, 2.0).panned((40.0, -20.0));
        assert_eq!((view.x, view.y), (-20.0, 10.0));
        assert_eq!(view.to_view((0.0, 0.0)), (40.0, -20.0));
    }

    #[test]
    fn a_fit_shows_the_frame_whole_and_centered() {
        let view = Viewport::fitting(Frame::new(0.0, 0.0, 800.0, 400.0), (440.0, 440.0), 20.0);
        assert_eq!(view.zoom, 0.5);
        assert_eq!(view.to_view((400.0, 200.0)), (220.0, 220.0));
        let small = Viewport::fitting(Frame::new(0.0, 0.0, 100.0, 50.0), (440.0, 440.0), 20.0);
        assert_eq!(small.zoom, 1.0, "no nearer than a zoom of one");
        assert_eq!(small.to_view((50.0, 25.0)), (220.0, 220.0));
    }

    #[test]
    fn marks_step_by_one_two_or_five_and_stay_apart() {
        assert_eq!(step(1.0, 16.0), 20.0);
        assert_eq!(step(0.5, 16.0), 50.0);
        assert_eq!(step(4.0, 16.0), 5.0);
        assert_eq!(marks(-15.0, 45.0, 20.0), [0.0, 20.0, 40.0]);
    }
}
