use std::{collections::HashMap, rc::Rc};

use gpui::{
    AnyElement, App, ElementId, Entity, IntoElement, ParentElement, Pixels, RenderOnce,
    SharedString, Styled, Window, canvas, div,
};
use web_time::Instant;

use super::{BASE, duration, ease_in_out_cubic, ease_out_cubic};
use crate::theme::{ActiveTheme, Elevation};

/// Where a row sits, where it glides from, and since when.
#[derive(Clone, Copy)]
struct Place {
    height: Pixels,
    top: Pixels,
    from: Pixels,
    since: Instant,
    turn: usize,
}

impl Place {
    fn now(&self, length: std::time::Duration) -> Pixels {
        let share =
            (self.since.elapsed().as_secs_f32() / length.as_secs_f32().max(f32::EPSILON)).min(1.0);
        self.from + (self.top - self.from) * ease_in_out_cubic(share)
    }
}

/// Measures a row's height as it draws; a change asks for the next frame.
fn measure<T: 'static>(
    state: &Entity<HashMap<SharedString, T>>,
    key: SharedString,
    height: impl Fn(&mut T) -> &mut Pixels + 'static,
) -> impl IntoElement {
    let state = state.clone();
    canvas(
        move |bounds, window, cx| {
            let changed = state.update(cx, |rows, _| {
                let Some(row) = rows.get_mut(&key) else {
                    return false;
                };
                let slot = height(row);
                let changed = *slot != bounds.size.height;
                *slot = bounds.size.height;
                changed
            });
            if changed {
                state.update(cx, |_, cx| cx.notify());
                window.request_animation_frame();
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// A column whose rows glide to their new places when the order changes: first, last, invert, play. Rows are keyed so each is followed.
#[derive(IntoElement)]
pub struct Flip {
    id: ElementId,
    rows: Vec<(SharedString, AnyElement)>,
    lifted: Option<(SharedString, Pixels)>,
}

impl Flip {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            rows: Vec::new(),
            lifted: None,
        }
    }

    /// A row in display order; its key stays with it as the order changes.
    pub fn row(mut self, key: impl Into<SharedString>, row: impl IntoElement) -> Self {
        self.rows.push((key.into(), row.into_any_element()));
        self
    }

    /// Draws one row raised at `top`, free of its slot, as a drag holds it.
    pub(super) fn lifted(mut self, key: SharedString, top: Pixels) -> Self {
        self.lifted = Some((key, top));
        self
    }
}

impl RenderOnce for Flip {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state((self.id.clone(), "places"), cx, |_, _| {
            HashMap::<SharedString, Place>::new()
        });
        let length = duration(BASE, cx);
        let keys: Vec<SharedString> = self.rows.iter().map(|(key, _)| key.clone()).collect();
        let places = state.update(cx, |places, _| {
            places.retain(|key, _| keys.contains(key));
            let measured = keys.iter().all(|key| {
                places
                    .get(key)
                    .is_some_and(|place| place.height > Pixels::ZERO)
            });
            let mut top = Pixels::ZERO;
            for key in &keys {
                let place = places.entry(key.clone()).or_insert(Place {
                    height: Pixels::ZERO,
                    top,
                    from: top,
                    since: Instant::now(),
                    turn: 0,
                });
                if place.top != top {
                    place.from = if measured { place.now(length) } else { top };
                    place.top = top;
                    place.since = Instant::now();
                    place.turn += 1;
                }
                top += place.height;
            }
            (places.clone(), top)
        });
        let (places, total) = places;
        if places
            .values()
            .any(|place| place.turn > 0 && place.since.elapsed() < length)
        {
            window.request_animation_frame();
        }
        let shadow = cx.theme().elevation(Elevation::Floating);
        let rows = self.rows.into_iter().map(|(key, row)| {
            let place = places[&key];
            let slot = div()
                .absolute()
                .left_0()
                .right_0()
                .child(row)
                .child(measure(&state, key.clone(), |place: &mut Place| {
                    &mut place.height
                }));
            match &self.lifted {
                Some((lifted, top)) if *lifted == key => (
                    true,
                    slot.top(*top).shadow(shadow.clone()).into_any_element(),
                ),
                _ => (false, slot.top(place.now(length)).into_any_element()),
            }
        });
        let (mut rest, mut raised): (Vec<_>, Vec<_>) = (Vec::new(), Vec::new());
        for (lifted, row) in rows {
            if lifted {
                raised.push(row)
            } else {
                rest.push(row)
            }
        }
        rest.append(&mut raised);
        div().relative().h(total).children(rest)
    }
}

/// Whether a row shows, how tall it is, and when it last turned.
#[derive(Clone, Copy)]
struct Presence {
    height: Pixels,
    shown: bool,
    since: Instant,
    turn: usize,
    told: bool,
}

type OnGone = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// A hidden row is gone once its fold ends, or at once if it was never shown here.
fn gone(presence: &Presence, length: std::time::Duration) -> bool {
    presence.turn == 0 || presence.since.elapsed() >= length
}

/// Rows that fold open and fade in when they arrive, and fold away when the owner marks them gone. `on_gone` says when a row may be dropped. Rows there at first stay still.
#[derive(IntoElement)]
pub struct AnimatePresence {
    id: ElementId,
    rows: Vec<(SharedString, bool, AnyElement)>,
    on_gone: Option<OnGone>,
}

impl AnimatePresence {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            rows: Vec::new(),
            on_gone: None,
        }
    }

    /// A row, and whether it should show; keep passing it while it leaves.
    pub fn row(mut self, key: impl Into<SharedString>, shown: bool, row: impl IntoElement) -> Self {
        self.rows.push((key.into(), shown, row.into_any_element()));
        self
    }

    pub fn on_gone(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_gone = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for AnimatePresence {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let first = window.use_keyed_state((self.id.clone(), "first"), cx, |_, _| true);
        let state = window.use_keyed_state((self.id.clone(), "rows"), cx, |_, _| {
            HashMap::<SharedString, Presence>::new()
        });
        let still = *first.read(cx);
        first.update(cx, |first, _| *first = false);
        let length = duration(BASE, cx);
        let on_gone = self.on_gone;
        let mut rows = Vec::new();
        for (key, shown, row) in self.rows {
            let presence = state.update(cx, |rows, _| {
                let presence = rows.entry(key.clone()).or_insert(Presence {
                    height: Pixels::ZERO,
                    shown,
                    since: Instant::now(),
                    turn: usize::from(!still && shown),
                    told: false,
                });
                if presence.shown != shown {
                    presence.shown = shown;
                    presence.since = Instant::now();
                    presence.turn += 1;
                    presence.told = false;
                }
                let copy = *presence;
                if !shown && gone(presence, length) {
                    presence.told = true;
                }
                copy
            });
            let settled = presence.since.elapsed() >= length;
            if !shown && gone(&presence, length) {
                if let Some(on_gone) = on_gone.clone().filter(|_| !presence.told) {
                    let key = key.clone();
                    window.defer(cx, move |window, cx| on_gone(&key, window, cx));
                }
                continue;
            }
            if !settled && presence.turn > 0 {
                window.request_animation_frame();
            }
            let frame = div().relative().overflow_hidden().child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .child(row)
                    .child(measure(&state, key.clone(), |row: &mut Presence| {
                        &mut row.height
                    })),
            );
            let share = (presence.since.elapsed().as_secs_f32()
                / length.as_secs_f32().max(f32::EPSILON))
            .min(1.0);
            let open = if presence.turn == 0 {
                1.0
            } else {
                ease_out_cubic(if shown { share } else { 1.0 - share })
            };
            rows.push(
                frame
                    .h(presence.height * open)
                    .opacity(open)
                    .into_any_element(),
            );
        }
        div().flex().flex_col().children(rows)
    }
}
