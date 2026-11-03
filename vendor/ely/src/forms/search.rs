use std::rc::Rc;

use gpui::{
    App, Bounds, ElementId, Entity, IntoElement, ParentElement, Pixels, RenderOnce, SharedString,
    Styled, Window, canvas, div, prelude::*,
};

use super::{
    Input, TextInput,
    options::{Choice, Popup},
};
use crate::{
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize},
};

type OnPick = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// A search field: a magnifier, a clear button, recent searches while empty.
#[derive(IntoElement)]
pub struct SearchInput {
    id: ElementId,
    state: Entity<TextInput>,
    history: Vec<SharedString>,
    size: ControlSize,
    on_pick: Option<OnPick>,
}

impl SearchInput {
    pub fn new(id: impl Into<ElementId>, state: &Entity<TextInput>) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            history: Vec::new(),
            size: ControlSize::default(),
            on_pick: None,
        }
    }

    /// Recent searches, newest first; shown while the field is empty.
    pub fn history(mut self, history: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.history = history.into_iter().map(Into::into).collect();
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Runs after a recent search fills the field.
    pub fn on_pick(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SearchInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let frame = window.use_keyed_state(self.id.clone(), cx, |_, _| Bounds::<Pixels>::default());
        let input = self.state.read(cx);
        let open = input.focus().is_focused(window) && input.is_empty() && !self.history.is_empty();
        let anchor = *frame.read(cx);
        let subtle = cx.theme().colors.fg_subtle;
        let rows: Vec<Choice> = self
            .history
            .iter()
            .map(|query| Choice::new(query.clone(), query.clone()))
            .collect();
        let (state, history, on_pick) = (self.state.clone(), self.history, self.on_pick);
        let pick = Rc::new(move |ix: usize, window: &mut Window, cx: &mut App| {
            let query = history[ix].clone();
            log::info!("search input: recent {query}");
            state.update(cx, |input, cx| input.set_text(query.to_string(), cx));
            if let Some(on_pick) = &on_pick {
                on_pick(&query, window, cx);
            }
        });
        div()
            .relative()
            .child(
                Input::new(&self.state)
                    .size(self.size)
                    .clearable()
                    .prefix(Icon::new(IconName::Search).size(IconSize::Sm).color(subtle)),
            )
            .child(
                canvas(
                    move |bounds, _, cx| {
                        if *frame.read(cx) != bounds {
                            frame.update(cx, |frame, _| *frame = bounds);
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .when(open, |field| {
                field.child(
                    Popup {
                        id: (self.id.clone(), "recent").into(),
                        anchor,
                        rows: &rows,
                        highlighted: None,
                        checked: None,
                        pick,
                        dismiss: None,
                        scroll: None,
                        reveal: None,
                    }
                    .render(window, cx),
                )
            })
    }
}
