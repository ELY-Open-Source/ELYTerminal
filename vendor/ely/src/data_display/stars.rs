use std::f32::consts::{FRAC_PI_2, PI};

use gpui::{
    App, Bounds, ContentMask, IntoElement, ParentElement, Path, PathBuilder, Pixels, RenderOnce,
    Styled, Window, canvas, div, point, size,
};

use crate::theme::{ActiveTheme, IconSize};

/// A five-point star inside `bounds`, filled or outlined with Lucide's stroke ratio.
pub(crate) fn star(bounds: Bounds<Pixels>, fill: bool) -> Path<Pixels> {
    let side = bounds.size.width.min(bounds.size.height);
    let stroke = side / 12.0;
    let center = bounds.center();
    let outer = side / 2.0 - stroke;
    let mut path = if fill {
        PathBuilder::fill()
    } else {
        PathBuilder::stroke(stroke)
    };
    for k in 0..10 {
        let radius = if k % 2 == 0 { outer } else { outer * 0.5 };
        let angle = -FRAC_PI_2 + k as f32 * PI / 5.0;
        let at = point(
            center.x + radius * angle.cos(),
            center.y + radius * angle.sin(),
        );
        if k == 0 {
            path.move_to(at);
        } else {
            path.line_to(at);
        }
    }
    path.close();
    path.build().expect("a star is a simple closed path")
}

/// How much of star `ix` a score fills, from none to all.
fn share(score: f32, ix: usize) -> f32 {
    (score - ix as f32).clamp(0.0, 1.0)
}

/// A score in stars, read only. A score of 4.6 fills four stars and three fifths of the fifth.
#[derive(IntoElement)]
pub struct Stars {
    score: f32,
    max: u8,
    size: IconSize,
}

impl Stars {
    pub fn new(score: f32) -> Self {
        assert!(
            score.is_finite() && score >= 0.0,
            "a score of {score} cannot be drawn"
        );
        Self {
            score,
            max: 5,
            size: IconSize::Md,
        }
    }

    pub fn max(mut self, max: u8) -> Self {
        assert!(max > 0, "stars need at least one");
        self.max = max;
        self
    }

    pub fn size(mut self, size: IconSize) -> Self {
        self.size = size;
        self
    }
}

impl RenderOnce for Stars {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (score, max) = (self.score, self.max);
        if score > max as f32 {
            log::error!("stars: a score of {score} is above {max}; all filled");
        }
        let theme = cx.theme();
        let (on, off) = (theme.colors.accent, theme.colors.border_strong);
        let side = theme.icon_size(self.size);
        div()
            .flex()
            .flex_none()
            .gap_0p5()
            .children((0..max as usize).map(|ix| {
                let filled = share(score, ix);
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        window.paint_path(star(bounds, false), off);
                        if filled > 0.0 {
                            let part = Bounds::new(
                                bounds.origin,
                                size(bounds.size.width * filled, bounds.size.height),
                            );
                            window.with_content_mask(
                                Some(ContentMask { bounds: part }),
                                |window| {
                                    window.paint_path(star(bounds, true), on);
                                    window.paint_path(star(bounds, false), on);
                                },
                            );
                        }
                    },
                )
                .size(side)
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::share;

    #[test]
    fn a_score_fills_whole_stars_then_part_of_one() {
        let fills: Vec<f32> = (0..5).map(|ix| share(3.5, ix)).collect();
        assert_eq!(fills, [1.0, 1.0, 1.0, 0.5, 0.0]);
        assert_eq!(share(0.0, 0), 0.0);
        assert_eq!(share(5.0, 4), 1.0);
    }
}
