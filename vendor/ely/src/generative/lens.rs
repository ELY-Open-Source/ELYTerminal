use std::{
    collections::HashSet,
    f32::consts::{PI, TAU},
};

use super::embed::Embedded;

/// How far the eye sits from the middle, in the points' own units.
const EYE: f32 = 4.0;
/// Where a turned view starts.
pub(crate) const START: (f32, f32) = (0.6, 0.35);
/// What a key turns the view.
pub(crate) const STEP: f32 = PI / 12.0;
/// What a drag turns the view for each pixel.
pub(crate) const DRAG: f32 = 0.01;
/// Farthest the view tips up or down, short of looking straight along the upright axis.
const TIP: f32 = 1.4;

/// How the view shows its points: flat or turned, in a box this many pixels across and down, fitting points out to `radius` from the middle.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Lens {
    pub turn: Option<(f32, f32)>,
    pub size: (f32, f32),
    pub radius: f32,
}

impl Lens {
    /// Where a point lands, in pixels from the box's top left, and how near it sits, 0 far to 1 near. Flat, x runs across and y up, and z is dropped. Turned by yaw about the upright axis, then by pitch, the view scales by the box's shorter side, nearer points spread wider, and a point out to `radius` stays inside at any turn.
    pub(crate) fn place(&self, [x, y, z]: [f32; 3]) -> (f32, f32, f32) {
        let (w, h) = self.size;
        let Some((yaw, pitch)) = self.turn else {
            return (w * (0.5 + x * 0.45), h * (0.5 - y * 0.45), 0.5);
        };
        let (x1, z1) = (x * yaw.cos() + z * yaw.sin(), z * yaw.cos() - x * yaw.sin());
        let (y2, z2) = (
            y * pitch.cos() - z1 * pitch.sin(),
            y * pitch.sin() + z1 * pitch.cos(),
        );
        let widest = self.radius * EYE / (EYE * EYE - self.radius * self.radius).sqrt();
        let scale = w.min(h) * 0.45 / widest * EYE / (EYE - z2);
        (
            w / 2.0 + x1 * scale,
            h / 2.0 - y2 * scale,
            (z2 / self.radius + 1.0) / 2.0,
        )
    }

    /// The point nearest `at` within `reach`, both in pixels; hidden groups are skipped.
    pub(crate) fn nearest(
        &self,
        points: &[Embedded],
        hidden: &HashSet<usize>,
        at: (f32, f32),
        reach: f32,
    ) -> Option<usize> {
        points
            .iter()
            .enumerate()
            .filter(|(_, point)| !hidden.contains(&point.group))
            .map(|(ix, point)| {
                let (x, y, _) = self.place(point.at);
                (ix, (x - at.0).hypot(y - at.1))
            })
            .filter(|(_, far)| *far <= reach)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(ix, _)| ix)
    }
}

/// How far the farthest point sits from the middle, and never less than 1: a cloud inside the unit ball keeps the ball's scale.
pub(crate) fn radius(points: &[Embedded]) -> f32 {
    points
        .iter()
        .map(|point| point.at.iter().map(|axis| axis * axis).sum::<f32>().sqrt())
        .fold(1.0, f32::max)
}

/// A turn moved by `yaw` and `pitch`; the pitch stops short of either pole.
pub(crate) fn turned((yaw, pitch): (f32, f32), by: (f32, f32)) -> (f32, f32) {
    (
        (yaw + by.0).rem_euclid(TAU),
        (pitch + by.1).clamp(-TIP, TIP),
    )
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashSet,
        f32::consts::{FRAC_PI_2, TAU},
    };

    use super::{Lens, radius, turned};
    use crate::generative::Embedded;

    const BOX: (f32, f32) = (400.0, 300.0);

    fn lens(turn: Option<(f32, f32)>, radius: f32) -> Lens {
        Lens {
            turn,
            size: BOX,
            radius,
        }
    }

    #[test]
    fn flat_points_fill_the_box_and_a_quarter_turn_swings_one_back() {
        let (x, y, near) = lens(None, 1.0).place([1.0, 1.0, 0.5]);
        assert!(
            (x - 380.0).abs() < 1e-3 && (y - 15.0).abs() < 1e-3 && near == 0.5,
            "{x}, {y}, {near}"
        );
        let (x, y, near) = lens(Some((0.0, 0.0)), 1.0).place([1.0, 0.0, 0.0]);
        let ball = 300.0 * 0.45 * (15.0f32).sqrt() / 4.0;
        assert!(
            (x - 200.0 - ball).abs() < 1e-3
                && (y - 150.0).abs() < 1e-3
                && (near - 0.5).abs() < 1e-6,
            "the unit ball keeps its scale: {x}, {y}, {near}"
        );
        let (x, _, near) = lens(Some((FRAC_PI_2, 0.0)), 1.0).place([1.0, 0.0, 0.0]);
        assert!(
            (x - 200.0).abs() < 1e-3 && near < 0.5,
            "right swings to the back: {x}, {near}"
        );
    }

    #[test]
    fn every_corner_stays_inside_at_any_turn_and_one_reaches_the_edge() {
        let corners: Vec<Embedded> = (0..8)
            .map(|corner: usize| Embedded {
                key: corner.to_string().into(),
                label: corner.to_string().into(),
                group: 0,
                at: [0, 1, 2].map(|axis| if corner >> axis & 1 == 1 { 1.0 } else { -1.0 }),
            })
            .collect();
        let fit = radius(&corners);
        assert!((fit - 3f32.sqrt()).abs() < 1e-6, "{fit}");
        let mut widest: f32 = 0.0;
        for yaw in 0..36 {
            for pitch in -14..=14 {
                let turn = (yaw as f32 * TAU / 36.0, pitch as f32 / 10.0);
                for corner in &corners {
                    let (x, y, near) = lens(Some(turn), fit).place(corner.at);
                    widest = widest
                        .max((x - BOX.0 / 2.0).abs())
                        .max((y - BOX.1 / 2.0).abs());
                    assert!(
                        (0.0..=1.0).contains(&near),
                        "{:?} at {turn:?}: near {near}",
                        corner.at
                    );
                }
            }
        }
        assert!(
            widest <= BOX.1 * 0.45 + 1e-3 && widest > BOX.1 * 0.43,
            "{widest}"
        );
    }

    #[test]
    fn the_nearest_point_within_reach_and_not_hidden() {
        let point = |key: &str, group, x| Embedded {
            key: key.to_string().into(),
            label: key.to_string().into(),
            group,
            at: [x, 0.0, 0.0],
        };
        let points = [point("a", 0, 0.0), point("b", 1, 0.5)];
        let (flat, mut hidden) = (lens(None, 1.0), HashSet::new());
        assert_eq!(flat.nearest(&points, &hidden, (285.0, 150.0), 8.0), Some(1));
        assert_eq!(flat.nearest(&points, &hidden, (300.0, 150.0), 8.0), None);
        hidden.insert(1);
        assert_eq!(flat.nearest(&points, &hidden, (285.0, 150.0), 8.0), None);
        assert_eq!(
            radius(&points),
            1.0,
            "a cloud inside the ball fits the ball"
        );
    }

    #[test]
    fn a_turn_wraps_around_and_stops_short_of_the_poles() {
        let (yaw, pitch) = turned((6.0, 1.3), (0.5, 0.5));
        assert!(
            (yaw - (6.5 - TAU)).abs() < 1e-5 && pitch == 1.4,
            "{yaw}, {pitch}"
        );
        assert_eq!(turned((0.0, -1.3), (0.0, -0.5)).1, -1.4);
    }
}
