use gpui::{
    AnyElement, App, Div, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, ScrollHandle, SharedString, StatefulInteractiveElement, StyleRefinement, Styled,
    Window, div,
};

use crate::{
    layout::StickyHeader,
    theme::{ActiveTheme, TextSize},
};

/// Rows under group headers; each header pins to the top while its group scrolls past. Give it a height.
#[derive(IntoElement)]
pub struct GroupedList {
    id: ElementId,
    base: Div,
    groups: Vec<(SharedString, Vec<AnyElement>)>,
}

impl GroupedList {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            groups: Vec::new(),
        }
    }

    pub fn group(
        mut self,
        title: impl Into<SharedString>,
        rows: impl IntoIterator<Item = impl IntoElement>,
    ) -> Self {
        let rows = rows
            .into_iter()
            .map(IntoElement::into_any_element)
            .collect();
        self.groups.push((title.into(), rows));
        self
    }
}

impl Styled for GroupedList {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for GroupedList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let handle = window
            .use_keyed_state((self.id.clone(), "scroll"), cx, |_, _| ScrollHandle::new())
            .read(cx)
            .clone();
        let id = self.id.clone();
        self.base
            .id(self.id)
            .overflow_y_scroll()
            .track_scroll(&handle)
            .children(
                self.groups
                    .into_iter()
                    .enumerate()
                    .map(|(ix, (title, rows))| {
                        StickyHeader::new(
                            (id.clone(), format!("group-{ix}")),
                            &handle,
                            move |_, cx| {
                                let theme = cx.theme();
                                div()
                                    .px_3()
                                    .pt_3()
                                    .pb_1()
                                    .text_size(theme.text_size(TextSize::Xs))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.fg_muted)
                                    .child(title.clone())
                                    .into_any_element()
                            },
                        )
                        .children(rows)
                    }),
            )
    }
}
