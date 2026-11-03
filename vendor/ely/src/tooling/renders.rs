use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div,
};

use crate::{
    theme::{ActiveTheme, TextSize},
    typography::tabular,
};

/// How many times the view that draws it has rendered, beside a label: place it in a view's render to see how often that view redraws. It shows the count and nothing moves, since any motion would redraw the view it counts.
#[derive(IntoElement)]
pub struct RenderCounter {
    id: ElementId,
    label: SharedString,
}

impl RenderCounter {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

impl RenderOnce for RenderCounter {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let renders = window.use_keyed_state((self.id.clone(), "renders"), cx, |_, _| 0u64);
        let count = renders.update(cx, |renders, _| {
            *renders += 1;
            *renders
        });
        let theme = cx.theme();
        let colors = &theme.colors;
        let label = self.label;
        div()
            .debug_selector({
                let label = label.clone();
                move || format!("renders-{label}-{count}")
            })
            .flex()
            .flex_none()
            .items_center()
            .gap_1p5()
            .px_2()
            .py_0p5()
            .rounded_full()
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .text_size(theme.text_size(TextSize::Xs))
            .child(div().text_color(colors.fg_muted).child(label))
            .child(
                tabular(div())
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(colors.fg)
                    .child(count.to_string()),
            )
    }
}
