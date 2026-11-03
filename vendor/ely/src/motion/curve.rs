use std::time::Duration;

use gpui::{App, Pixels, px};

use crate::theme::ActiveTheme;

pub const FAST: Duration = Duration::from_millis(120);
pub const BASE: Duration = Duration::from_millis(200);
pub const SLOW: Duration = Duration::from_millis(320);
pub const THEME: Duration = Duration::from_millis(280);

/// Travel of small entrances.
pub const NUDGE: Pixels = px(4.0);

const DAMPING: f32 = 0.75;
const OMEGA: f32 = 6.91 / DAMPING;

/// `base`, or one millisecond under reduced motion.
pub fn duration(base: Duration, cx: &App) -> Duration {
    if cx.theme().reduced_motion {
        Duration::from_millis(1)
    } else {
        base
    }
}

pub fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

pub fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

pub fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// Damped spring. Overshoots; not a gpui easing.
pub fn spring(t: f32) -> f32 {
    if t >= 1.0 {
        return 1.0;
    }
    let damped = OMEGA * (1.0 - DAMPING * DAMPING).sqrt();
    let decay = (-DAMPING * OMEGA * t).exp();
    1.0 - decay * ((damped * t).cos() + DAMPING * OMEGA / damped * (damped * t).sin())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easings_pin_their_ends() {
        for ease in [ease_out_cubic, ease_in_out_cubic, spring] {
            assert!(ease(0.0).abs() < 1e-6);
            assert_eq!(ease(1.0), 1.0);
        }
    }

    #[test]
    fn spring_overshoots_a_little_then_settles() {
        let peak = (0..=100)
            .map(|i| spring(i as f32 / 100.0))
            .fold(0.0, f32::max);
        assert!(peak > 1.01 && peak < 1.05, "peak {peak}");
        assert!((spring(0.95) - 1.0).abs() < 0.01);
    }

    #[test]
    fn ease_in_out_is_symmetric() {
        for i in 0..=10 {
            let t = i as f32 / 10.0;
            assert!((ease_in_out_cubic(t) + ease_in_out_cubic(1.0 - t) - 1.0).abs() < 1e-5);
        }
    }
}
