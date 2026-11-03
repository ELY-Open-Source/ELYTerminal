use std::{
    f32::consts::{FRAC_PI_2, TAU},
    time::Duration,
};

use gpui::{
    Animation, AnimationExt, App, Bounds, ElementId, Hsla, IntoElement, PathBuilder, Pixels, Point,
    RenderOnce, Styled, Window, canvas, div, fill, point, prelude::*, size,
};

use crate::theme::{ActiveTheme, IconSize};

/// How a spinner moves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpinnerStyle {
    /// An arc that runs around a faint ring.
    #[default]
    Ring,
    /// Three dots that brighten in turn.
    Dots,
    /// Four bars that rise in turn.
    Bars,
    /// A dot that sends out a fading ring.
    Pulse,
    /// A dot that circles a faint ring.
    Orbit,
    /// Three dots that bob in a wave.
    Wave,
}

impl SpinnerStyle {
    fn period(self) -> Duration {
        Duration::from_millis(match self {
            Self::Ring => 900,
            Self::Dots | Self::Wave => 1_200,
            Self::Bars => 1_000,
            Self::Pulse => 1_400,
            Self::Orbit => 1_100,
        })
    }
}

/// Where in its cycle a still spinner rests.
const STILL: f32 = 0.3;
/// Segments in a drawn arc.
const ARC: usize = 32;

/// A small busy mark, one of six motions. Under reduced motion it rests on one frame.
#[derive(IntoElement)]
pub struct Spinner {
    id: ElementId,
    style: SpinnerStyle,
    size: IconSize,
    color: Option<Hsla>,
}

impl Spinner {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: SpinnerStyle::default(),
            size: IconSize::Md,
            color: None,
        }
    }

    pub fn style(mut self, style: SpinnerStyle) -> Self {
        self.style = style;
        self
    }

    pub fn size(mut self, size: IconSize) -> Self {
        self.size = size;
        self
    }

    /// Defaults to muted text.
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

/// A 0..1 bump that peaks once per cycle, `lag` cycles late.
fn bump(t: f32, lag: f32) -> f32 {
    0.5 - 0.5 * (TAU * (t - lag)).cos()
}

fn dot(center: Point<Pixels>, radius: Pixels, color: Hsla, window: &mut Window) {
    let side = radius * 2.0;
    let corner = point(center.x - radius, center.y - radius);
    window.paint_quad(fill(Bounds::new(corner, size(side, side)), color).corner_radii(radius));
}

pub(crate) fn arc(
    center: Point<Pixels>,
    radius: Pixels,
    from: f32,
    sweep: f32,
    width: Pixels,
    color: Hsla,
    window: &mut Window,
) {
    let steps = ((ARC as f32 * sweep / TAU).ceil() as usize).max(2);
    let mut path = PathBuilder::stroke(width);
    for step in 0..=steps {
        let angle = from + sweep * step as f32 / steps as f32;
        let at = point(
            center.x + radius * angle.cos(),
            center.y + radius * angle.sin(),
        );
        if step == 0 {
            path.move_to(at);
        } else {
            path.line_to(at);
        }
    }
    match path.build() {
        Ok(path) => window.paint_path(path, color),
        Err(error) => log::error!("spinner: an arc failed to build: {error:#}"),
    }
}

/// One frame of `style` at `t` in its cycle.
fn paint(style: SpinnerStyle, bounds: Bounds<Pixels>, t: f32, color: Hsla, window: &mut Window) {
    let side = bounds.size.width.min(bounds.size.height);
    let center = bounds.center();
    let stroke = side / 8.0;
    let faint = color.opacity(0.2);
    match style {
        SpinnerStyle::Ring => {
            let radius = side / 2.0 - stroke;
            arc(center, radius, 0.0, TAU, stroke, faint, window);
            let sweep = TAU * (0.2 + 0.3 * bump(t, 0.0));
            arc(
                center,
                radius,
                -FRAC_PI_2 + TAU * t,
                sweep,
                stroke,
                color,
                window,
            );
        }
        SpinnerStyle::Dots | SpinnerStyle::Wave => {
            let radius = side / 9.0;
            for ix in 0..3 {
                let lag = ix as f32 * 0.18;
                let x = center.x + (ix as f32 - 1.0) * radius * 3.2;
                if style == SpinnerStyle::Dots {
                    dot(
                        point(x, center.y),
                        radius,
                        color.opacity(0.3 + 0.7 * bump(t, lag)),
                        window,
                    );
                } else {
                    let lift = side * 0.22 * bump(t, lag);
                    dot(
                        point(x, center.y + side * 0.11 - lift),
                        radius,
                        color,
                        window,
                    );
                }
            }
        }
        SpinnerStyle::Bars => {
            let width = side / 8.0;
            for ix in 0..4 {
                let height = side * (0.3 + 0.55 * bump(t, ix as f32 * 0.14));
                let x = center.x + (ix as f32 - 1.5) * width * 2.0 - width / 2.0;
                let top = center.y - height / 2.0;
                window.paint_quad(
                    fill(Bounds::new(point(x, top), size(width, height)), color)
                        .corner_radii(width / 2.0),
                );
            }
        }
        SpinnerStyle::Pulse => {
            let core = side / 6.0;
            let reach = core + (side / 2.0 - core) * t;
            arc(
                center,
                reach,
                0.0,
                TAU,
                stroke * 0.75,
                color.opacity(1.0 - t),
                window,
            );
            dot(center, core, color, window);
        }
        SpinnerStyle::Orbit => {
            let radius = side / 2.0 - stroke;
            arc(center, radius, 0.0, TAU, stroke * 0.75, faint, window);
            let angle = -FRAC_PI_2 + TAU * t;
            let at = point(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            );
            dot(at, stroke * 1.1, color, window);
        }
    }
}

impl RenderOnce for Spinner {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let color = self.color.unwrap_or(theme.colors.fg_muted);
        let style = self.style;
        let face = move |t: f32| {
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| paint(style, bounds, t, color, window),
            )
            .size_full()
        };
        let frame = div().flex_none().size(theme.icon_size(self.size));
        if theme.reduced_motion {
            return frame.child(face(STILL)).into_any_element();
        }
        frame
            .with_animation(
                self.id,
                Animation::new(style.period()).repeat(),
                move |frame, t| frame.child(face(t)),
            )
            .into_any_element()
    }
}
