use std::{cell::Cell, rc::Rc};

use gpui::{
    AnyElement, App, DragMoveEvent, ElementId, EmptyView, EntityId, FontWeight, InteractiveElement,
    IntoElement, ParentElement, Pixels, Point, RenderOnce, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, anchored, div, point, prelude::*,
};
use smallvec::SmallVec;

use super::seeded::use_seeded;
use crate::{
    primitives::raise,
    theme::{ActiveTheme, ControlSize, Elevation, Radius, TextSize},
};

struct Carry {
    owner: EntityId,
    start: Point<Pixels>,
    anchor: Rc<Cell<Option<Point<Pixels>>>>,
}

/// Keeps `origin` so that at least `keep` of the panel stays in the window.
fn clamp_to_window(
    origin: Point<Pixels>,
    keep: Pixels,
    window: gpui::Size<Pixels>,
) -> Point<Pixels> {
    point(
        origin.x.clamp(keep - window.width, window.width - keep),
        origin.y.clamp(Pixels::ZERO, window.height - keep),
    )
}

type OnMove = Rc<dyn Fn(&Point<Pixels>, &mut Window, &mut App)>;

/// Panel that floats above the page; drag its title bar to move it.
#[derive(IntoElement)]
pub struct FloatingPanel {
    id: ElementId,
    title: SharedString,
    start: Point<Pixels>,
    base: gpui::Div,
    on_move: Option<OnMove>,
    body: SmallVec<[AnyElement; 2]>,
}

impl FloatingPanel {
    /// Top-left corner in window coordinates. A new value replaces a dragged one.
    pub fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        start: Point<Pixels>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            start,
            base: div(),
            on_move: None,
            body: SmallVec::new(),
        }
    }

    /// Reports each new top-left corner while dragging.
    pub fn on_move(
        mut self,
        handler: impl Fn(&Point<Pixels>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_move = Some(Rc::new(handler));
        self
    }
}

impl Styled for FloatingPanel {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for FloatingPanel {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl RenderOnce for FloatingPanel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let origin = use_seeded(self.id.clone(), self.start, window, cx);
        let owner = origin.entity_id();
        let theme = cx.theme();
        let keep = theme
            .control_height(ControlSize::Lg)
            .to_pixels(window.rem_size());
        let viewport = window.viewport_size();
        let at = clamp_to_window(origin.read(cx).value, keep, viewport);
        let on_move = self.on_move;
        let carry = Carry {
            owner,
            start: at,
            anchor: Rc::new(Cell::new(None)),
        };
        let panel = self
            .base
            .id(self.id.clone())
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.overlay)
            .shadow(theme.elevation(Elevation::Floating))
            .on_drag_move(move |event: &DragMoveEvent<Carry>, window, cx| {
                let carry = event.drag(cx);
                if carry.owner != owner {
                    return;
                }
                let pointer = event.event.position;
                let anchor = match carry.anchor.get() {
                    Some(anchor) => anchor,
                    None => {
                        carry.anchor.set(Some(pointer));
                        pointer
                    }
                };
                let next = clamp_to_window(carry.start + (pointer - anchor), keep, viewport);
                if let Some(report) = &on_move {
                    report(&next, window, cx);
                }
                origin.update(cx, |origin, cx| {
                    origin.value = next;
                    cx.notify();
                });
            })
            .child(
                div()
                    .id("floating-title")
                    .flex()
                    .items_center()
                    .h(theme.control_height(ControlSize::Md))
                    .px_3()
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .cursor_grab()
                    .text_size(theme.text_size(TextSize::Sm))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.colors.fg_muted)
                    .on_drag(carry, |_, _, _, cx| cx.new(|_| EmptyView))
                    .child(self.title),
            )
            .child(div().p_3().children(self.body));
        raise((self.id, "raised"), anchored().position(at).child(panel))
    }
}

#[cfg(test)]
mod tests {
    use gpui::{point, px, size};

    use super::clamp_to_window;

    #[test]
    fn panel_stays_reachable() {
        let window = size(px(800.0), px(600.0));
        let inside = clamp_to_window(point(px(100.0), px(100.0)), px(32.0), window);
        assert_eq!(inside, point(px(100.0), px(100.0)));
        let far = clamp_to_window(point(px(2000.0), px(-50.0)), px(32.0), window);
        assert_eq!(far, point(px(768.0), px(0.0)));
    }
}
