use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, Window, div, img, prelude::*,
};

use super::viewer::DocPage;
use crate::{
    documents::blocks::source,
    forms::Pick,
    theme::{ActiveTheme, Radius, TextSize},
    typography::tabular,
};

/// Share of the label column a thumbnail spans.
const THUMB: f32 = 0.75;

/// Pages as small pictures down a column, each numbered, the current one outlined; a press goes to a page.
#[derive(IntoElement)]
pub struct PageThumbnailList {
    id: ElementId,
    pages: Rc<Vec<DocPage>>,
    current: usize,
    on_pick: Option<Pick>,
}

impl PageThumbnailList {
    pub fn new(
        id: impl Into<ElementId>,
        pages: impl Into<Rc<Vec<DocPage>>>,
        current: usize,
    ) -> Self {
        Self {
            id: id.into(),
            pages: pages.into(),
            current,
            on_pick: None,
        }
    }

    pub fn on_pick(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for PageThumbnailList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let width = theme.label_width() * THUMB;
        div()
            .id(self.id.clone())
            .h_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .items_center()
            .gap_3()
            .p_3()
            .children(self.pages.iter().enumerate().map(|(ix, page)| {
                let current = ix == self.current;
                let pick = self.on_pick.clone();
                div()
                    .id((self.id.clone(), format!("thumb-{ix}")))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .when_some(pick, |thumb, pick| {
                        thumb.on_click(move |_, window, cx| {
                            log::info!("page thumbnails: page {ix}");
                            pick(ix, window, cx)
                        })
                    })
                    .child(
                        div()
                            .w(width)
                            .h(width * (page.height / page.width))
                            .rounded(theme.radius(Radius::Sm))
                            .border_1()
                            .border_color(if current { colors.focus } else { colors.border })
                            .bg(colors.paper)
                            .child(
                                img(source(&page.source))
                                    .id((self.id.clone(), format!("thumb-picture-{ix}")))
                                    .size_full()
                                    .rounded(theme.radius(Radius::Sm)),
                            ),
                    )
                    .child(
                        tabular(div().text_size(theme.text_size(TextSize::Xs)))
                            .text_color(if current { colors.fg } else { colors.fg_subtle })
                            .child(format!("{}", ix + 1)),
                    )
            }))
    }
}
