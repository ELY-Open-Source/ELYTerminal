use gpui::{App, ElementId, Entity, Window};

/// Local state that restarts from its seed whenever the owner passes a new one.
pub(crate) struct Seeded<T> {
    seed: T,
    pub value: T,
}

pub(crate) fn use_seeded<T: Clone + PartialEq + 'static>(
    id: impl Into<ElementId>,
    seed: T,
    window: &mut Window,
    cx: &mut App,
) -> Entity<Seeded<T>> {
    let state = window.use_keyed_state(id, cx, |_, _| Seeded {
        seed: seed.clone(),
        value: seed.clone(),
    });
    if state.read(cx).seed != seed {
        state.update(cx, |state, _| {
            state.value = seed.clone();
            state.seed = seed;
        });
    }
    state
}
