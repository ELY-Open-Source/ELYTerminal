use std::rc::Rc;

use gpui::{
    Animation, AnimationExt, App, ElementId, FontWeight, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::*,
};

use crate::{
    motion::{self, Axis, Marker, glide, measure_item, measure_origin, slide},
    theme::{ActiveTheme, TextSize},
};

type OnKey = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// A document's headings, each level stepping in, beside a rail whose marker glides to the section in view. A press asks the owner to go there; the owner says which is current.
#[derive(IntoElement)]
pub struct Outline {
    id: ElementId,
    items: Vec<(SharedString, SharedString, usize)>,
    current: Option<SharedString>,
    on_select: Option<OnKey>,
}

impl Outline {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
            current: None,
            on_select: None,
        }
    }

    /// A heading under `key`, its title, and its level, counted from one.
    pub fn item(
        mut self,
        key: impl Into<SharedString>,
        title: impl Into<SharedString>,
        level: usize,
    ) -> Self {
        assert!(level >= 1, "heading levels start at one");
        self.items.push((key.into(), title.into(), level));
        self
    }

    pub fn current(mut self, key: impl Into<SharedString>) -> Self {
        self.current = Some(key.into());
        self
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Outline {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let keys: Vec<SharedString> = self.items.iter().map(|(key, _, _)| key.clone()).collect();
        let sliding = self
            .current
            .as_ref()
            .map(|current| slide((self.id.clone(), "slide"), &keys, current, window, cx));
        let duration = motion::duration(motion::SLOW, cx);
        let theme = cx.theme();
        let colors = &theme.colors;
        let marker = sliding
            .as_ref()
            .and_then(|(_, marker)| marker.as_ref())
            .map(
                |Marker {
                     from,
                     to,
                     generation,
                 }| {
                    let (from, to) = (*from, *to);
                    div()
                        .absolute()
                        .left_0()
                        .border_l_2()
                        .border_color(colors.accent)
                        .with_animation(
                            (self.id.clone(), format!("marker-{generation}")),
                            Animation::new(duration),
                            move |bar, t| {
                                let (top, height) = glide(from, to, t);
                                bar.top(top).h(height)
                            },
                        )
                },
            );
        let on_select = self.on_select;
        let rows = self
            .items
            .into_iter()
            .enumerate()
            .map(|(ix, (key, title, level))| {
                let here = self.current.as_ref() == Some(&key);
                let (select, id) = (on_select.clone(), self.id.clone());
                div()
                    .id((self.id.clone(), format!("item-{key}")))
                    .relative()
                    .py_1()
                    .pl_3()
                    .cursor_pointer()
                    .text_color(if here { colors.fg } else { colors.fg_muted })
                    .when(here, |row| row.font_weight(FontWeight::MEDIUM))
                    .hover(|style| style.text_color(colors.fg))
                    .on_click(move |_, window, cx| {
                        log::info!("outline {id:?}: {key}");
                        if let Some(select) = &select {
                            select(&key, window, cx);
                        }
                    })
                    .child(
                        div()
                            .ml(theme.tree_indent() * (level - 1) as f32)
                            .child(title),
                    )
                    .children(
                        sliding
                            .as_ref()
                            .map(|(state, _)| measure_item(state.clone(), ix, Axis::Vertical)),
                    )
            });
        div()
            .relative()
            .text_size(theme.text_size(TextSize::Sm))
            .children(
                sliding
                    .as_ref()
                    .map(|(state, _)| measure_origin(state.clone(), Axis::Vertical)),
            )
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .bottom_0()
                    .border_l_1()
                    .border_color(colors.border),
            )
            .children(marker)
            .children(rows)
    }
}
