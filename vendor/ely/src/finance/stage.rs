use std::ops::Range;

use crate::charts::nice;

/// The fewest candles a chart zooms in to.
const CLOSEST: f64 = 10.0;

/// The candles a chart shows: from a first index, which may sit between candles while panning, across `count` of them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Visible {
    pub(crate) start: f64,
    pub(crate) count: f64,
}

impl Visible {
    /// The newest `count` of `total` candles, a little room left past the last.
    pub(crate) fn latest(total: usize, count: f64) -> Self {
        let count = count.clamp(
            CLOSEST.min(total as f64).max(1.0),
            (total as f64).max(1.0) * 1.1,
        );
        Self {
            start: total as f64 - count * 0.95,
            count,
        }
        .kept(total)
    }

    /// Held to `total` candles after the data changed: no wider than they allow, and in reach of them.
    pub(crate) fn fitted(self, total: usize) -> Self {
        let count = self.count.clamp(
            CLOSEST.min(total as f64).max(1.0),
            (total as f64).max(1.0) * 1.1,
        );
        Self { count, ..self }.kept(total)
    }

    /// Kept to what `total` candles allow: never before the first, at most a quarter window past the last.
    fn kept(self, total: usize) -> Self {
        let last = (total as f64 - self.count * 0.75).max(0.0);
        Self {
            start: self.start.clamp(0.0, last),
            ..self
        }
    }

    /// Where candle `ix`'s middle sits across a frame starting at `left` and `width` wide.
    pub(crate) fn x(&self, ix: f64, (left, width): (f32, f32)) -> f32 {
        left + ((ix - self.start + 0.5) / self.count) as f32 * width
    }

    /// The candle nearest `x`, which may lie outside the data.
    pub(crate) fn at(&self, x: f32, (left, width): (f32, f32)) -> i64 {
        (self.start + f64::from((x - left) / width) * self.count - 0.5).round() as i64
    }

    /// How wide one candle's slot is.
    pub(crate) fn slot(&self, width: f32) -> f32 {
        width / self.count as f32
    }

    /// The candles in view, whole ones and those cut by an edge.
    pub(crate) fn range(&self, total: usize) -> Range<usize> {
        let first = self.start.floor().max(0.0) as usize;
        let last = ((self.start + self.count).ceil().max(0.0) as usize).min(total);
        first.min(last)..last
    }

    /// Closer in (`factor` under one) or farther out, holding still the candle at `share` of the way across.
    pub(crate) fn zoom(&self, factor: f64, share: f64, total: usize) -> Self {
        let count = (self.count * factor).clamp(
            CLOSEST.min(total as f64).max(1.0),
            (total as f64).max(1.0) * 1.1,
        );
        let held = self.start + self.count * share;
        Self {
            start: held - count * share,
            count,
        }
        .kept(total)
    }

    /// Moved by `candles`: later when positive.
    pub(crate) fn pan(&self, candles: f64, total: usize) -> Self {
        Self {
            start: self.start + candles,
            ..*self
        }
        .kept(total)
    }
}

/// The prices a pane shows: its low and high a little widened, and round ticks inside.
pub(crate) fn fit(low: f64, high: f64) -> ((f64, f64), Vec<f64>) {
    let room = ((high - low) * 0.08)
        .max(high.abs() * 1e-4)
        .max(f64::EPSILON);
    let (low, high) = (low - room, high + room);
    let (_, ticks) = nice(low, high, 5);
    let inside = ticks
        .into_iter()
        .filter(|tick| *tick >= low && *tick <= high)
        .collect();
    ((low, high), inside)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_fits_data_that_shrank_under_it() {
        let window = Visible {
            start: 150.0,
            count: 50.0,
        };
        let fitted = window.fitted(12);
        assert!(fitted.count <= 12.0 * 1.1);
        let range = fitted.range(12);
        assert!(
            !range.is_empty() && range.end == 12,
            "{range:?} shows the newest candles"
        );
    }

    #[test]
    fn a_window_maps_candles_across_its_frame() {
        let window = Visible {
            start: 10.0,
            count: 20.0,
        };
        assert_eq!(
            window.x(10.0, (0.0, 200.0)),
            5.0,
            "the first candle's middle"
        );
        assert_eq!(window.at(5.0, (0.0, 200.0)), 10);
        assert_eq!(window.at(199.0, (0.0, 200.0)), 29);
        assert_eq!(window.slot(200.0), 10.0);
        assert_eq!(window.range(100), 10..30);
        assert_eq!(
            Visible {
                start: 95.5,
                count: 20.0
            }
            .range(100),
            95..100,
            "the edge of the data"
        );
    }

    #[test]
    fn zoom_holds_the_candle_under_the_pointer_and_pan_stops_at_the_ends() {
        let window = Visible {
            start: 0.0,
            count: 40.0,
        };
        let closer = window.zoom(0.5, 0.5, 100);
        assert_eq!(
            (closer.start, closer.count),
            (10.0, 20.0),
            "the middle candle stays in the middle"
        );
        assert_eq!(
            window.zoom(0.01, 0.5, 100).count,
            10.0,
            "no closer than ten candles"
        );
        assert_eq!(window.pan(-5.0, 100).start, 0.0, "not before the first");
        assert_eq!(
            window.pan(500.0, 100).start,
            70.0,
            "a quarter window past the last at most"
        );
        let latest = Visible::latest(100, 40.0);
        assert_eq!(latest.range(100).end, 100, "the newest candle shows");
    }

    #[test]
    fn prices_fit_with_room_and_read_to_their_step() {
        let ((low, high), ticks) = fit(100.0, 110.0);
        assert!(low < 100.0 && high > 110.0);
        assert!(ticks.iter().all(|tick| (low..=high).contains(tick)) && ticks.len() >= 3);
    }
}
