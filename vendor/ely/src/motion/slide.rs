use gpui::{
    App, Bounds, ElementId, Entity, IntoElement, Pixels, SharedString, Styled, Window, canvas,
};

use super::spring;

/// Start and length of an item along the axis a marker slides on.
pub(crate) type Span = (Pixels, Pixels);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    fn span(self, bounds: Bounds<Pixels>, origin: Pixels) -> Span {
        match self {
            Axis::Horizontal => (bounds.left() - origin, bounds.size.width),
            Axis::Vertical => (bounds.top() - origin, bounds.size.height),
        }
    }

    fn start(self, bounds: Bounds<Pixels>) -> Pixels {
        match self {
            Axis::Horizontal => bounds.left(),
            Axis::Vertical => bounds.top(),
        }
    }
}

/// Where a sliding marker's items sit, and where it last stood.
#[derive(Default)]
pub(crate) struct Slide {
    origin: Pixels,
    spans: Vec<Option<Span>>,
    from: Option<Span>,
    last: Option<SharedString>,
    generation: u64,
}

/// A marker's trip: from the last choice to this one, keyed by `generation`.
pub(crate) struct Marker {
    pub from: Span,
    pub to: Span,
    pub generation: u64,
}

/// The slide over `values` with `selected` chosen; no marker until it is measured, or if it is not there.
pub(crate) fn slide(
    id: impl Into<ElementId>,
    values: &[SharedString],
    selected: &SharedString,
    window: &mut Window,
    cx: &mut App,
) -> (Entity<Slide>, Option<Marker>) {
    let id = id.into();
    let state = window.use_keyed_state(id.clone(), cx, |_, _| Slide::default());
    if state.read(cx).spans.len() != values.len() {
        state.update(cx, |slide, _| slide.spans = vec![None; values.len()]);
    }
    let Some(chosen) = values.iter().position(|value| value == selected) else {
        log::error!("slide {id:?}: no item {selected} among {values:?}; nothing marked");
        return (state, None);
    };
    if state.read(cx).last.as_ref() != Some(selected) {
        state.update(cx, |slide, _| {
            let previous = slide
                .last
                .as_ref()
                .and_then(|last| values.iter().position(|value| value == last));
            slide.from = previous.and_then(|ix| slide.spans[ix]);
            slide.last = Some(selected.clone());
            slide.generation += 1;
        });
    }
    let marker = {
        let slide = state.read(cx);
        slide.spans[chosen].map(|to| Marker {
            from: slide.from.unwrap_or(to),
            to,
            generation: slide.generation,
        })
    };
    (state, marker)
}

/// Records item `ix`'s span; place it as an absolute child that fills the item.
pub(crate) fn measure_item(state: Entity<Slide>, ix: usize, axis: Axis) -> impl IntoElement {
    canvas(
        move |bounds: Bounds<Pixels>, window, cx| {
            let span = Some(axis.span(bounds, state.read(cx).origin));
            if state.read(cx).spans.get(ix) != Some(&span) {
                state.update(cx, |slide, cx| {
                    slide.spans[ix] = span;
                    cx.notify();
                    window.request_animation_frame();
                });
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// Records where the container starts; place it as an absolute child that fills it.
pub(crate) fn measure_origin(state: Entity<Slide>, axis: Axis) -> impl IntoElement {
    canvas(
        move |bounds: Bounds<Pixels>, window, cx| {
            let start = axis.start(bounds);
            if state.read(cx).origin != start {
                state.update(cx, |slide, cx| {
                    slide.origin = start;
                    cx.notify();
                    window.request_animation_frame();
                });
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// A marker's span at `t` of its trip, overshooting a little.
pub(crate) fn glide(from: Span, to: Span, t: f32) -> Span {
    let s = spring(t);
    let lerp = |a: Pixels, b: Pixels| a + (b - a) * s;
    (lerp(from.0, to.0), lerp(from.1, to.1))
}

#[cfg(test)]
mod tests {
    use gpui::px;

    use super::glide;

    #[test]
    fn glide_starts_at_from_and_settles_on_to() {
        let (from, to) = ((px(0.0), px(40.0)), (px(100.0), px(60.0)));
        assert_eq!(glide(from, to, 0.0), from);
        let (left, width) = glide(from, to, 1.0);
        assert!((f32::from(left) - 100.0).abs() < 0.5 && (f32::from(width) - 60.0).abs() < 0.5);
    }
}
