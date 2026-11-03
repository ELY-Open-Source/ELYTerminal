use std::rc::Rc;

use gpui::{
    AnyElement, App, Div, InteractiveElement, IntoElement as _, MouseButton, MouseDownEvent,
    ParentElement as _, TextLayout, Window, div,
};

use super::{Draw, table::CellAt};
use crate::documents::tree::Inline;

/// Where a press fell, as the drawing knows it.
pub(crate) enum Place {
    /// A drawn byte, read back through its text's sources.
    Text(Rc<Inline>, usize),
    /// A source byte.
    At(usize),
    /// Drawn byte `at` of `text`, its lines written from `from`.
    Lines {
        text: Rc<str>,
        from: usize,
        at: usize,
    },
    /// A table cell, pressed past its text.
    Cell(CellAt),
    /// Somewhere the block holds no byte for.
    Block,
}

/// A press on drawn content.
pub(crate) struct Pointed {
    pub place: Place,
    pub link: Option<String>,
    pub image: Option<String>,
    /// The code's text, pressed in a code block.
    pub code: Option<Rc<str>>,
    pub table: bool,
}

impl Pointed {
    /// A press past any drawn text.
    pub(crate) fn block() -> Self {
        Self::at(Place::Block)
    }

    fn at(place: Place) -> Self {
        Self {
            place,
            link: None,
            image: None,
            code: None,
            table: false,
        }
    }
}

/// Takes a press's button and what it fell on.
pub(crate) type OnPoint = Rc<dyn Fn(Pointed, MouseButton, &mut Window, &mut App)>;

/// `element` hands its left and right presses to `on_point`.
fn pointable(
    element: AnyElement,
    on_point: &Option<OnPoint>,
    table: bool,
    point: impl Fn(&MouseDownEvent) -> Pointed + 'static,
) -> AnyElement {
    match on_point {
        Some(_) => pressed(div().child(element), on_point, table, point).into_any_element(),
        None => element,
    }
}

/// `element` itself takes the presses `pointable` hands on.
fn pressed<E: InteractiveElement>(
    element: E,
    on_point: &Option<OnPoint>,
    table: bool,
    point: impl Fn(&MouseDownEvent) -> Pointed + 'static,
) -> E {
    let Some(on_point) = on_point.clone() else {
        return element;
    };
    let point = Rc::new(point);
    let press = move |button: MouseButton| {
        let (on_point, point) = (on_point.clone(), point.clone());
        move |event: &MouseDownEvent, window: &mut Window, cx: &mut App| {
            // Cmd or Ctrl leaves the press to a link.
            if button == MouseButton::Left && event.modifiers.secondary() {
                return;
            }
            window.prevent_default();
            cx.stop_propagation();
            let mut pointed = point(event);
            pointed.table |= table;
            on_point(pointed, button, window, cx);
        }
    };
    element
        .on_mouse_down(MouseButton::Left, press(MouseButton::Left))
        .on_mouse_down(MouseButton::Right, press(MouseButton::Right))
}

/// Drawn `text`, its lines written from source byte `from`.
pub(crate) fn point_lines(
    element: AnyElement,
    layout: TextLayout,
    text: &str,
    from: usize,
    on_point: &Option<OnPoint>,
    (table, code): (bool, bool),
) -> AnyElement {
    let text: Rc<str> = Rc::from(text);
    pointable(element, on_point, table, move |event| {
        let at = layout
            .index_for_position(event.position)
            .unwrap_or_else(|near| near);
        Pointed {
            code: code.then(|| text.clone()),
            ..Pointed::at(Place::Lines {
                text: text.clone(),
                from,
                at,
            })
        }
    })
}

impl Draw<'_> {
    fn pointable(
        &self,
        element: AnyElement,
        point: impl Fn(&MouseDownEvent) -> Pointed + 'static,
    ) -> AnyElement {
        pointable(element, self.hooks.on_point, self.table.get(), point)
    }

    /// A table cell taking its presses, past its text too.
    pub(super) fn pressed_cell(&self, cell: Div, at: CellAt) -> Div {
        pressed(cell, self.hooks.on_point, true, move |_| {
            Pointed::at(Place::Cell(at))
        })
    }

    /// Drawn `inline`; a press finds its byte through `at`.
    pub(super) fn point_text(
        &self,
        element: AnyElement,
        at: impl Fn(gpui::Point<gpui::Pixels>) -> usize + 'static,
        inline: &Inline,
    ) -> AnyElement {
        if self.hooks.on_point.is_none() {
            return element;
        }
        let inline = Rc::new(inline.clone());
        self.pointable(element, move |event| {
            let at = at(event.position);
            let link = inline.links.iter().find(|(range, _)| range.contains(&at));
            Pointed {
                link: link.map(|(_, url)| url.clone()),
                ..Pointed::at(Place::Text(inline.clone(), at))
            }
        })
    }

    /// A code block; its lines start at source byte `from`.
    pub(super) fn point_code(
        &self,
        element: AnyElement,
        layout: Option<TextLayout>,
        text: &str,
        from: usize,
    ) -> AnyElement {
        let table = self.table.get();
        match layout {
            Some(layout) => point_lines(
                element,
                layout,
                text,
                from,
                self.hooks.on_point,
                (table, true),
            ),
            None => {
                let text: Rc<str> = Rc::from(text);
                self.pointable(element, move |_| Pointed {
                    code: Some(text.clone()),
                    ..Pointed::block()
                })
            }
        }
    }

    /// A picture written at source byte `at`.
    pub(super) fn point_image(
        &self,
        element: AnyElement,
        at: usize,
        url: &str,
        link: &Option<String>,
    ) -> AnyElement {
        let (url, link) = (url.to_string(), link.clone());
        self.pointable(element, move |_| Pointed {
            image: Some(url.clone()),
            link: link.clone(),
            ..Pointed::at(Place::At(at))
        })
    }
}
