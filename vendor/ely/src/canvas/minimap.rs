use std::rc::Rc;

use gpui::{
    App, AppContext as _, Bounds, ElementId, EmptyView, Entity, EntityId, InteractiveElement,
    IntoElement, ParentElement, Pixels, RenderOnce, StatefulInteractiveElement, Styled, Window,
    canvas, div, fill, point, size,
};

use super::{
    plane::OnViewport,
    view::{Frame, Viewport},
};
use crate::theme::{ActiveTheme, Radius};

/// A drag on a minimap, and whose map it is.
struct Steer {
    owner: EntityId,
}

/// Where a map of `whole` shown in a box of `size` puts things: its scale and the offset that centers it.
pub(crate) fn fitted(whole: Frame, (w, h): (f32, f32)) -> (f32, (f32, f32)) {
    let scale = (w / whole.w.max(1.0)).min(h / whole.h.max(1.0));
    let offset = ((w - whole.w * scale) / 2.0, (h - whole.h * scale) / 2.0);
    (scale, offset)
}

/// A small map of what lies on a canvas and the part in view; a press or a drag on it centers the view there.
#[derive(IntoElement)]
pub struct MiniMap {
    id: ElementId,
    frames: Vec<Frame>,
    viewport: Viewport,
    view: (f32, f32),
    on_viewport: Option<OnViewport>,
}

impl MiniMap {
    /// The frames of what lies on the canvas, the viewport, and the size of the canvas's view in pixels.
    pub fn new(
        id: impl Into<ElementId>,
        frames: impl IntoIterator<Item = Frame>,
        viewport: Viewport,
        view: (f32, f32),
    ) -> Self {
        Self {
            id: id.into(),
            frames: frames.into_iter().collect(),
            viewport,
            view,
            on_viewport: None,
        }
    }

    pub fn on_viewport(
        mut self,
        handler: impl Fn(Viewport, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_viewport = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for MiniMap {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let bounds: Entity<Bounds<Pixels>> =
            window.use_keyed_state((self.id.clone(), "box"), cx, |_, _| Bounds::default());
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.canvas(), window.rem_size());
        let (w, h) = (
            f32::from(sizes.map.0.to_pixels(rem)),
            f32::from(sizes.map.1.to_pixels(rem)),
        );
        let (left, top) = self.viewport.to_canvas((0.0, 0.0));
        let seen = Frame::new(
            left,
            top,
            self.view.0 / self.viewport.zoom,
            self.view.1 / self.viewport.zoom,
        );
        let whole = self
            .frames
            .iter()
            .fold(seen, |whole, frame| whole.union(frame));
        let (scale, (dx, dy)) = fitted(whole, (w, h));
        let map = move |frame: &Frame| {
            Frame::new(
                dx + (frame.x - whole.x) * scale,
                dy + (frame.y - whole.y) * scale,
                frame.w * scale,
                frame.h * scale,
            )
        };
        let steer = {
            let (bounds, viewport, view, on_viewport) = (
                bounds.clone(),
                self.viewport,
                self.view,
                self.on_viewport.clone(),
            );
            move |at: gpui::Point<Pixels>, window: &mut Window, cx: &mut App| {
                let origin = bounds.read(cx).origin;
                let (x, y) = (f32::from(at.x - origin.x), f32::from(at.y - origin.y));
                let (cx_, cy_) = (whole.x + (x - dx) / scale, whole.y + (y - dy) / scale);
                let next = Viewport::new(
                    cx_ - view.0 / 2.0 / viewport.zoom,
                    cy_ - view.1 / 2.0 / viewport.zoom,
                    viewport.zoom,
                );
                log::info!("minimap: view at {:.0}, {:.0}", next.x, next.y);
                if let Some(on_viewport) = &on_viewport {
                    on_viewport(next, window, cx);
                }
            }
        };
        let frames: Vec<Frame> = self.frames.iter().map(map).collect();
        let lens = map(&seen);
        let (palette, hairline) = (colors.clone(), sizes.hairline);
        let owner = bounds.entity_id();
        let (pressed, moved, measured) = (steer.clone(), steer, bounds);
        div()
            .id(self.id)
            .relative()
            .w(sizes.map.0)
            .h(sizes.map.1)
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .overflow_hidden()
            .on_mouse_down(gpui::MouseButton::Left, move |event, window, cx| {
                pressed(event.position, window, cx)
            })
            .on_drag(Steer { owner }, |_, _, _, cx| cx.new(|_| EmptyView))
            .on_drag_move::<Steer>(move |event, window, cx| {
                if event.drag(cx).owner == owner {
                    moved(event.event.position, window, cx)
                }
            })
            .child(
                canvas(
                    move |bounds, _, cx| {
                        if *measured.read(cx) != bounds {
                            measured.update(cx, |held, _| *held = bounds);
                        }
                    },
                    move |bounds, _, window, _| {
                        let place = |frame: &Frame| {
                            Bounds::new(
                                bounds.origin + point(Pixels::from(frame.x), Pixels::from(frame.y)),
                                size(Pixels::from(frame.w), Pixels::from(frame.h)),
                            )
                        };
                        for frame in &frames {
                            window.paint_quad(fill(place(frame), palette.border_strong));
                        }
                        window.paint_quad(
                            fill(place(&lens), palette.fg.alpha(0.06))
                                .border_widths(hairline)
                                .border_color(palette.fg_muted),
                        );
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{Frame, fitted};

    #[test]
    fn a_map_fits_the_whole_and_centers_it() {
        let (scale, offset) = fitted(Frame::new(0.0, 0.0, 400.0, 100.0), (200.0, 140.0));
        assert_eq!(scale, 0.5);
        assert_eq!(offset, (0.0, 45.0));
    }
}
