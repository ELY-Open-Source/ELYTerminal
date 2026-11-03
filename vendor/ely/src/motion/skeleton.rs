use std::{f32::consts::TAU, time::Duration};

use gpui::{
    AbsoluteLength, Animation, AnimationExt, AnyElement, App, Bounds, Div, ElementId, Hsla,
    IntoElement, ParentElement, PathBuilder, Pixels, RenderOnce, StyleRefinement, Styled, Window,
    canvas, div, linear_color_stop, linear_gradient, point, prelude::*, px, relative,
};
use smallvec::SmallVec;

use crate::theme::{ActiveTheme, ControlSize, Mix, Radius, TextSize};

/// One breath of a skeleton, in and out.
const BREATH: Duration = Duration::from_millis(1_600);
/// One pass of a shimmer's band.
const SWEEP: Duration = Duration::from_millis(1_800);
/// Columns in each half of a shimmer's band, where it meets rounded corners.
const RAMP: usize = 12;
/// Widths a skeleton table's cells cycle through.
const CELLS: [f32; 4] = [0.72, 0.48, 0.6, 0.36];

/// A placeholder in the shape of what is loading; it breathes until the content arrives. Size it as the content.
#[derive(IntoElement)]
pub struct Skeleton {
    id: ElementId,
    base: Div,
}

impl Skeleton {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div(),
        }
    }

    /// Round, for avatars and icons.
    pub fn circle(mut self) -> Self {
        self.base = self.base.rounded_full();
        self
    }
}

impl Styled for Skeleton {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Skeleton {
    fn render(mut self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let mut block = div()
            .rounded(theme.radius(Radius::Sm))
            .bg(theme.colors.active);
        block.style().refine(self.base.style());
        if theme.reduced_motion {
            return block.into_any_element();
        }
        block
            .with_animation(self.id, Animation::new(BREATH).repeat(), |block, t| {
                block.opacity(0.775 + 0.225 * (TAU * t).cos())
            })
            .into_any_element()
    }
}

/// Lines of text to come; the last runs short.
#[derive(IntoElement)]
pub struct SkeletonText {
    id: ElementId,
    lines: usize,
}

impl SkeletonText {
    pub fn new(id: impl Into<ElementId>, lines: usize) -> Self {
        assert!(lines > 0, "a skeleton text needs a line");
        Self {
            id: id.into(),
            lines,
        }
    }
}

impl RenderOnce for SkeletonText {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let line = cx.theme().text_size(TextSize::Base);
        div()
            .flex()
            .flex_col()
            .gap_2()
            .children((0..self.lines).map(|ix| {
                let width = if ix + 1 == self.lines && self.lines > 1 {
                    0.6
                } else {
                    1.0
                };
                Skeleton::new((self.id.clone(), format!("line-{ix}")))
                    .h(line)
                    .w(relative(width))
            }))
    }
}

/// A round avatar to come.
#[derive(IntoElement)]
pub struct SkeletonAvatar {
    id: ElementId,
}

impl SkeletonAvatar {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self { id: id.into() }
    }
}

impl RenderOnce for SkeletonAvatar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        Skeleton::new(self.id)
            .circle()
            .flex_none()
            .size(cx.theme().control_height(ControlSize::Lg))
    }
}

/// A card to come: who, a picture and a few lines.
#[derive(IntoElement)]
pub struct SkeletonCard {
    id: ElementId,
}

impl SkeletonCard {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self { id: id.into() }
    }
}

impl RenderOnce for SkeletonCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let line = theme.text_size(TextSize::Base);
        let id = |part: &'static str| (self.id.clone(), part);
        div()
            .flex()
            .flex_col()
            .gap_4()
            .p_4()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.surface)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(SkeletonAvatar::new(id("avatar")))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(Skeleton::new(id("name")).h(line).w(relative(0.45)))
                            .child(Skeleton::new(id("when")).h(line).w(relative(0.25))),
                    ),
            )
            .child(
                Skeleton::new(id("picture"))
                    .w_full()
                    .h(theme.skeleton_media())
                    .rounded(theme.radius(Radius::Md)),
            )
            .child(SkeletonText::new(id("text"), 2))
    }
}

/// A table to come: a header and rows of cells.
#[derive(IntoElement)]
pub struct SkeletonTable {
    id: ElementId,
    rows: usize,
    columns: usize,
}

impl SkeletonTable {
    pub fn new(id: impl Into<ElementId>, rows: usize, columns: usize) -> Self {
        assert!(rows > 0 && columns > 0, "a skeleton table needs a cell");
        Self {
            id: id.into(),
            rows,
            columns,
        }
    }
}

impl RenderOnce for SkeletonTable {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (line, border) = (theme.text_size(TextSize::Base), theme.colors.border);
        let row = |row: usize| {
            div()
                .flex()
                .gap_4()
                .py_3()
                .border_b_1()
                .border_color(border)
                .children((0..self.columns).map(|column| {
                    let width = if row == 0 {
                        0.4
                    } else {
                        CELLS[(row + column) % CELLS.len()]
                    };
                    div().flex_1().child(
                        Skeleton::new((self.id.clone(), format!("cell-{row}-{column}")))
                            .h(line)
                            .w(relative(width)),
                    )
                }))
        };
        div().flex().flex_col().children((0..=self.rows).map(row))
    }
}

/// Sweeps a soft band of light across what it holds while that loads. It holds still under reduced motion.
#[derive(IntoElement)]
pub struct Shimmer {
    id: ElementId,
    base: Div,
    body: SmallVec<[AnyElement; 2]>,
}

impl Shimmer {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            body: SmallVec::new(),
        }
    }
}

impl Styled for Shimmer {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Shimmer {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

/// The corners' radii, clockwise from the top left.
type Radii = [Pixels; 4];

/// A style's corner radii in pixels, clockwise from the top left.
pub(super) fn radii(style: &StyleRefinement, rem: Pixels) -> Radii {
    let corners = &style.corner_radii;
    let radius = |corner: Option<AbsoluteLength>| corner.map_or(Pixels::ZERO, |r| r.to_pixels(rem));
    [
        radius(corners.top_left),
        radius(corners.top_right),
        radius(corners.bottom_right),
        radius(corners.bottom_left),
    ]
}

/// Where a rounded box's outline crosses the column at `x`: its top and bottom. Radii past half the box are clamped, as gpui draws them.
pub(super) fn span(bounds: Bounds<Pixels>, radii: Radii, x: Pixels) -> (Pixels, Pixels) {
    let limit = bounds.size.width.min(bounds.size.height) / 2.0;
    let [tl, tr, br, bl] = radii.map(|radius| radius.min(limit));
    let bite = |radius: Pixels, from_edge: Pixels| {
        if radius <= Pixels::ZERO || from_edge >= radius {
            return Pixels::ZERO;
        }
        let d = f32::from(radius - from_edge.max(Pixels::ZERO));
        radius - px((f32::from(radius).powi(2) - d * d).max(0.0).sqrt())
    };
    let (left, right) = (x - bounds.left(), bounds.right() - x);
    let top = bite(tl, left).max(bite(tr, right));
    let bottom = bite(bl, left).max(bite(br, right));
    (bounds.top() + top, bounds.bottom() - bottom)
}

/// Fills the part of the rounded box between `from` and `to` with a ramp from `start` to `end`.
fn ramp(
    bounds: Bounds<Pixels>,
    radii: Radii,
    (from, to): (Pixels, Pixels),
    (start, end): (Hsla, Hsla),
    window: &mut Window,
) {
    let (a, b) = (from.max(bounds.left()), to.min(bounds.right()));
    if a >= b {
        return;
    }
    let at = |x: Pixels| {
        let share = f32::from(x - from) / f32::from(to - from);
        start.mix(&end, share)
    };
    let columns: Vec<Pixels> = (0..=RAMP)
        .map(|ix| a + (b - a) * (ix as f32 / RAMP as f32))
        .collect();
    let mut path = PathBuilder::fill();
    for (ix, x) in columns.iter().enumerate() {
        let top = point(*x, span(bounds, radii, *x).0);
        if ix == 0 {
            path.move_to(top);
        } else {
            path.line_to(top);
        }
    }
    for x in columns.iter().rev() {
        path.line_to(point(*x, span(bounds, radii, *x).1));
    }
    path.close();
    match path.build() {
        Ok(path) => window.paint_path(
            path,
            linear_gradient(
                90.0,
                linear_color_stop(at(a), 0.0),
                linear_color_stop(at(b), 1.0),
            ),
        ),
        Err(error) => log::error!("shimmer: a band failed to build: {error:#}"),
    }
}

impl RenderOnce for Shimmer {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (shine, clear) = (theme.colors.shimmer, theme.colors.shimmer.alpha(0.0));
        let radii = radii(self.base.style(), window.rem_size());
        let still = theme.reduced_motion;
        let base = self.base.relative().children(self.body);
        if still {
            return base.into_any_element();
        }
        base.child(
            div().absolute().inset_0().with_animation(
                self.id,
                Animation::new(SWEEP)
                    .repeat()
                    .with_easing(crate::motion::ease_in_out_cubic),
                move |overlay, t| {
                    overlay.child(
                        canvas(
                            |_, _, _| {},
                            move |bounds, _, window, _| {
                                let width = bounds.size.width * 0.4;
                                let from = bounds.left() + bounds.size.width * (-0.4 + 1.4 * t);
                                let middle = from + width / 2.0;
                                ramp(bounds, radii, (from, middle), (clear, shine), window);
                                ramp(
                                    bounds,
                                    radii,
                                    (middle, from + width),
                                    (shine, clear),
                                    window,
                                );
                            },
                        )
                        .size_full(),
                    )
                },
            ),
        )
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, point, px, size};

    use super::span;

    #[test]
    fn a_band_keeps_inside_rounded_corners() {
        let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(100.0), px(50.0)));
        let radii = [px(8.0); 4];
        assert_eq!(span(bounds, radii, px(0.0)), (px(8.0), px(42.0)));
        assert_eq!(span(bounds, radii, px(50.0)), (px(0.0), px(50.0)));
        assert_eq!(span(bounds, radii, px(100.0)), (px(8.0), px(42.0)));
        let pill = [px(9999.0); 4];
        assert_eq!(span(bounds, pill, px(0.0)), (px(25.0), px(25.0)));
        assert_eq!(span(bounds, pill, px(50.0)), (px(0.0), px(50.0)));
        let (top, _) = span(bounds, radii, px(4.0));
        assert!(top > px(0.0) && top < px(8.0), "the arc eases in: {top:?}");
    }
}
