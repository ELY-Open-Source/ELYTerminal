use gpui::{
    App, Bounds, Div, Hsla, IntoElement, ParentElement, PathBuilder, Pixels, Refineable,
    RenderOnce, StyleRefinement, Styled, Window, canvas, div, fill, point, size,
};

use crate::theme::ActiveTheme;

/// How a sparkline draws its series.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Spark {
    Line,
    Area,
    Bars,
}

/// Each value's height as a share of the series' own range; a flat series sits in the middle.
fn shares(values: &[f32]) -> Vec<f32> {
    let (low, high) = values.iter().fold((f32::MAX, f32::MIN), |(low, high), v| {
        (low.min(*v), high.max(*v))
    });
    let span = high - low;
    values
        .iter()
        .map(|value| {
            if span > 0.0 {
                (value - low) / span
            } else {
                0.5
            }
        })
        .collect()
}

/// A word-sized chart of a series, scaled to its own range: a line, a filled area, or bars.
#[derive(IntoElement)]
pub struct Sparkline {
    base: Div,
    values: Vec<f32>,
    spark: Spark,
    color: Option<Hsla>,
}

impl Sparkline {
    pub fn new(values: impl IntoIterator<Item = f32>) -> Self {
        let values: Vec<f32> = values.into_iter().collect();
        assert!(values.len() >= 2, "a sparkline needs two values");
        assert!(
            values.iter().all(|value| value.is_finite()),
            "a sparkline needs finite values"
        );
        Self {
            base: div(),
            values,
            spark: Spark::Line,
            color: None,
        }
    }

    /// The line with the area under it lightly filled.
    pub fn area(mut self) -> Self {
        self.spark = Spark::Area;
        self
    }

    pub fn bars(mut self) -> Self {
        self.spark = Spark::Bars;
        self
    }

    /// Its ink; the foreground unless set.
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

impl Styled for Sparkline {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Sparkline {
    fn render(mut self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let ink = self.color.unwrap_or(theme.colors.fg_muted);
        let (heights, spark) = (shares(&self.values), self.spark);
        let mut frame = div()
            .flex_none()
            .w(theme.spark_size().width)
            .h(theme.spark_size().height);
        frame.style().refine(self.base.style());
        frame.child(
            canvas(
                |_, _, _| {},
                move |bounds: Bounds<Pixels>, _, window, _| {
                    let stroke = bounds.size.height / 12.0;
                    let inner = bounds.size.height - stroke * 2.0;
                    let count = heights.len() as f32;
                    if spark == Spark::Bars {
                        let slot = bounds.size.width / count;
                        for (ix, share) in heights.iter().enumerate() {
                            let tall = (inner * *share).max(stroke);
                            let corner = point(
                                bounds.left() + slot * ix as f32 + slot * 0.15,
                                bounds.bottom() - tall,
                            );
                            window
                                .paint_quad(fill(Bounds::new(corner, size(slot * 0.7, tall)), ink));
                        }
                        return;
                    }
                    let at = |ix: usize, share: f32| {
                        point(
                            bounds.left() + bounds.size.width * ix as f32 / (count - 1.0),
                            bounds.top() + stroke + inner * (1.0 - share),
                        )
                    };
                    if spark == Spark::Area {
                        let mut area = PathBuilder::fill();
                        area.move_to(point(bounds.left(), bounds.bottom()));
                        for (ix, share) in heights.iter().enumerate() {
                            area.line_to(at(ix, *share));
                        }
                        area.line_to(point(bounds.right(), bounds.bottom()));
                        area.close();
                        if let Ok(area) = area.build() {
                            window.paint_path(area, ink.opacity(0.16));
                        }
                    }
                    let mut line = PathBuilder::stroke(stroke);
                    for (ix, share) in heights.iter().enumerate() {
                        if ix == 0 {
                            line.move_to(at(ix, *share));
                        } else {
                            line.line_to(at(ix, *share));
                        }
                    }
                    match line.build() {
                        Ok(line) => window.paint_path(line, ink),
                        Err(error) => log::error!("sparkline: the line failed to build: {error:#}"),
                    }
                },
            )
            .size_full(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::shares;

    #[test]
    fn values_scale_to_their_own_range() {
        assert_eq!(shares(&[2.0, 4.0, 3.0]), [0.0, 1.0, 0.5]);
        assert_eq!(
            shares(&[7.0, 7.0]),
            [0.5, 0.5],
            "a flat series sits in the middle"
        );
    }
}
