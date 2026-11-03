use std::rc::Rc;

use gpui::{
    AnyElement, App, Div, ElementId, FocusHandle, FontWeight, InteractiveElement, IntoElement,
    MouseButton, ParentElement, RenderOnce, SharedString, Stateful, StatefulInteractiveElement,
    Styled, Window, div, prelude::*,
};

use super::{
    CheckState,
    check::check_mark,
    options::{OnFlag, Run},
    radio::radio_mark,
};
use crate::{
    motion,
    primitives::{Icon, IconName, tab_stop},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
};

/// What a card says: a title, a line under it, a preview under them, and an icon at its end.
struct Face {
    title: SharedString,
    description: Option<SharedString>,
    preview: Option<AnyElement>,
    icon: Option<IconName>,
    disabled: bool,
    on: bool,
}

impl Face {
    fn new(title: SharedString, on: bool) -> Self {
        Self {
            title,
            description: None,
            preview: None,
            icon: None,
            disabled: false,
            on,
        }
    }
}

/// A bordered choice: the mark, then the words. The border darkens while chosen.
fn card(
    id: ElementId,
    face: Face,
    mark: AnyElement,
    focus: &FocusHandle,
    press: Run,
    window: &Window,
    cx: &App,
) -> Stateful<Div> {
    let (on, focused) = (face.on, focus.is_focused(window));
    let theme = cx.theme();
    let colors = &theme.colors;
    let border = if focused {
        colors.focus
    } else if on {
        colors.accent
    } else {
        colors.border
    };
    let strong = colors.border_strong;
    div()
        .id(id)
        .track_focus(focus)
        .flex()
        .items_start()
        .gap_3()
        .p_4()
        .rounded(theme.radius(Radius::Lg))
        .border_1()
        .border_color(border)
        .bg(colors.surface)
        .map(|card| {
            if face.disabled {
                return card.opacity(0.5);
            }
            card.cursor_pointer()
                .when(!on && !focused, |card| {
                    card.hover(|style| style.border_color(strong))
                })
                .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                .on_click(move |_, window, cx| press(window, cx))
        })
        .child(div().pt_0p5().child(mark))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .text_size(theme.text_size(TextSize::Base))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(colors.fg)
                        .child(face.title),
                )
                .when_some(face.description, |words, description| {
                    words.child(
                        div()
                            .mt_0p5()
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(colors.fg_muted)
                            .child(description),
                    )
                })
                .when_some(face.preview, |words, preview| {
                    words.child(div().pt_2().child(preview))
                }),
        )
        .when_some(face.icon, |card, icon| {
            card.child(Icon::new(icon).size(IconSize::Md).color(colors.fg_muted))
        })
}

/// A card that turns on and off, with a checkbox in its corner.
#[derive(IntoElement)]
pub struct CheckboxCard {
    id: ElementId,
    checked: bool,
    face: Face,
    on_change: Option<OnFlag>,
}

impl CheckboxCard {
    pub fn new(id: impl Into<ElementId>, checked: bool, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            checked,
            face: Face::new(title.into(), checked),
            on_change: None,
        }
    }

    pub fn description(mut self, text: impl Into<SharedString>) -> Self {
        self.face.description = Some(text.into());
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.face.icon = Some(icon);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.face.disabled = disabled;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for CheckboxCard {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus = tab_stop(
            (self.id.clone(), "focus").into(),
            !self.face.disabled,
            window,
            cx,
        );
        let changes = motion::changes((self.id.clone(), "changes"), self.checked, window, cx);
        let (id, next, on_change) = (self.id.clone(), !self.checked, self.on_change);
        let mark = check_mark(
            CheckState::from(self.checked),
            false,
            self.face.disabled,
            changes,
            cx,
        );
        let press: Run = Rc::new(move |window, cx| {
            log::info!("checkbox card {id:?}: {}", if next { "on" } else { "off" });
            if let Some(on_change) = &on_change {
                on_change(next, window, cx);
            }
        });
        card(self.id, self.face, mark, &focus, press, window, cx)
    }
}

/// A card chosen from several, with a radio in its corner.
#[derive(IntoElement)]
pub struct RadioCard {
    id: ElementId,
    selected: bool,
    face: Face,
    on_select: Option<Run>,
}

impl RadioCard {
    pub fn new(id: impl Into<ElementId>, selected: bool, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            selected,
            face: Face::new(title.into(), selected),
            on_select: None,
        }
    }

    pub fn description(mut self, text: impl Into<SharedString>) -> Self {
        self.face.description = Some(text.into());
        self
    }

    /// Shows `element` under the words, such as what the choice looks like.
    pub fn preview(mut self, element: impl IntoElement) -> Self {
        self.face.preview = Some(element.into_any_element());
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.face.icon = Some(icon);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.face.disabled = disabled;
        self
    }

    /// Runs when chosen while not selected.
    pub fn on_select(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for RadioCard {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus = tab_stop(
            (self.id.clone(), "focus").into(),
            !self.face.disabled,
            window,
            cx,
        );
        let changes = motion::changes((self.id.clone(), "changes"), self.selected, window, cx);
        let (id, selected, on_select) = (self.id.clone(), self.selected, self.on_select);
        let mark = radio_mark(self.selected, false, self.face.disabled, changes, cx);
        let press: Run = Rc::new(move |window, cx| {
            if selected {
                return;
            }
            log::info!("radio card {id:?}: chosen");
            if let Some(on_select) = &on_select {
                on_select(window, cx);
            }
        });
        card(self.id, self.face, mark, &focus, press, window, cx)
    }
}
