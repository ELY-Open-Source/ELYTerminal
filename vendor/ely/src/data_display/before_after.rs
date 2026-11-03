use gpui::{
    AnyElement, App, Bounds, Div, DragMoveEvent, ElementId, EmptyView, EntityId,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, Point, RenderOnce,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, canvas, div, prelude::*, relative,
};

use crate::{
    primitives::{FocusRing, Icon, IconName, tab_stop},
    theme::{ActiveTheme, ControlSize, Elevation, IconSize},
};

/// A drag of one comparison's divider.
struct Divider {
    owner: EntityId,
}

/// Where the divider stands, as a share of its travel, and the box it travels in.
#[derive(Clone, Copy)]
struct Split {
    share: f32,
    bounds: Bounds<Pixels>,
}

/// The share of the travel under `at`; the travel stops `inset` short of each side. None when the box is narrower than the handle.
fn share_at(at: Point<Pixels>, bounds: Bounds<Pixels>, inset: Pixels) -> Option<f32> {
    let travel = bounds.size.width - inset * 2.0;
    (travel > Pixels::ZERO).then(|| ((at.x - bounds.left() - inset) / travel).clamp(0.0, 1.0))
}

/// Two versions of a picture, one over the other: `before` shows left of the divider, `after` right of it. Drag the divider, press anywhere, or use Left and Right. Give it a size.
#[derive(IntoElement)]
pub struct BeforeAfter {
    id: ElementId,
    base: Div,
    before: AnyElement,
    after: AnyElement,
}

impl BeforeAfter {
    pub fn new(
        id: impl Into<ElementId>,
        before: impl IntoElement,
        after: impl IntoElement,
    ) -> Self {
        Self {
            id: id.into(),
            base: div(),
            before: before.into_any_element(),
            after: after.into_any_element(),
        }
    }
}

impl Styled for BeforeAfter {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for BeforeAfter {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus = tab_stop((self.id.clone(), "focus").into(), true, window, cx);
        let split = window.use_keyed_state((self.id.clone(), "split"), cx, |_, _| Split {
            share: 0.5,
            bounds: Bounds::default(),
        });
        let Split { share, bounds } = *split.read(cx);
        let owner = split.entity_id();
        let theme = cx.theme();
        let colors = &theme.colors;
        let handle = theme.control_height(ControlSize::Lg);
        let inset = handle.to_pixels(window.rem_size()) / 2.0;
        let width = bounds.size.width;
        let x = inset + (width - inset * 2.0) * share;
        let set = {
            let (split, id) = (split.clone(), self.id.clone());
            move |share: f32, cx: &mut App| {
                split.update(cx, |split, cx| {
                    split.share = share;
                    log::debug!("before-after {id:?}: divider at {share:.2}");
                    cx.notify();
                })
            }
        };
        let (press, drag, keys, measured) = (set.clone(), set.clone(), set, split.clone());
        let overlay = (width > Pixels::ZERO).then(|| {
            [
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left_0()
                    .w(x)
                    .overflow_hidden()
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .left_0()
                            .w(width)
                            .child(self.before),
                    )
                    .into_any_element(),
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(x)
                    .border_l_1()
                    .border_color(colors.on_media)
                    .into_any_element(),
                div()
                    .absolute()
                    .top(relative(0.5))
                    .mt(handle * -0.5)
                    .left(x - inset)
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(handle)
                    .rounded_full()
                    .bg(colors.on_media)
                    .shadow(theme.elevation(Elevation::Raised))
                    .child(
                        Icon::new(IconName::ChevronsLeftRight)
                            .size(IconSize::Sm)
                            .color(colors.media_backdrop),
                    )
                    .into_any_element(),
            ]
        });
        self.base
            .id(self.id)
            .relative()
            .overflow_hidden()
            .border_1()
            .border_color(gpui::transparent_black())
            .track_focus(&focus)
            .focus_ring(cx)
            .cursor_ew_resize()
            .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                let bounds = measured.read(cx).bounds;
                if let Some(share) = share_at(event.position, bounds, inset) {
                    press(share, cx);
                }
            })
            .on_drag(Divider { owner }, |_, _, _, cx| cx.new(|_| EmptyView))
            .on_drag_move(move |event: &DragMoveEvent<Divider>, _, cx| {
                if event.drag(cx).owner != owner {
                    return;
                }
                if let Some(share) = share_at(event.event.position, bounds, inset) {
                    drag(share, cx);
                }
            })
            .on_key_down(move |event, _, cx| {
                let to = match event.keystroke.key.as_str() {
                    "left" => share - 0.05,
                    "right" => share + 0.05,
                    _ => return,
                };
                cx.stop_propagation();
                keys(to.clamp(0.0, 1.0), cx);
            })
            .child(div().absolute().inset_0().child(self.after))
            .children(overlay.into_iter().flatten())
            .child(
                canvas(
                    move |bounds, window, cx| {
                        if split.read(cx).bounds != bounds {
                            split.update(cx, |split, cx| {
                                split.bounds = bounds;
                                cx.notify();
                            });
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            )
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, point, px, size};

    use super::share_at;

    #[test]
    fn the_divider_travels_inside_the_handle_inset() {
        let bounds = Bounds::new(point(px(100.0), px(0.0)), size(px(240.0), px(100.0)));
        let at = |x: f32| share_at(point(px(x), px(50.0)), bounds, px(20.0));
        assert_eq!(at(120.0), Some(0.0));
        assert_eq!(at(220.0), Some(0.5));
        assert_eq!(at(320.0), Some(1.0));
        assert_eq!(at(90.0), Some(0.0), "left of the box holds at the start");
        assert_eq!(at(400.0), Some(1.0));
        let narrow = Bounds::new(point(px(0.0), px(0.0)), size(px(30.0), px(30.0)));
        assert_eq!(share_at(point(px(10.0), px(10.0)), narrow, px(20.0)), None);
    }
}
