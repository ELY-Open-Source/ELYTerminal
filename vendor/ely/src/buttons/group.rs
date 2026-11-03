use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use super::{
    Button,
    button::{Slot, label_size},
};
use crate::{
    i18n,
    primitives::{FocusRing, Icon, IconName, Tooltip},
    theme::{ActiveTheme, ControlSize, Radius},
};

/// Buttons joined edge to edge.
#[derive(IntoElement, Default)]
pub struct ButtonGroup {
    buttons: Vec<Button>,
}

impl ButtonGroup {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn button(mut self, button: Button) -> Self {
        self.buttons.push(button);
        self
    }
}

impl RenderOnce for ButtonGroup {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let count = self.buttons.len();
        assert!(count >= 2, "a button group joins two buttons or more");
        div().flex().children(
            self.buttons
                .into_iter()
                .enumerate()
                .map(|(ix, mut button)| {
                    button.slot = Some(match ix {
                        0 => Slot::First,
                        ix if ix + 1 == count => Slot::Last,
                        _ => Slot::Middle,
                    });
                    button
                }),
        )
    }
}

/// A toggle choice: a label, an icon, or both.
#[derive(Clone)]
pub struct ToggleItem {
    value: SharedString,
    label: Option<SharedString>,
    icon: Option<IconName>,
    tooltip: Option<SharedString>,
}

impl ToggleItem {
    pub fn new(value: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: None,
            icon: None,
            tooltip: None,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn tooltip(mut self, text: impl Into<SharedString>) -> Self {
        self.tooltip = Some(text.into());
        self
    }
}

type OnPress = Rc<dyn Fn(&mut Window, &mut App)>;

fn toggle_face(
    id: ElementId,
    item: &ToggleItem,
    on: bool,
    size: ControlSize,
    press: Option<OnPress>,
    cx: &App,
) -> impl IntoElement + use<> {
    assert!(
        item.label.is_some() || item.icon.is_some(),
        "toggle {} needs a label or an icon",
        item.value
    );
    let theme = cx.theme();
    let colors = &theme.colors;
    let (text, icon_size) = label_size(size);
    let fg = if on { colors.fg } else { colors.fg_muted };
    let (hover, value) = (colors.hover, item.value.clone());
    let name = item
        .label
        .clone()
        .or_else(|| item.tooltip.clone())
        .unwrap_or_else(|| item.value.clone());
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(name)
        .aria_toggled(on.into())
        .debug_selector(move || format!("toggle {value}"))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .gap_1p5()
        .h(theme.control_height(size))
        .min_w(theme.control_height(size))
        .when(item.label.is_some(), |face| {
            face.px(theme.control_padding(size))
        })
        .rounded(theme.radius(Radius::Md))
        .border_1()
        .border_color(gpui::transparent_black())
        .text_size(theme.text_size(text))
        .font_weight(FontWeight::MEDIUM)
        .text_color(fg)
        .when(on, |face| face.bg(colors.active))
        .map(|face| match press {
            None => face
                .aria_description(i18n::text(cx, "state.unavailable", &[]))
                .opacity(0.45)
                .cursor_not_allowed(),
            Some(press) => face
                .when(!on, |face| face.hover(|style| style.bg(hover)))
                .cursor_pointer()
                .tab_index(0)
                .focus_ring(cx)
                .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                .on_click(move |_, window, cx| press(window, cx)),
        })
        .when_some(item.icon, |face, icon| {
            face.child(Icon::new(icon).size(icon_size).color(fg))
        })
        .when_some(item.label.clone(), |face, label| face.child(label))
        .when_some(item.tooltip.clone(), |face, text| {
            face.tooltip(Tooltip::text(text))
        })
}

type OnToggle = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// A button that stays pressed until pressed again.
#[derive(IntoElement)]
pub struct ToggleButton {
    id: ElementId,
    item: ToggleItem,
    on: bool,
    disabled: bool,
    size: ControlSize,
    on_toggle: Option<OnToggle>,
}

impl ToggleButton {
    pub fn new(id: impl Into<ElementId>, item: ToggleItem, on: bool) -> Self {
        Self {
            id: id.into(),
            item,
            on,
            disabled: false,
            size: ControlSize::default(),
            on_toggle: None,
        }
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Shown, not pressable; the item's tooltip can say why.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_toggle(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ToggleButton {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (on, toggle, value) = (self.on, self.on_toggle, self.item.value.clone());
        let press: OnPress = Rc::new(move |window, cx| {
            log::info!("toggle {value}: {}", !on);
            if let Some(toggle) = &toggle {
                toggle(!on, window, cx);
            }
        });
        let press = (!self.disabled).then_some(press);
        toggle_face(self.id, &self.item, on, self.size, press, cx)
    }
}

type OnChange = Rc<dyn Fn(&[SharedString], &mut Window, &mut App)>;

/// Wrapping toggles; one chosen, or several with `multiple`.
#[derive(IntoElement)]
pub struct ToggleGroup {
    id: ElementId,
    items: Vec<ToggleItem>,
    selected: Vec<SharedString>,
    multiple: bool,
    size: ControlSize,
    on_change: Option<OnChange>,
}

impl ToggleGroup {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
            selected: Vec::new(),
            multiple: false,
            size: ControlSize::default(),
            on_change: None,
        }
    }

    pub fn item(mut self, item: ToggleItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn selected(mut self, values: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.selected = values.into_iter().map(Into::into).collect();
        self
    }

    pub fn multiple(mut self) -> Self {
        self.multiple = true;
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Reports the whole selection after each press.
    pub fn on_change(
        mut self,
        handler: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

/// The selection after pressing `value`.
fn pressed(selected: &[SharedString], value: &SharedString, multiple: bool) -> Vec<SharedString> {
    let on = selected.contains(value);
    match (multiple, on) {
        (_, true) => selected.iter().filter(|v| *v != value).cloned().collect(),
        (true, false) => selected.iter().chain([value]).cloned().collect(),
        (false, false) => vec![value.clone()],
    }
}

impl RenderOnce for ToggleGroup {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let faces: Vec<_> = self
            .items
            .iter()
            .enumerate()
            .map(|(ix, item)| {
                let on = self.selected.contains(&item.value);
                let next = pressed(&self.selected, &item.value, self.multiple);
                let change = self.on_change.clone();
                let press: OnPress = Rc::new(move |window, cx| {
                    log::info!("toggle group: {next:?}");
                    if let Some(change) = &change {
                        change(&next, window, cx);
                    }
                });
                toggle_face(
                    (self.id.clone(), format!("toggle-{ix}")).into(),
                    item,
                    on,
                    self.size,
                    Some(press),
                    cx,
                )
            })
            .collect();
        div()
            .id(self.id)
            .flex()
            .flex_wrap()
            .items_center()
            .gap_0p5()
            .p_0p5()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(theme.colors.border)
            .children(faces)
    }
}

#[cfg(test)]
mod tests {
    use gpui::SharedString;

    use super::pressed;

    fn values(list: &[&'static str]) -> Vec<SharedString> {
        list.iter().map(|v| SharedString::from(*v)).collect()
    }

    #[test]
    fn single_replaces_multiple_adds_and_pressing_again_clears() {
        let b = SharedString::from("b");
        assert_eq!(pressed(&values(&["a"]), &b, false), values(&["b"]));
        assert_eq!(pressed(&values(&["a"]), &b, true), values(&["a", "b"]));
        assert_eq!(pressed(&values(&["a", "b"]), &b, true), values(&["a"]));
        assert_eq!(pressed(&values(&["b"]), &b, false), values(&[]));
    }
}
