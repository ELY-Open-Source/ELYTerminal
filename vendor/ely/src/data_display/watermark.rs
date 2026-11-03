use gpui::{
    AnyElement, App, Div, ElementId, IntoElement, ParentElement, Pixels, RenderOnce, SharedString,
    Size, StyleRefinement, Styled, Window, canvas, div, prelude::*,
};

use crate::theme::{ActiveTheme, TextSize};

/// Faint text over its content, repeated in level rows, every other row shifted half a tile. gpui draws text upright, so the tiles do not tilt. Presses pass through to the content.
#[derive(IntoElement)]
pub struct Watermark {
    id: ElementId,
    text: SharedString,
    base: Div,
}

impl Watermark {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            base: div(),
        }
    }
}

impl Styled for Watermark {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Watermark {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl RenderOnce for Watermark {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let area = window.use_keyed_state(self.id, cx, |_, _| Size::<Pixels>::default());
        let room = *area.read(cx);
        let theme = cx.theme();
        let rem = window.rem_size();
        let tile = theme.watermark_tile();
        let (wide, tall) = (tile.width.to_pixels(rem), tile.height.to_pixels(rem));
        let columns = (room.width / wide).ceil() as usize + 1;
        let rows = (room.height / tall).ceil() as usize;
        let text = self.text;
        let layer = div()
            .absolute()
            .inset_0()
            .overflow_hidden()
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(theme.colors.fg.opacity(0.07))
            .children((0..rows).map(|row| {
                div()
                    .flex()
                    .when(row % 2 == 1, |row| row.ml(wide * -0.5))
                    .children((0..columns).map(|_| {
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(wide)
                            .h(tall)
                            .whitespace_nowrap()
                            .child(text.clone())
                    }))
            }));
        self.base.relative().child(layer).child(
            canvas(
                move |bounds, window, cx| {
                    if *area.read(cx) != bounds.size {
                        area.update(cx, |area, cx| {
                            *area = bounds.size;
                            cx.notify();
                        });
                        window.request_animation_frame();
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .inset_0(),
        )
    }
}
