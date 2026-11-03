use std::rc::Rc;

use gpui::{
    AnyElement, App, Div, ElementId, IntoElement, ListAlignment, ListState, ParentElement,
    RenderOnce, StyleRefinement, Styled, Window, div, list,
};
use smallvec::SmallVec;

use crate::{motion::Spinner, primitives::IntersectionObserver, theme::ActiveTheme};

type Row = Rc<dyn Fn(usize, &mut Window, &mut App) -> AnyElement>;

/// A long list that builds only the rows in view, each as tall as it needs. `row(ix)` builds row `ix`. Rows added or dropped at the end keep the scroll. Give it a height.
#[derive(IntoElement)]
pub struct VirtualList {
    id: ElementId,
    base: Div,
    count: usize,
    row: Row,
}

impl VirtualList {
    pub fn new(
        id: impl Into<ElementId>,
        count: usize,
        row: impl Fn(usize, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            base: div(),
            count,
            row: Rc::new(row),
        }
    }
}

impl Styled for VirtualList {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for VirtualList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (count, overdraw) = (
            self.count,
            cx.theme().list_overdraw().to_pixels(window.rem_size()),
        );
        let state = window
            .use_keyed_state(self.id.clone(), cx, |_, _| {
                ListState::new(count, ListAlignment::Top, overdraw)
            })
            .read(cx)
            .clone();
        let known = state.item_count();
        if known != count {
            log::debug!("virtual list {:?}: {known} rows to {count}", self.id);
            if count > known {
                state.splice(known..known, count - known);
            } else {
                state.splice(count..known, 0);
            }
        }
        let row = self.row;
        self.base
            .child(list(state, move |ix, window, cx| row(ix, window, cx)).size_full())
    }
}

type OnMore = Rc<dyn Fn(&mut Window, &mut App)>;

/// Rows that ask for more when their end comes into view: `on_more` runs once each time it arrives, a spinner shows while `loading`, and `done` ends the list quietly. Put it in a scrolling box.
#[derive(IntoElement)]
pub struct InfiniteList {
    id: ElementId,
    rows: SmallVec<[AnyElement; 8]>,
    loading: bool,
    done: bool,
    on_more: Option<OnMore>,
}

impl InfiniteList {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            rows: SmallVec::new(),
            loading: false,
            done: false,
            on_more: None,
        }
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Nothing more to load.
    pub fn done(mut self, done: bool) -> Self {
        self.done = done;
        self
    }

    pub fn on_more(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_more = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for InfiniteList {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.rows.extend(elements);
    }
}

impl RenderOnce for InfiniteList {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let count = self.rows.len();
        let end = match (self.done, self.loading) {
            (true, _) => None,
            (false, true) => Some(
                div()
                    .flex()
                    .justify_center()
                    .py_3()
                    .child(Spinner::new((self.id.clone(), "spinner")))
                    .into_any_element(),
            ),
            (false, false) => {
                let (id, on_more) = (self.id.clone(), self.on_more);
                Some(
                    IntersectionObserver::new((self.id, format!("end-{count}")), move |seen, window, cx| {
                        if seen && let Some(on_more) = &on_more {
                            log::info!("infinite list {id:?}: the end came into view after {count} rows");
                            on_more(window, cx);
                        }
                    })
                    .h_1()
                    .into_any_element(),
                )
            }
        };
        div().flex().flex_col().children(self.rows).children(end)
    }
}
