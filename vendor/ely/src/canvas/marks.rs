use gpui::{
    App, Hsla, InteractiveElement, IntoElement, ParentElement, Pixels, RenderOnce, Styled, Window,
    div,
};

use super::{
    edit::Handle,
    shape::Guide,
    view::{Frame, Viewport},
};
use crate::theme::ActiveTheme;

/// A box in view pixels as an absolute element.
pub(super) fn placed(frame: &Frame) -> gpui::Div {
    div()
        .absolute()
        .left(Pixels::from(frame.x))
        .top(Pixels::from(frame.y))
        .w(Pixels::from(frame.w))
        .h(Pixels::from(frame.h))
}

/// The marquee a drag on empty canvas draws, in view pixels: a faint wash in the accent and its edge.
#[derive(IntoElement)]
pub struct SelectionBox {
    frame: Frame,
}

impl SelectionBox {
    pub fn new(frame: Frame) -> Self {
        Self { frame }
    }
}

impl RenderOnce for SelectionBox {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let accent = cx.theme().colors.accent;
        placed(&self.frame)
            .bg(accent.alpha(0.08))
            .border_1()
            .border_color(accent)
    }
}

/// A selection's frame in view pixels with a square handle at each corner and side.
#[derive(IntoElement)]
pub struct TransformHandles {
    frame: Frame,
}

impl TransformHandles {
    pub fn new(frame: Frame) -> Self {
        Self { frame }
    }
}

/// Where each handle's square sits, in view pixels, for a frame and a handle `side` wide.
pub(crate) fn handle_boxes(frame: &Frame, side: f32) -> Vec<(Handle, Frame)> {
    Handle::ALL
        .into_iter()
        .map(|handle| {
            let (sx, sy) = handle.at();
            let (x, y) = (
                frame.x + frame.w * sx - side / 2.0,
                frame.y + frame.h * sy - side / 2.0,
            );
            (handle, Frame::new(x, y, side, side))
        })
        .collect()
}

impl RenderOnce for TransformHandles {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (accent, surface) = (theme.colors.accent, theme.colors.surface);
        let side = f32::from(theme.canvas().handle.to_pixels(window.rem_size()));
        let squares = handle_boxes(&self.frame, side)
            .into_iter()
            .map(move |(handle, frame)| {
                placed(&frame)
                    .bg(surface)
                    .border_1()
                    .border_color(accent)
                    .debug_selector(move || format!("handle {handle:?}"))
            });
        div()
            .absolute()
            .inset_0()
            .child(placed(&self.frame).border_1().border_color(accent))
            .children(squares)
    }
}

/// The lines a moving shape snaps to, across the whole view.
#[derive(IntoElement)]
pub struct SnapIndicator {
    guides: Vec<Guide>,
    viewport: Viewport,
}

impl SnapIndicator {
    pub fn new(guides: impl IntoIterator<Item = Guide>, viewport: Viewport) -> Self {
        Self {
            guides: guides.into_iter().collect(),
            viewport,
        }
    }
}

impl RenderOnce for SnapIndicator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let line: Hsla = cx.theme().colors.hue(3, "snap");
        let view = self.viewport;
        div()
            .absolute()
            .inset_0()
            .children(self.guides.into_iter().map(move |guide| {
                match guide {
                    Guide::Vertical(x) => div()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left(Pixels::from(view.to_view((x, 0.0)).0))
                        .border_l_1()
                        .border_color(line),
                    Guide::Horizontal(y) => div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .top(Pixels::from(view.to_view((0.0, y)).1))
                        .border_t_1()
                        .border_color(line),
                }
            }))
    }
}
