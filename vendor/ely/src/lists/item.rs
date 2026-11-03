use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, ElementId, FontWeight, InteractiveElement, IntoElement,
    MouseButton, ParentElement, RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled,
    Window, div, prelude::*, relative,
};
use smallvec::SmallVec;

use crate::{
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
    typography::{Ellipsis, LEADING},
};

type OnClick = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// One row of a list: something to lead with, a title and a line under it, and something at the end. It lights on hover when it can be pressed, and stays lit when selected.
#[derive(IntoElement)]
pub struct ListItem {
    id: ElementId,
    title: SharedString,
    /// A field drawn in the title's place.
    editor: Option<AnyElement>,
    description: Option<SharedString>,
    detail: Option<SharedString>,
    leading: Option<AnyElement>,
    trailing: Option<AnyElement>,
    selected: bool,
    current: bool,
    disabled: bool,
    strong: bool,
    quiet: bool,
    struck: bool,
    /// Its place among its list's rows, from one, and their count.
    place: Option<(usize, usize)>,
    on_click: Option<OnClick>,
}

impl ListItem {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            editor: None,
            description: None,
            detail: None,
            leading: None,
            trailing: None,
            selected: false,
            current: false,
            disabled: false,
            strong: false,
            quiet: false,
            struck: false,
            place: None,
            on_click: None,
        }
    }

    /// Sets the title in the strong weight, as a channel with unread messages has it.
    pub fn strong(mut self, strong: bool) -> Self {
        self.strong = strong;
        self
    }

    /// Sets the title in the quiet color, as a muted channel has it.
    pub fn quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }

    /// Strikes the title through, as a finished task has it.
    pub fn struck(mut self, struck: bool) -> Self {
        self.struck = struck;
        self
    }

    /// A field in place of the title, as a row renamed in place has it; the title stays the row's name.
    pub fn editor(mut self, editor: impl IntoElement) -> Self {
        self.editor = Some(editor.into_any_element());
        self
    }

    /// A second line, quieter than the title.
    pub fn description(mut self, text: impl Into<SharedString>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// A third line, quieter still, such as a message's first words.
    pub fn detail(mut self, text: impl Into<SharedString>) -> Self {
        self.detail = Some(text.into());
        self
    }

    /// An icon or an avatar before the text.
    pub fn leading(mut self, element: impl IntoElement) -> Self {
        self.leading = Some(element.into_any_element());
        self
    }

    /// A time, a count or an action after the text.
    pub fn trailing(mut self, element: impl IntoElement) -> Self {
        self.trailing = Some(element.into_any_element());
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Rings the row the keyboard is on, in a list that has focus.
    pub fn current(mut self, current: bool) -> Self {
        self.current = current;
        self
    }

    /// Tells assistive tech it is row `at` of `of`, from one.
    pub fn place(mut self, at: usize, of: usize) -> Self {
        self.place = Some((at, of));
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub(crate) fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub(crate) fn title_text(&self) -> &SharedString {
        &self.title
    }

    /// Makes the row pressable; the press keeps focus where it is.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ListItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let line = |text: SharedString| {
            div()
                .min_w_0()
                .line_height(relative(LEADING))
                .child(Ellipsis::new(text))
        };
        let pressable = self.on_click.is_some() && !self.disabled;
        div()
            .id(self.id)
            .role(Role::ListItem)
            .aria_label(self.title.clone())
            .aria_selected(self.selected)
            .when_some(self.description.clone(), |row, text| {
                row.aria_description(text)
            })
            .when_some(self.place, |row, (at, of)| {
                row.aria_position_in_set(at).aria_size_of_set(of)
            })
            .flex()
            .items_center()
            .gap_3()
            .px_3()
            .py_1p5()
            .min_h(theme.control_height(ControlSize::Lg))
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(if self.current {
                colors.focus
            } else {
                gpui::transparent_black()
            })
            .when(self.selected, |row| {
                row.debug_selector(|| format!("item-selected-{}", self.title))
                    .bg(colors.active)
            })
            .when(self.disabled, |row| row.opacity(0.5))
            .when_some(self.on_click.filter(|_| pressable), |row, on_click| {
                row.cursor_pointer()
                    .when(!self.selected, |row| {
                        row.hover(|style| style.bg(colors.hover))
                    })
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .on_click(move |event, window, cx| on_click(event, window, cx))
            })
            .children(self.leading.map(|leading| div().flex_none().child(leading)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(match self.editor {
                        Some(editor) => div().min_w_0().child(editor).into_any_element(),
                        None => line(self.title)
                            .text_size(theme.text_size(TextSize::Base))
                            .when(self.strong, |title| title.font_weight(FontWeight::SEMIBOLD))
                            .when(self.struck, |title| title.line_through())
                            .text_color(if self.quiet {
                                colors.fg_muted
                            } else {
                                colors.fg
                            })
                            .into_any_element(),
                    })
                    .children(self.description.map(|text| {
                        line(text.clone())
                            .debug_selector(|| format!("item-description-{text}"))
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(colors.fg_muted)
                    }))
                    .children(self.detail.map(|text| {
                        line(text)
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(colors.fg_subtle)
                    })),
            )
            .children(self.trailing.map(|trailing| {
                div()
                    .flex_none()
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(colors.fg_subtle)
                    .child(trailing)
            }))
    }
}

/// Rows in a column; `divided` draws a hairline between them.
#[derive(IntoElement, Default)]
pub struct List {
    rows: SmallVec<[AnyElement; 8]>,
    divided: bool,
}

impl List {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn divided(mut self) -> Self {
        self.divided = true;
        self
    }
}

impl ParentElement for List {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.rows.extend(elements);
    }
}

impl RenderOnce for List {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let border = cx.theme().colors.border;
        let divided = self.divided;
        div()
            .flex()
            .flex_col()
            .when(!divided, |list| list.gap_0p5())
            .children(self.rows.into_iter().enumerate().map(move |(ix, row)| {
                div()
                    .when(divided && ix > 0, |row| {
                        row.border_t_1().border_color(border)
                    })
                    .child(row)
            }))
    }
}
