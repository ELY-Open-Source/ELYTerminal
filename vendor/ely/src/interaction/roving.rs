use std::rc::Rc;

use gpui::{
    AnyElement, App, Axis, Div, ElementId, FocusHandle, InteractiveElement, IntoElement,
    MouseButton, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement,
    StyleRefinement, Styled, Window, div, prelude::*,
};

use crate::{
    primitives::{FocusRing, tab_stop},
    theme::{ActiveTheme, Radius},
};

type OnKey = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// Where an arrow key or Home and End take focus among `count` items from `at`, along `axis`; the ends hold.
fn roved(key: &str, axis: Axis, at: usize, count: usize) -> Option<usize> {
    let (back, ahead) = match axis {
        Axis::Horizontal => ("left", "right"),
        Axis::Vertical => ("up", "down"),
    };
    match key {
        key if key == back => at.checked_sub(1),
        key if key == ahead => (at + 1 < count).then_some(at + 1),
        "home" => Some(0),
        "end" => count.checked_sub(1),
        _ => None,
    }
}

/// Items in a row or a column behind one Tab stop: the arrows along the axis move focus among them, Home and End go to the ends, and Enter or Space presses the one focused. The stop stays on the item last focused or pressed; a press, as a button's, leaves focus where it was. The owner lays them out through the box's style.
#[derive(IntoElement)]
pub struct RovingFocus {
    id: ElementId,
    axis: Axis,
    base: Div,
    items: Vec<(SharedString, AnyElement)>,
    on_press: OnKey,
}

impl RovingFocus {
    pub fn new(
        id: impl Into<ElementId>,
        axis: Axis,
        on_press: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            axis,
            base: div(),
            items: Vec::new(),
            on_press: Rc::new(on_press),
        }
    }

    pub fn item(mut self, key: impl Into<SharedString>, content: impl IntoElement) -> Self {
        self.items.push((key.into(), content.into_any_element()));
        self
    }
}

impl Styled for RovingFocus {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for RovingFocus {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let on_press = self.on_press;
        assert!(!self.items.is_empty(), "roving focus {id:?} has no item");
        let stop = window.use_keyed_state((id.clone(), "stop"), cx, |_, _| 0usize);
        let count = self.items.len();
        let handles: Vec<FocusHandle> = (0..count)
            .map(|ix| tab_stop((id.clone(), format!("item-{ix}")).into(), false, window, cx))
            .collect();
        let focused = handles.iter().position(|handle| handle.is_focused(window));
        let at = focused.unwrap_or(*stop.read(cx)).min(count - 1);
        if at != *stop.read(cx) {
            stop.update(cx, |stop, _| *stop = at);
        }
        let handles: Vec<FocusHandle> = handles
            .into_iter()
            .enumerate()
            .map(|(ix, handle)| handle.tab_stop(ix == at))
            .collect();
        let theme = cx.theme();
        let radius = theme.radius(Radius::Md);
        let items: Vec<_> = self
            .items
            .into_iter()
            .zip(handles.iter().cloned())
            .enumerate()
            .map(|(ix, ((key, content), handle))| {
                let (press, held) = (on_press.clone(), stop.clone());
                div()
                    .id((id.clone(), format!("item-{ix}")))
                    .track_focus(&handle)
                    .rounded(radius)
                    .border_1()
                    .border_color(gpui::transparent_black())
                    .focus_ring(cx)
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        window.prevent_default();
                        held.update(cx, |stop, _| *stop = ix);
                    })
                    .on_click(move |_, window, cx| {
                        log::info!("roving focus: press {key}");
                        press(&key, window, cx)
                    })
                    .child(content)
            })
            .collect();
        let (axis, moved) = (self.axis, stop);
        self.base
            .id(id)
            .flex()
            .when(axis == Axis::Vertical, |group| group.flex_col())
            .on_key_down(move |event, window, cx| {
                if event.keystroke.modifiers.modified() {
                    return;
                }
                let Some(to) = roved(&event.keystroke.key, axis, at, count) else {
                    return;
                };
                cx.stop_propagation();
                moved.update(cx, |stop, cx| {
                    *stop = to;
                    cx.notify();
                });
                window.focus(&handles[to], cx);
            })
            .children(items)
    }
}

#[cfg(test)]
mod tests {
    use gpui::Axis;

    use super::roved;

    #[test]
    fn arrows_move_along_the_axis_and_hold_at_the_ends() {
        assert_eq!(roved("right", Axis::Horizontal, 0, 3), Some(1));
        assert_eq!(
            roved("right", Axis::Horizontal, 2, 3),
            None,
            "the end holds"
        );
        assert_eq!(roved("left", Axis::Horizontal, 0, 3), None);
        assert_eq!(
            roved("down", Axis::Horizontal, 0, 3),
            None,
            "across the axis does nothing"
        );
        assert_eq!(roved("down", Axis::Vertical, 0, 3), Some(1));
        assert_eq!(roved("end", Axis::Vertical, 0, 3), Some(2));
        assert_eq!(roved("home", Axis::Horizontal, 2, 3), Some(0));
    }
}
