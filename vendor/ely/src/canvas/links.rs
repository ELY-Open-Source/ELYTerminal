use gpui::SharedString;

use super::view::Frame;

/// A line from one shape to another that follows them as they move: straight or turning at right angles, with an arrow at its end and words at its middle.
#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    pub key: SharedString,
    pub from: SharedString,
    pub to: SharedString,
    pub label: Option<SharedString>,
    pub elbow: bool,
}

impl Link {
    pub fn new(
        key: impl Into<SharedString>,
        from: impl Into<SharedString>,
        to: impl Into<SharedString>,
    ) -> Self {
        let (from, to) = (from.into(), to.into());
        assert!(from != to, "a link from {from} to itself");
        Self {
            key: key.into(),
            from,
            to,
            label: None,
            elbow: false,
        }
    }

    pub fn label(mut self, text: impl Into<SharedString>) -> Self {
        self.label = Some(text.into());
        self
    }

    /// Turns at right angles instead of running straight.
    pub fn elbow(mut self) -> Self {
        self.elbow = true;
        self
    }
}

/// The points a link runs through, in canvas units: out of the side of `from` that faces `to`, into the side of `to` that faces back. An elbow turns halfway between.
pub(crate) fn route(from: &Frame, to: &Frame, elbow: bool) -> Vec<(f32, f32)> {
    let (a, b) = (from.center(), to.center());
    let across = (b.0 - a.0).abs() >= (b.1 - a.1).abs();
    let (exit, entry) = match (across, b.0 >= a.0, b.1 >= a.1) {
        (true, true, _) => ((from.right(), a.1), (to.x, b.1)),
        (true, false, _) => ((from.x, a.1), (to.right(), b.1)),
        (false, _, true) => ((a.0, from.bottom()), (b.0, to.y)),
        (false, _, false) => ((a.0, from.y), (b.0, to.bottom())),
    };
    match (elbow, across) {
        (false, _) => vec![exit, entry],
        (true, true) => {
            let middle = (exit.0 + entry.0) / 2.0;
            vec![exit, (middle, exit.1), (middle, entry.1), entry]
        }
        (true, false) => {
            let middle = (exit.1 + entry.1) / 2.0;
            vec![exit, (exit.0, middle), (entry.0, middle), entry]
        }
    }
}

/// The point halfway along a run of points.
pub(crate) fn halfway(points: &[(f32, f32)]) -> (f32, f32) {
    let length =
        |(a, b): (&(f32, f32), &(f32, f32))| ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
    let whole: f32 = points.iter().zip(&points[1..]).map(length).sum();
    let mut left = whole / 2.0;
    for (a, b) in points.iter().zip(&points[1..]) {
        let span = length((a, b));
        if left <= span && span > 0.0 {
            let share = left / span;
            return (a.0 + (b.0 - a.0) * share, a.1 + (b.1 - a.1) * share);
        }
        left -= span;
    }
    *points.last().expect("a run has points")
}

#[cfg(test)]
mod tests {
    use super::{halfway, route};
    use crate::canvas::Frame;

    #[test]
    fn a_link_leaves_the_side_that_faces_the_other_shape() {
        let (a, b) = (
            Frame::new(0.0, 0.0, 100.0, 50.0),
            Frame::new(200.0, 100.0, 100.0, 50.0),
        );
        assert_eq!(route(&a, &b, false), [(100.0, 25.0), (200.0, 125.0)]);
        assert_eq!(route(&b, &a, false), [(200.0, 125.0), (100.0, 25.0)]);
        let below = Frame::new(0.0, 300.0, 100.0, 50.0);
        assert_eq!(route(&a, &below, false), [(50.0, 50.0), (50.0, 300.0)]);
    }

    #[test]
    fn an_elbow_turns_halfway() {
        let (a, b) = (
            Frame::new(0.0, 0.0, 100.0, 50.0),
            Frame::new(200.0, 100.0, 100.0, 50.0),
        );
        assert_eq!(
            route(&a, &b, true),
            [(100.0, 25.0), (150.0, 25.0), (150.0, 125.0), (200.0, 125.0)]
        );
        assert_eq!(halfway(&route(&a, &b, true)), (150.0, 75.0));
    }
}
