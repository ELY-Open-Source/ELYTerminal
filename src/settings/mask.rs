//! A picture's mask, as CSS's `mask-image` draws it: an alpha gradient over the picture's box.

use anyhow::{Context as _, ensure};
use gpui::Hsla;

use super::color;

/// The fewest stops a mask holds.
pub const MIN_STOPS: usize = 2;
/// The most stops a mask holds.
pub const MAX_STOPS: usize = 16;

/// A mask's gradient: the way it runs and each stop's alpha, the stops even steps across the box.
#[derive(Clone, Debug, PartialEq)]
pub struct Mask {
    /// How far a step across the box, and one down it, moves along the gradient.
    across: f32,
    down: f32,
    stops: Vec<f32>,
}

impl Mask {
    /// Reads `"transparent, black"`, two to sixteen colors, comma separated, for a box of `size`.
    pub fn new(angle: f32, stops: &str, (width, height): (f32, f32)) -> anyhow::Result<Self> {
        let alpha = |stop: &str| -> anyhow::Result<f32> {
            let color: Hsla = color(stop.trim()).with_context(|| format!("mask stop {stop:?}"))?;
            Ok(color.a)
        };
        // A comma inside `rgba(…)` parts no stops.
        let mut depth = 0;
        let apart = |ch: char| {
            depth += i32::from(ch == '(') - i32::from(ch == ')');
            ch == ',' && depth == 0
        };
        let stops = stops
            .split(apart)
            .map(alpha)
            .collect::<anyhow::Result<Vec<f32>>>()?;
        ensure!(
            (MIN_STOPS..=MAX_STOPS).contains(&stops.len()),
            "a mask holds {MIN_STOPS} to {MAX_STOPS} stops, not {}",
            stops.len()
        );
        // CSS's gradient line: through the center, as long as the box's corners reach along it.
        let (sin, cos) = angle.to_radians().sin_cos();
        let length = (width * sin).abs() + (height * cos).abs();
        Ok(Self {
            across: width * sin / length,
            down: -height * cos / length,
            stops,
        })
    }

    /// Its alpha at `u` and `v`, a point of the box from 0 at its top left to 1 at its bottom right.
    pub fn alpha(&self, u: f32, v: f32) -> f32 {
        let t = 0.5 + (u - 0.5) * self.across + (v - 0.5) * self.down;
        let at = t.clamp(0.0, 1.0) * (self.stops.len() - 1) as f32;
        let low = at.floor() as usize;
        let high = (low + 1).min(self.stops.len() - 1);
        let mix = at - low as f32;
        let (from, to) = (self.stops[low], self.stops[high]);
        from + (to - from) * mix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_stop_is_a_color() {
        let mask = Mask::new(90.0, "transparent, black", (1.0, 1.0)).expect("two stops");
        assert_eq!(mask.stops, [0.0, 1.0]);
        assert!(
            Mask::new(90.0, "black", (1.0, 1.0)).is_err(),
            "one stop is none"
        );
        assert!(
            Mask::new(90.0, "#000, #111, #222, #333, #444, #555, #666, #777, #888, #999, #aaa, #bbb, #ccc, #ddd, #eee, #fff, #000", (1.0, 1.0)).is_err(),
            "seventeen stops are too many"
        );
        assert!(
            Mask::new(90.0, "transparent, nope", (1.0, 1.0)).is_err(),
            "a stop reads"
        );
    }

    #[test]
    fn the_angle_fades_across_the_box() {
        let mask = Mask::new(90.0, "transparent, black", (1.0, 1.0)).expect("two stops");
        assert_eq!(mask.alpha(0.0, 0.5), 0.0, "the left is the first stop");
        assert_eq!(mask.alpha(1.0, 0.5), 1.0, "the right is the last");
        assert!((mask.alpha(0.5, 0.5) - 0.5).abs() < 1e-6);
        let up = Mask::new(0.0, "transparent, black", (1.0, 1.0)).expect("two stops");
        assert_eq!(
            up.alpha(0.5, 1.0),
            0.0,
            "0 points up, so the bottom is first"
        );
        assert_eq!(up.alpha(0.5, 0.0), 1.0);
        assert_eq!(up.alpha(0.0, 0.0), 1.0, "across the angle nothing changes");
    }

    #[test]
    fn a_slanted_mask_runs_as_css_s_gradient_line_does() {
        let mask = Mask::new(45.0, "transparent, black", (200.0, 100.0)).expect("two stops");
        assert!((mask.alpha(0.75, 0.5) - 2.0 / 3.0).abs() < 1e-6);
        assert!(mask.alpha(0.0, 1.0).abs() < 1e-6, "the far corner is first");
        assert!(
            (mask.alpha(1.0, 0.0) - 1.0).abs() < 1e-6,
            "the near one last"
        );
    }

    #[test]
    fn a_color_s_own_commas_part_no_stops() {
        let stops = "rgba(0, 0, 0, 0), rgba(0, 0, 0, 1), #000";
        let mask = Mask::new(90.0, stops, (1.0, 1.0)).expect("three stops");
        assert_eq!(mask.stops, [0.0, 1.0, 1.0]);
    }

    #[test]
    fn the_stops_share_the_way_evenly() {
        let mask = Mask::new(90.0, "transparent, black, black", (1.0, 1.0)).expect("three stops");
        assert!(
            (mask.alpha(0.25, 0.5) - 0.5).abs() < 1e-6,
            "halfway to the second"
        );
        assert!(
            (mask.alpha(0.5, 0.5) - 1.0).abs() < 1e-6,
            "the second is black"
        );
        assert!(
            (mask.alpha(0.75, 0.5) - 1.0).abs() < 1e-6,
            "the last is black"
        );
    }
}
