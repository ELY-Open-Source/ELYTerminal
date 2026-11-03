use gpui::{
    AnyElement, App, Div, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, Window,
    div,
};
use smallvec::SmallVec;

use crate::theme::{ActiveTheme, ContainerSize};

/// Row with children centered on the cross axis.
pub fn h_stack() -> Div {
    div().flex().flex_row().items_center()
}

/// Column.
pub fn v_stack() -> Div {
    div().flex().flex_col()
}

/// Layers: the first child sets the size, the rest cover it.
#[derive(IntoElement)]
pub struct ZStack {
    base: Div,
    layers: SmallVec<[AnyElement; 2]>,
}

impl ZStack {
    pub fn new() -> Self {
        Self {
            base: div(),
            layers: SmallVec::new(),
        }
    }
}

impl Default for ZStack {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for ZStack {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for ZStack {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.layers.extend(elements);
    }
}

impl RenderOnce for ZStack {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let mut layers = self.layers.into_iter();
        let ground = layers.next();
        self.base
            .relative()
            .children(ground)
            .children(layers.map(|layer| div().absolute().inset_0().child(layer)))
    }
}

/// Keeps width to height at `ratio`; width fills the parent.
#[derive(IntoElement)]
pub struct AspectRatio {
    base: Div,
    ratio: f32,
}

impl AspectRatio {
    /// Ratios such as `16.0 / 9.0`. Must be positive.
    pub fn new(ratio: f32) -> Self {
        assert!(
            ratio.is_finite() && ratio > 0.0,
            "aspect ratio {ratio} is not positive"
        );
        Self { base: div(), ratio }
    }
}

impl Styled for AspectRatio {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for AspectRatio {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl RenderOnce for AspectRatio {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let mut base = self.base.w_full().overflow_hidden();
        base.style().aspect_ratio = Some(self.ratio);
        base
    }
}

/// Centered column with a readable maximum width.
#[derive(IntoElement)]
pub struct Container {
    base: Div,
    size: ContainerSize,
}

impl Container {
    pub fn new(size: ContainerSize) -> Self {
        Self { base: div(), size }
    }
}

impl Styled for Container {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Container {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl RenderOnce for Container {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        self.base
            .w_full()
            .max_w(cx.theme().container_width(self.size))
            .mx_auto()
            .px_6()
    }
}
