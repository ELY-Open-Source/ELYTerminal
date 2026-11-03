use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use crate::{
    primitives::tab_stop,
    theme::{ActiveTheme, IconSize, Radius, TextSize},
    typography::tabular,
};

type OnRatio = Rc<dyn Fn((u32, u32), &mut Window, &mut App)>;

/// The share of a square a `width:height` shape spans across and down; its long side spans it all.
pub(crate) fn span(width: u32, height: u32) -> (f32, f32) {
    assert!(width > 0 && height > 0, "a ratio of {width}:{height}");
    let (width, height) = (width as f32, height as f32);
    let long = width.max(height);
    (width / long, height / long)
}

/// A picture's shape: a chip per ratio, each with an outline drawn to its proportions. The chosen chip takes accent; each is a Tab stop.
#[derive(IntoElement)]
pub struct AspectRatioPicker {
    id: ElementId,
    ratios: Vec<(u32, u32)>,
    selected: (u32, u32),
    on_select: Option<OnRatio>,
}

impl AspectRatioPicker {
    /// `selected` is one of `ratios`, each `(width, height)`.
    pub fn new(
        id: impl Into<ElementId>,
        ratios: impl IntoIterator<Item = (u32, u32)>,
        selected: (u32, u32),
    ) -> Self {
        let ratios: Vec<(u32, u32)> = ratios.into_iter().collect();
        if !ratios.contains(&selected) {
            log::error!(
                "aspect ratio: {}:{} is not among the ratios; none selected",
                selected.0,
                selected.1
            );
        }
        Self {
            id: id.into(),
            ratios,
            selected,
            on_select: None,
        }
    }

    pub fn on_select(
        mut self,
        handler: impl Fn((u32, u32), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for AspectRatioPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let chips: Vec<_> = self
            .ratios
            .iter()
            .map(|&(width, height)| {
                let id: ElementId = (self.id.clone(), format!("ratio-{width}-{height}")).into();
                let focus = tab_stop((id.clone(), "focus").into(), true, window, cx);
                (id, focus.is_focused(window), focus, (width, height))
            })
            .collect();
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let glyph = theme.icon_size(IconSize::Md);
        div()
            .flex()
            .flex_wrap()
            .gap_2()
            .children(chips.into_iter().map(|(id, focused, focus, ratio)| {
                let chosen = ratio == self.selected;
                let ink = if chosen {
                    colors.accent
                } else {
                    colors.fg_muted
                };
                let border = if focused {
                    colors.focus
                } else if chosen {
                    colors.accent
                } else {
                    colors.border
                };
                let (across, down) = span(ratio.0, ratio.1);
                let pick = self.on_select.clone();
                div()
                    .id(id)
                    .track_focus(&focus)
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .px_2p5()
                    .py_1()
                    .rounded(theme.radius(Radius::Md))
                    .border_1()
                    .border_color(border)
                    .text_size(theme.text_size(TextSize::Sm))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(ink)
                    .cursor_pointer()
                    .when(!chosen && !focused, |chip| {
                        chip.hover(|style| style.border_color(colors.border_strong))
                    })
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .on_click(move |_, window, cx| {
                        if chosen {
                            return;
                        }
                        log::info!("aspect ratio: {}:{}", ratio.0, ratio.1);
                        if let Some(pick) = &pick {
                            pick(ratio, window, cx);
                        }
                    })
                    .child(
                        div()
                            .flex_none()
                            .size(glyph)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .w(glyph * across)
                                    .h(glyph * down)
                                    .rounded(theme.radius(Radius::Sm))
                                    .border_1()
                                    .border_color(ink),
                            ),
                    )
                    .child(tabular(div()).child(format!("{}:{}", ratio.0, ratio.1)))
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::span;

    #[test]
    fn a_ratio_spans_its_long_side_whole() {
        assert_eq!(span(16, 9), (1.0, 0.5625));
        assert_eq!(span(9, 16), (0.5625, 1.0));
        assert_eq!(span(1, 1), (1.0, 1.0));
    }

    #[test]
    #[should_panic(expected = "a ratio of 0:1")]
    fn an_empty_side_fails_loud() {
        span(0, 1);
    }
}
