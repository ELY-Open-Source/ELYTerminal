use gpui::{App, ElementId, Window};

struct Changes<T> {
    last: T,
    count: usize,
}

/// How often `value` changed since `id` first rendered. Key an animation on it to replay per change.
pub(crate) fn changes<T: Clone + PartialEq + 'static>(
    id: impl Into<ElementId>,
    value: T,
    window: &mut Window,
    cx: &mut App,
) -> usize {
    let state = window.use_keyed_state(id, cx, |_, _| Changes {
        last: value.clone(),
        count: 0,
    });
    if state.read(cx).last != value {
        state.update(cx, |state, _| {
            state.last = value;
            state.count += 1;
        });
    }
    state.read(cx).count
}
