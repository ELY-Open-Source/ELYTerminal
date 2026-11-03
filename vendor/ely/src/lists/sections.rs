use gpui::{
    App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div,
};

use super::{ListItem, SelectableList};
use crate::{
    forms::OnValue,
    theme::{ActiveTheme, TextSize},
};

/// Lists under titles down a column, each a `SelectableList` keyed by its title under the id: Tab moves between them, and the chosen key is lit wherever it sits. A press or an arrow selects, Enter or a double press activates. Owners check their titles and keys.
#[derive(IntoElement)]
pub(crate) struct Sections {
    id: ElementId,
    sections: Vec<(SharedString, Vec<(SharedString, ListItem)>)>,
    counted: bool,
    selected: Option<SharedString>,
    on_select: Option<OnValue>,
    on_activate: Option<OnValue>,
}

impl Sections {
    pub(crate) fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            sections: Vec::new(),
            counted: false,
            selected: None,
            on_select: None,
            on_activate: None,
        }
    }

    pub(crate) fn section(
        mut self,
        title: SharedString,
        rows: Vec<(SharedString, ListItem)>,
    ) -> Self {
        self.sections.push((title, rows));
        self
    }

    /// Shows how many rows each section holds beside its title.
    pub(crate) fn counted(mut self) -> Self {
        self.counted = true;
        self
    }

    pub(crate) fn selected(mut self, key: Option<SharedString>) -> Self {
        self.selected = key;
        self
    }

    pub(crate) fn on_select(mut self, handler: OnValue) -> Self {
        self.on_select = Some(handler);
        self
    }

    pub(crate) fn on_activate(mut self, handler: OnValue) -> Self {
        self.on_activate = Some(handler);
        self
    }
}

impl RenderOnce for Sections {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let sections =
            self.sections.into_iter().map(|(title, rows)| {
                let count = rows.len();
                let lit = self
                    .selected
                    .clone()
                    .filter(|key| rows.iter().any(|(each, _)| each == key));
                let list = rows.into_iter().fold(
                    SelectableList::new((self.id.clone(), format!("section-{title}"))),
                    |list, (key, row)| list.row(key, row),
                );
                let (select, activate) = (self.on_select.clone(), self.on_activate.clone());
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .px_3()
                            .flex()
                            .items_baseline()
                            .gap_2()
                            .text_size(theme.text_size(TextSize::Xs))
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.fg_muted)
                                    .child(title),
                            )
                            .children(self.counted.then(|| {
                                div().text_color(colors.fg_subtle).child(count.to_string())
                            })),
                    )
                    .child(
                        list.selected(lit)
                            .on_change(move |keys, window, cx| {
                                let key = keys.first().expect("a pick names a row");
                                if let Some(select) = &select {
                                    select(key, window, cx);
                                }
                            })
                            .on_activate(move |key, window, cx| {
                                if let Some(activate) = &activate {
                                    activate(key, window, cx);
                                }
                            }),
                    )
            });
        div().flex().flex_col().gap_4().children(sections)
    }
}
