use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, transparent_black,
};

use crate::{
    documents::source,
    forms::OnValue,
    primitives::{Icon, IconName, Image, tab_stop},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
    typography::Ellipsis,
};

/// A style to generate in: its key, its name, and a picture of it; one with no picture draws a plain tile.
#[derive(Clone, Debug, PartialEq)]
pub struct StylePreset {
    pub key: SharedString,
    pub name: SharedString,
    pub picture: Option<SharedString>,
}

/// Styles to generate in, a tile each with its picture and name. The chosen tile takes accent; each is a Tab stop.
#[derive(IntoElement)]
pub struct StylePresetPicker {
    id: ElementId,
    presets: Vec<StylePreset>,
    selected: SharedString,
    on_select: Option<OnValue>,
}

impl StylePresetPicker {
    /// `selected` names one of `presets` by key.
    pub fn new(
        id: impl Into<ElementId>,
        presets: impl IntoIterator<Item = StylePreset>,
        selected: impl Into<SharedString>,
    ) -> Self {
        let (presets, selected): (Vec<StylePreset>, SharedString) =
            (presets.into_iter().collect(), selected.into());
        if !presets.iter().any(|preset| preset.key == selected) {
            log::error!("style presets: {selected} is not among the presets; none selected");
        }
        Self {
            id: id.into(),
            presets,
            selected,
            on_select: None,
        }
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for StylePresetPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tiles: Vec<_> = self
            .presets
            .into_iter()
            .map(|preset| {
                let id: ElementId = (self.id.clone(), format!("preset-{}", preset.key)).into();
                let focus = tab_stop((id.clone(), "focus").into(), true, window, cx);
                (id, focus.is_focused(window), focus, preset)
            })
            .collect();
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let side = theme.generative().preset;
        div()
            .flex()
            .flex_wrap()
            .gap_3()
            .children(tiles.into_iter().map(|(id, focused, focus, preset)| {
                let chosen = preset.key == self.selected;
                let ring = if focused {
                    colors.focus
                } else if chosen {
                    colors.accent
                } else {
                    transparent_black()
                };
                let (key, pick) = (preset.key.clone(), self.on_select.clone());
                let inner = theme.radius(Radius::Md);
                let picture = match &preset.picture {
                    Some(path) => Image::new((id.clone(), "picture"), source(path))
                        .size_full()
                        .rounded(inner)
                        .into_any_element(),
                    None => div()
                        .size_full()
                        .rounded(inner)
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(colors.sunken)
                        .child(
                            Icon::new(IconName::Ban)
                                .size(IconSize::Md)
                                .color(colors.fg_subtle),
                        )
                        .into_any_element(),
                };
                div()
                    .id(id)
                    .track_focus(&focus)
                    .w(side)
                    .flex()
                    .flex_col()
                    .gap_1p5()
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .on_click(move |_, window, cx| {
                        if chosen {
                            return;
                        }
                        log::info!("style preset: {key}");
                        if let Some(pick) = &pick {
                            pick(&key, window, cx);
                        }
                    })
                    .child(
                        div()
                            .size(side)
                            .p_0p5()
                            .rounded(theme.radius(Radius::Lg))
                            .border_1()
                            .border_color(ring)
                            .child(picture),
                    )
                    .child(
                        div()
                            .px_0p5()
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(if chosen { colors.fg } else { colors.fg_muted })
                            .child(Ellipsis::new(preset.name)),
                    )
            }))
    }
}
