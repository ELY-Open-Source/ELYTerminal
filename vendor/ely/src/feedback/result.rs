use std::f32::consts::{FRAC_PI_2, TAU};

use gpui::{
    Animation, AnimationExt, App, Bounds, ElementId, Hsla, IntoElement, PathBuilder, Pixels, Point,
    RenderOnce, SharedString, Styled, Window, canvas, div, point, prelude::*,
};

use super::states::{Words, state_view, words_builders};
use crate::{
    motion,
    primitives::Severity,
    theme::{ActiveTheme, IconSize},
};

/// Where the mark's strokes run, in a unit square: a check, or the two bars of an X.
const CHECK: [[(f32, f32); 3]; 1] = [[(0.31, 0.52), (0.45, 0.65), (0.70, 0.38)]];
const CROSS: [[(f32, f32); 3]; 2] = [
    [(0.36, 0.36), (0.50, 0.50), (0.64, 0.64)],
    [(0.64, 0.36), (0.50, 0.50), (0.36, 0.64)],
];
/// Segments in the drawn ring.
const ARC: usize = 64;

/// Where a task ended: a ring and a check or an X that draw themselves, a title, a line and actions.
#[derive(IntoElement)]
pub struct ResultView {
    id: ElementId,
    success: bool,
    words: Words,
}

impl ResultView {
    pub fn success(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            success: true,
            words: Words::new(title),
        }
    }

    pub fn failure(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            success: false,
            words: Words::new(title),
        }
    }

    words_builders!();
}

/// `strokes`, each a polyline in the unit square, drawn to `share` of their length.
fn marks(
    bounds: Bounds<Pixels>,
    strokes: &[[(f32, f32); 3]],
    share: f32,
) -> Vec<Vec<Point<Pixels>>> {
    let at = |(x, y): (f32, f32)| {
        point(
            bounds.origin.x + bounds.size.width * x,
            bounds.origin.y + bounds.size.height * y,
        )
    };
    let each = 1.0 / strokes.len() as f32;
    strokes
        .iter()
        .enumerate()
        .filter_map(|(ix, stroke)| {
            let local = ((share - ix as f32 * each) / each).clamp(0.0, 1.0);
            (local > 0.0).then(|| partial(&stroke.map(at), local))
        })
        .collect()
}

/// The first `share` of a polyline's length.
fn partial(points: &[Point<Pixels>], share: f32) -> Vec<Point<Pixels>> {
    let lengths: Vec<f32> = points
        .windows(2)
        .map(|pair| {
            let (x, y) = (
                f32::from(pair[1].x - pair[0].x),
                f32::from(pair[1].y - pair[0].y),
            );
            (x * x + y * y).sqrt()
        })
        .collect();
    let mut left = lengths.iter().sum::<f32>() * share;
    let mut drawn = vec![points[0]];
    for (pair, length) in points.windows(2).zip(lengths) {
        if left >= length {
            drawn.push(pair[1]);
            left -= length;
        } else {
            let t = left / length;
            drawn.push(pair[0] + (pair[1] - pair[0]) * t);
            break;
        }
    }
    drawn
}

fn stroke(points: &[Point<Pixels>], width: Pixels, color: Hsla, window: &mut Window) {
    let mut path = PathBuilder::stroke(width);
    path.move_to(points[0]);
    for at in &points[1..] {
        path.line_to(*at);
    }
    match path.build() {
        Ok(path) => window.paint_path(path, color),
        Err(error) => log::error!("result mark: a stroke failed to build: {error:#}"),
    }
}

impl RenderOnce for ResultView {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let severity = if self.success {
            Severity::Success
        } else {
            Severity::Danger
        };
        let (tone, fill) = (
            severity.color(&theme.colors),
            severity.subtle(&theme.colors),
        );
        let strokes: &'static [[(f32, f32); 3]] = if self.success { &CHECK } else { &CROSS };
        let head = div()
            .size(theme.icon_size(IconSize::Xxl) * 2.0)
            .rounded_full()
            .bg(fill)
            .with_animation(
                (self.id.clone(), "draw"),
                Animation::new(motion::duration(motion::SLOW * 3, cx)),
                move |head, t| {
                    let ring = motion::ease_out_cubic((t / 0.6).min(1.0));
                    let mark = motion::ease_out_cubic(((t - 0.45) / 0.55).clamp(0.0, 1.0));
                    head.child(
                        canvas(
                            |_, _, _| {},
                            move |bounds, _, window, _| {
                                let side = bounds.size.width.min(bounds.size.height);
                                let width = side / 24.0;
                                let (center, radius) = (bounds.center(), side / 2.0 - width);
                                let arc: Vec<Point<Pixels>> = (0..=((ARC as f32 * ring) as usize))
                                    .map(|step| {
                                        let angle = -FRAC_PI_2 + TAU * step as f32 / ARC as f32;
                                        point(
                                            center.x + radius * angle.cos(),
                                            center.y + radius * angle.sin(),
                                        )
                                    })
                                    .collect();
                                if arc.len() > 1 {
                                    stroke(&arc, width, tone, window);
                                }
                                for line in marks(bounds, strokes, mark) {
                                    stroke(&line, width * 1.5, tone, window);
                                }
                            },
                        )
                        .size_full(),
                    )
                },
            )
            .into_any_element();
        state_view(self.id, head, self.words, cx)
    }
}
