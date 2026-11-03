use std::{cell::RefCell, panic::Location};

use gpui::{
    AnyElement, App, AvailableSpace, Bounds, Element, ElementId, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Pixels, Point, Position, Style, Window, size,
};

/// What was raised while a layer prepaints its content, each by its owner's id, with where it sits and its priority.
type Queue = Vec<(ElementId, AnyElement, Point<Pixels>, usize)>;

thread_local! {
    static LAYERS: RefCell<Vec<Queue>> = const { RefCell::new(Vec::new()) };
}

fn nested() -> bool {
    LAYERS.with_borrow(|layers| !layers.is_empty())
}

/// Draws `child` over the page, last inside a raise.
pub fn raise(id: impl Into<ElementId>, child: impl IntoElement) -> Raised {
    let layer = Layer {
        child: child.into_any_element(),
    };
    Raised {
        id: id.into(),
        child: Some(layer.into_any_element()),
        priority: 0,
        nested: false,
    }
}

/// An element drawn over the page; see [`raise`].
pub struct Raised {
    id: ElementId,
    child: Option<AnyElement>,
    priority: usize,
    nested: bool,
}

impl Raised {
    /// A higher priority draws later, over the lower.
    pub fn with_priority(mut self, priority: usize) -> Self {
        self.priority = priority;
        self
    }
}

impl IntoElement for Raised {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Raised {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        self.nested = nested();
        if self.nested {
            let style = Style {
                position: Position::Absolute,
                ..Style::default()
            };
            return (window.request_layout(style, [], cx), ());
        }
        let child = self.child.as_mut().expect("a raised element lays out once");
        (child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        _: &mut App,
    ) {
        let child = self.child.take().expect("a raised element prepaints once");
        if !self.nested {
            window.defer_draw(child, window.element_offset(), self.priority, None);
            return;
        }
        LAYERS.with_borrow_mut(|layers| {
            let queue = layers
                .last_mut()
                .expect("a nested element prepaints in its layer");
            queue.push((self.id.clone(), child, bounds.origin, self.priority));
        });
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        _: &mut Window,
        _: &mut App,
    ) {
    }
}

/// What a raised element draws: its content, then what was raised inside it, each laid out on its own and the higher priority last.
struct Layer {
    child: AnyElement,
}

impl IntoElement for Layer {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

/// Runs `f` with a fresh queue on the stack, and hands back what was raised meanwhile.
fn collect(f: impl FnOnce()) -> Queue {
    LAYERS.with_borrow_mut(|layers| layers.push(Vec::new()));
    f();
    LAYERS
        .with_borrow_mut(|layers| layers.pop())
        .expect("a layer pops what it pushed")
}

impl Element for Layer {
    type RequestLayoutState = ();
    type PrepaintState = Vec<(ElementId, AnyElement)>;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut layout = None;
        collect(|| layout = Some(self.child.request_layout(window, cx)));
        (layout.expect("the child laid out"), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Vec<(ElementId, AnyElement)> {
        let mut raised = collect(|| {
            self.child.prepaint(window, cx);
        });
        raised.sort_by_key(|(_, _, _, priority)| *priority);
        let viewport = window.viewport_size();
        let space = size(
            AvailableSpace::Definite(viewport.width),
            AvailableSpace::Definite(viewport.height),
        );
        raised
            .into_iter()
            .map(|(id, mut element, at, _)| {
                window.with_element_namespace(id.clone(), |window| {
                    element.prepaint_as_root(at, space, window, cx)
                });
                (id, element)
            })
            .collect()
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        raised: &mut Vec<(ElementId, AnyElement)>,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
        for (id, element) in raised.iter_mut() {
            window.with_element_namespace(id.clone(), |window| element.paint(window, cx));
        }
    }
}
