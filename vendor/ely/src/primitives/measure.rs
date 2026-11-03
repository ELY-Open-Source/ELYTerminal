use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, Div, ElementId, IntoElement, ParentElement, Pixels, RenderOnce,
    StyleRefinement, Styled, Window, canvas, div,
};

type BoundsHandler = Rc<dyn Fn(Bounds<Pixels>, &mut Window, &mut App)>;
type VisibilityHandler = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// A div that reports its bounds whenever they change, then draws once more so what the report changed shows.
#[derive(IntoElement)]
pub struct Measure {
    id: ElementId,
    base: Div,
    on_measure: BoundsHandler,
}

impl Measure {
    pub fn new(
        id: impl Into<ElementId>,
        on_measure: impl Fn(Bounds<Pixels>, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            base: div(),
            on_measure: Rc::new(on_measure),
        }
    }
}

impl Styled for Measure {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Measure {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl RenderOnce for Measure {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let last = window.use_keyed_state(self.id, cx, |_, _| None::<Bounds<Pixels>>);
        let on_measure = self.on_measure;
        self.base.relative().child(
            canvas(
                move |bounds, window, cx| {
                    if *last.read(cx) != Some(bounds) {
                        last.update(cx, |seen, _| *seen = Some(bounds));
                        on_measure(bounds, window, cx);
                        window.request_animation_frame();
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
    }
}

/// A div that reports when it enters or leaves the visible area, then draws once more so what the report changed shows.
#[derive(IntoElement)]
pub struct IntersectionObserver {
    id: ElementId,
    base: Div,
    on_change: VisibilityHandler,
}

impl IntersectionObserver {
    pub fn new(
        id: impl Into<ElementId>,
        on_change: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            base: div(),
            on_change: Rc::new(on_change),
        }
    }
}

impl Styled for IntersectionObserver {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for IntersectionObserver {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl RenderOnce for IntersectionObserver {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let last = window.use_keyed_state(self.id, cx, |_, _| None::<bool>);
        let on_change = self.on_change;
        self.base.relative().child(
            canvas(
                move |bounds, window, cx| {
                    let visible = window.content_mask().bounds.intersects(&bounds);
                    if *last.read(cx) != Some(visible) {
                        last.update(cx, |seen, _| *seen = Some(visible));
                        on_change(visible, window, cx);
                        window.request_animation_frame();
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
    }
}
