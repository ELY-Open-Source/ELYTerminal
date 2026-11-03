use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, ScrollHandle,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use super::{
    Choice,
    check::toggled,
    options::{OnValues, Pick, option_row, step},
};
use crate::{
    primitives::tab_stop,
    theme::{ActiveTheme, Radius, TextSize},
};

/// `selected` after choosing row `ix`: one row replaces, several toggle.
pub(crate) fn chosen(
    choices: &[Choice],
    selected: &[SharedString],
    ix: usize,
    multiple: bool,
) -> Vec<SharedString> {
    let value = &choices[ix].value;
    if multiple {
        toggled(choices, selected, value, !selected.contains(value))
    } else {
        vec![value.clone()]
    }
}

#[derive(Default)]
struct Cursor {
    highlighted: usize,
    scroll: ScrollHandle,
}

/// A list to choose from in place: one row, or several with `multiple`.
#[derive(IntoElement)]
pub struct ListBox {
    id: ElementId,
    choices: Vec<Choice>,
    selected: Vec<SharedString>,
    multiple: bool,
    on_change: Option<OnValues>,
}

impl ListBox {
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

    /// Lets any number of rows be chosen.
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

impl RenderOnce for ListBox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let count = self.choices.len();
        assert!(count > 0, "list box {:?} has no choices", self.id);
        let focus = tab_stop((self.id.clone(), "focus").into(), true, window, cx);
        let focused = focus.is_focused(window);
        let cursor =
            window.use_keyed_state((self.id.clone(), "cursor"), cx, |_, _| Cursor::default());
        let (highlighted, scroll) = {
            let cursor = cursor.read(cx);
            (cursor.highlighted.min(count - 1), cursor.scroll.clone())
        };
        let (choices, selected) = (Rc::new(self.choices), Rc::new(self.selected));
        let press: Pick = {
            let (id, choices, selected, multiple) = (
                self.id.clone(),
                choices.clone(),
                selected.clone(),
                self.multiple,
            );
            let (cursor, on_change) = (cursor.clone(), self.on_change);
            Rc::new(move |ix, window, cx| {
                cursor.update(cx, |cursor, cx| {
                    cursor.highlighted = ix;
                    cx.notify();
                });
                let next = chosen(&choices, &selected, ix, multiple);
                log::info!("list box {id:?}: {next:?}");
                if let Some(on_change) = &on_change {
                    on_change(&next, window, cx);
                }
            })
        };
        let rows: Vec<_> = choices
            .iter()
            .enumerate()
            .map(|(ix, choice)| {
                let press = press.clone();
                let on = selected.contains(&choice.value);
                option_row(
                    ("row", ix),
                    choice,
                    focused && ix == highlighted,
                    Some(on),
                    cx,
                )
                .when(!choice.disabled, |row| {
                    row.on_click(move |_, window, cx| press(ix, window, cx))
                })
            })
            .collect();
        let theme = cx.theme();
        let colors = &theme.colors;
        let keys = choices.clone();
        div()
            .id(self.id)
            .track_focus(&focus)
            .track_scroll(&scroll)
            .overflow_y_scroll()
            .h_full()
            .max_h(theme.list_max_height())
            .flex()
            .flex_col()
            .p_1()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(if focused { colors.focus } else { colors.border })
            .bg(colors.surface)
            .text_size(theme.text_size(TextSize::Sm))
            .on_key_down(move |event, window, cx| {
                let at = cursor.read(cx).highlighted.min(count - 1);
                let to = match event.keystroke.key.as_str() {
                    "down" => step(&keys, at, 1),
                    "up" => step(&keys, at, -1),
                    "home" => step(&keys, count - 1, 1),
                    "end" => step(&keys, 0, -1),
                    "enter" | "space" if !keys[at].disabled => {
                        cx.stop_propagation();
                        press(at, window, cx);
                        return;
                    }
                    _ => return,
                };
                cx.stop_propagation();
                cursor.update(cx, |cursor, cx| {
                    cursor.highlighted = to;
                    cursor.scroll.scroll_to_item(to);
                    cx.notify();
                });
            })
            .children(rows)
    }
}

#[cfg(test)]
mod tests {
    use gpui::SharedString;

    use super::{Choice, chosen};

    #[test]
    fn one_row_replaces_and_several_toggle_in_order() {
        let rows = [
            Choice::new("a", "A"),
            Choice::new("b", "B"),
            Choice::new("c", "C"),
        ];
        let values = |list: &[&str]| {
            list.iter()
                .map(|v| SharedString::from(v.to_string()))
                .collect::<Vec<_>>()
        };
        assert_eq!(chosen(&rows, &values(&["a"]), 2, false), values(&["c"]));
        assert_eq!(chosen(&rows, &values(&["c"]), 0, true), values(&["a", "c"]));
        assert_eq!(chosen(&rows, &values(&["a", "c"]), 2, true), values(&["a"]));
    }
}
