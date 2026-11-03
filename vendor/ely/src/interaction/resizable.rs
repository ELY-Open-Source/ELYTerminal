use std::rc::Rc;

use gpui::{
    AnyElement, App, AppContext, Axis, CursorStyle, DragMoveEvent, ElementId, EmptyView, EntityId,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement, Pixels, Point,
    RenderOnce, Size, StatefulInteractiveElement, Styled, Window, div, point, size,
};
use smallvec::SmallVec;

use crate::{
    layout::{resize_edge, seeded::use_seeded},
    primitives::FocusRing,
    theme::{ActiveTheme, Radius},
};

/// Which side a drag moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Right,
    Bottom,
    Corner,
}

struct ResizeDrag {
    owner: EntityId,
    side: Side,
    start: Size<Pixels>,
}

/// `size` within `min` and `max`.
fn fit(size: Size<Pixels>, (min, max): (Size<Pixels>, Size<Pixels>)) -> Size<Pixels> {
    gpui::size(
        size.width.clamp(min.width, max.width),
        size.height.clamp(min.height, max.height),
    )
}

/// The size after moving `side` by `delta` from `start`, both measures kept within the limits.
fn resized(
    start: Size<Pixels>,
    side: Side,
    delta: Point<Pixels>,
    limits: (Size<Pixels>, Size<Pixels>),
) -> Size<Pixels> {
    let width = match side {
        Side::Bottom => start.width,
        Side::Right | Side::Corner => start.width + delta.x,
    };
    let height = match side {
        Side::Right => start.height,
        Side::Bottom | Side::Corner => start.height + delta.y,
    };
    fit(size(width, height), limits)
}

type OnResize = Rc<dyn Fn(Size<Pixels>, &mut Window, &mut App)>;

/// A box the owner sizes, with handles on its right and bottom edges and a grip at its lower right corner. A drag resizes it within its least and greatest size, and the owner hears each new size; a new size from the owner replaces it. The grip is a Tab stop, and the arrows resize a step at a time.
#[derive(IntoElement)]
pub struct Resizable {
    id: ElementId,
    size: Size<Pixels>,
    limits: (Size<Pixels>, Size<Pixels>),
    children: SmallVec<[AnyElement; 1]>,
    on_resize: OnResize,
}

impl Resizable {
    /// `min` and `max` bound the size, which starts within them.
    pub fn new(
        id: impl Into<ElementId>,
        size: Size<Pixels>,
        min: Size<Pixels>,
        max: Size<Pixels>,
        on_resize: impl Fn(Size<Pixels>, &mut Window, &mut App) + 'static,
    ) -> Self {
        let id = id.into();
        assert!(
            min.width <= max.width && min.height <= max.height,
            "resizable {id:?}: limits {min:?} to {max:?} run backwards"
        );
        if fit(size, (min, max)) != size {
            log::error!("resizable {id:?}: size {size:?} outside {min:?} to {max:?}; pegged");
        }
        Self {
            id,
            size,
            limits: (min, max),
            children: SmallVec::new(),
            on_resize: Rc::new(on_resize),
        }
    }
}

impl ParentElement for Resizable {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Resizable {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let on_resize = self.on_resize;
        let limits = self.limits;
        let held = use_seeded((id.clone(), "size"), self.size, window, cx);
        let pressed =
            window.use_keyed_state((id.clone(), "press"), cx, |_, _| None::<Point<Pixels>>);
        let (owner, kept) = (held.entity_id(), held.read(cx).value);
        let now = fit(kept, limits);
        if now != kept {
            log::info!("resizable: {kept:?} fits the limits at {now:?}");
            held.update(cx, |held, _| held.value = now);
        }
        let report: OnResize = Rc::new(move |next, window, cx| {
            log::info!("resizable: {:?} by {:?}", next.width, next.height);
            held.update(cx, |held, cx| {
                held.value = next;
                cx.notify();
            });
            on_resize(next, window, cx);
        });
        let theme = cx.theme();
        let grip = theme.interaction().grip;
        let step = theme.interaction().step.to_pixels(window.rem_size());
        let grab = |side| {
            let drag = ResizeDrag {
                owner,
                side,
                start: now,
            };
            let at = pressed.clone();
            let press = move |event: &MouseDownEvent, _: &mut Window, cx: &mut App| {
                at.update(cx, |at, _| *at = Some(event.position))
            };
            (drag, press)
        };
        let ((right, right_press), (bottom, bottom_press), (corner, corner_press)) =
            (grab(Side::Right), grab(Side::Bottom), grab(Side::Corner));
        let (dragged, keyed) = (report.clone(), report);
        div()
            .id(id.clone())
            .debug_selector(|| "resizable-box".into())
            .relative()
            .w(now.width)
            .h(now.height)
            .on_drag_move(move |event: &DragMoveEvent<ResizeDrag>, window, cx| {
                let drag = event.drag(cx);
                if drag.owner != owner {
                    return;
                }
                let at = event.event.position;
                let Some(anchor) = *pressed.read(cx) else {
                    log::error!("resizable: a drag with no press behind it");
                    return;
                };
                let next = resized(drag.start, drag.side, at - anchor, limits);
                if next != now {
                    dragged(next, window, cx);
                }
            })
            .children(self.children)
            .child(
                resize_edge((id.clone(), "right"), Axis::Horizontal, cx)
                    .absolute()
                    .top_0()
                    .right_0()
                    .justify_end()
                    .on_mouse_down(MouseButton::Left, right_press)
                    .on_drag(right, |_, _, _, cx| cx.new(|_| EmptyView)),
            )
            .child(
                resize_edge((id.clone(), "bottom"), Axis::Vertical, cx)
                    .absolute()
                    .left_0()
                    .bottom_0()
                    .items_end()
                    .on_mouse_down(MouseButton::Left, bottom_press)
                    .on_drag(bottom, |_, _, _, cx| cx.new(|_| EmptyView)),
            )
            .child(
                div()
                    .id((id, "corner"))
                    .debug_selector(|| "resizable-grip".into())
                    .absolute()
                    .right_0()
                    .bottom_0()
                    .size(grip)
                    .rounded(theme.radius(Radius::Sm))
                    .border_1()
                    .border_color(theme.colors.border_strong)
                    .bg(theme.colors.surface)
                    .cursor(CursorStyle::ResizeUpLeftDownRight)
                    .tab_index(0)
                    .focus_ring(cx)
                    .on_key_down(move |event, window, cx| {
                        let delta = match event.keystroke.key.as_str() {
                            "left" => point(-step, Pixels::ZERO),
                            "right" => point(step, Pixels::ZERO),
                            "up" => point(Pixels::ZERO, -step),
                            "down" => point(Pixels::ZERO, step),
                            _ => return,
                        };
                        cx.stop_propagation();
                        let next = resized(now, Side::Corner, delta, limits);
                        if next != now {
                            keyed(next, window, cx);
                        }
                    })
                    .on_mouse_down(MouseButton::Left, corner_press)
                    .on_drag(corner, |_, _, _, cx| cx.new(|_| EmptyView)),
            )
    }
}

#[cfg(test)]
mod tests {
    use gpui::{point, px, size};

    use super::{Side, resized};

    #[test]
    fn a_side_moves_its_own_measure_within_the_limits() {
        let limits = (size(px(100.0), px(80.0)), size(px(400.0), px(300.0)));
        let start = size(px(200.0), px(150.0));
        let by = point(px(50.0), px(-500.0));
        assert_eq!(
            resized(start, Side::Right, by, limits),
            size(px(250.0), px(150.0))
        );
        assert_eq!(
            resized(start, Side::Bottom, by, limits),
            size(px(200.0), px(80.0))
        );
        assert_eq!(
            resized(start, Side::Corner, by, limits),
            size(px(250.0), px(80.0))
        );
        let far = point(px(900.0), px(900.0));
        assert_eq!(
            resized(start, Side::Corner, far, limits),
            size(px(400.0), px(300.0))
        );
    }
}
