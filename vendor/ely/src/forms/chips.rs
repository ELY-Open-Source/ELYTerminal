use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use super::{Choice, listbox::chosen, options::OnValues};
use crate::{
    primitives::{FocusRing, Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, TextSize},
};

/// Pills for a short set of choices. One is chosen, or several with `multiple`.
#[derive(IntoElement)]
pub struct ChoiceChips {
    id: ElementId,
    choices: Vec<Choice>,
    selected: Vec<SharedString>,
    multiple: bool,
    on_change: Option<OnValues>,
}

impl ChoiceChips {
    pub fn new(id: impl Into<ElementId>, choices: impl IntoIterator<Item = Choice>) -> Self {
        Self {
            id: id.into(),
            choices: choices.into_iter().collect(),
            selected: Vec::new(),
            multiple: false,
            on_change: None,
        }
    }

    pub fn selected(mut self, values: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.selected = values.into_iter().map(Into::into).collect();
        self
    }

    /// Filter chips: each toggles, and chosen ones show a check.
    pub fn multiple(mut self) -> Self {
        self.multiple = true;
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ChoiceChips {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let (choices, selected) = (Rc::new(self.choices), Rc::new(self.selected));
        let chips: Vec<_> = choices
            .iter()
            .enumerate()
            .map(|(ix, choice)| {
                let on = selected.contains(&choice.value);
                let (fg, bg, border) = if on {
                    (colors.on_accent, colors.accent, colors.accent)
                } else {
                    (colors.fg_muted, colors.surface, colors.border_strong)
                };
                let (id, choices, selected, multiple, on_change) = (
                    self.id.clone(),
                    choices.clone(),
                    selected.clone(),
                    self.multiple,
                    self.on_change.clone(),
                );
                div()
                    .id(("chip", ix))
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .h(theme.control_height(ControlSize::Sm))
                    .px_3()
                    .rounded_full()
                    .border_1()
                    .border_color(border)
                    .bg(bg)
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(if choice.disabled {
                        colors.fg_disabled
                    } else {
                        fg
                    })
                    .when(!choice.disabled, |chip| {
                        chip.tab_index(0)
                            .focus_ring(cx)
                            .cursor_pointer()
                            .when(!on, |chip| chip.hover(|style| style.bg(colors.hover)))
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.prevent_default()
                            })
                            .on_click(move |_, window, cx| {
                                let next = chosen(&choices, &selected, ix, multiple);
                                log::info!("choice chips {id:?}: {next:?}");
                                if let Some(on_change) = &on_change {
                                    on_change(&next, window, cx);
                                }
                            })
                    })
                    .when(on && self.multiple, |chip| {
                        chip.child(Icon::new(IconName::Check).size(IconSize::Xs).color(fg))
                    })
                    .when_some(choice.icon, |chip, icon| {
                        chip.child(Icon::new(icon).size(IconSize::Xs).color(fg))
                    })
                    .child(choice.label.clone())
            })
            .collect();
        div().id(self.id).flex().flex_wrap().gap_2().children(chips)
    }
}
