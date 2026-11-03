use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
    prelude::*,
};

use super::{Choice, ListBox, options::OnValues};
use crate::{
    buttons::{ButtonVariant, IconButton},
    primitives::IconName,
    theme::{ActiveTheme, TextSize},
};

/// Marked rows on each side, waiting to move.
#[derive(Default)]
struct Marks {
    left: Vec<SharedString>,
    right: Vec<SharedString>,
}

/// `chosen` after moving `marked` across, in choice order.
fn moved(
    choices: &[Choice],
    chosen: &[SharedString],
    marked: &[SharedString],
    add: bool,
) -> Vec<SharedString> {
    choices
        .iter()
        .map(|choice| &choice.value)
        .filter(|value| {
            if marked.contains(value) {
                add
            } else {
                chosen.contains(value)
            }
        })
        .cloned()
        .collect()
}

/// Two lists and the buttons between them. Mark rows, then move them across.
#[derive(IntoElement)]
pub struct TransferList {
    id: ElementId,
    choices: Vec<Choice>,
    chosen: Vec<SharedString>,
    titles: (SharedString, SharedString),
    on_change: Option<OnValues>,
}

impl TransferList {
    pub fn new(id: impl Into<ElementId>, choices: impl IntoIterator<Item = Choice>) -> Self {
        Self {
            id: id.into(),
            choices: choices.into_iter().collect(),
            chosen: Vec::new(),
            titles: ("Available".into(), "Chosen".into()),
            on_change: None,
        }
    }

    pub fn chosen(mut self, values: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.chosen = values.into_iter().map(Into::into).collect();
        self
    }

    pub fn titles(mut self, left: impl Into<SharedString>, right: impl Into<SharedString>) -> Self {
        self.titles = (left.into(), right.into());
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

impl RenderOnce for TransferList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let marks = window.use_keyed_state((self.id.clone(), "marks"), cx, |_, _| Marks::default());
        let (left, right) = {
            let marks = marks.read(cx);
            (marks.left.clone(), marks.right.clone())
        };
        let (available, picked): (Vec<Choice>, Vec<Choice>) = self
            .choices
            .iter()
            .cloned()
            .partition(|choice| !self.chosen.contains(&choice.value));
        let (choices, chosen) = (Rc::new(self.choices), Rc::new(self.chosen));
        let shift = |add: bool| {
            let (id, choices, chosen, marks, on_change) = (
                self.id.clone(),
                choices.clone(),
                chosen.clone(),
                marks.clone(),
                self.on_change.clone(),
            );
            move |_: &gpui::ClickEvent, window: &mut Window, cx: &mut App| {
                let marked = if add {
                    marks.read(cx).left.clone()
                } else {
                    marks.read(cx).right.clone()
                };
                let next = moved(&choices, &chosen, &marked, add);
                log::info!("transfer list {id:?}: {} chosen", next.len());
                marks.update(cx, |marks, cx| {
                    *marks = Marks::default();
                    cx.notify();
                });
                if let Some(on_change) = &on_change {
                    on_change(&next, window, cx);
                }
            }
        };
        let side = |key: &'static str,
                    title: SharedString,
                    rows: Vec<Choice>,
                    marked: Vec<SharedString>,
                    cx: &App| {
            let theme = cx.theme();
            let store = marks.clone();
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_size(theme.text_size(TextSize::Sm))
                        .text_color(theme.colors.fg_muted)
                        .child(format!("{title} · {}", rows.len())),
                )
                .child(div().h(theme.list_max_height()).map(|frame| {
                    if rows.is_empty() {
                        return frame;
                    }
                    frame.child(
                        ListBox::new((self.id.clone(), key), rows)
                            .multiple()
                            .selected(marked)
                            .on_change(move |next, _, cx| {
                                let next = next.to_vec();
                                store.update(cx, |marks, cx| {
                                    if key == "left" {
                                        marks.left = next;
                                    } else {
                                        marks.right = next;
                                    }
                                    cx.notify();
                                })
                            }),
                    )
                }))
        };
        let buttons = div()
            .flex()
            .flex_col()
            .justify_center()
            .gap_2()
            .child(
                IconButton::new((self.id.clone(), "add"), IconName::ChevronRight)
                    .variant(ButtonVariant::Outline)
                    .tooltip("Move to chosen")
                    .disabled(left.is_empty())
                    .on_click(shift(true)),
            )
            .child(
                IconButton::new((self.id.clone(), "remove"), IconName::ChevronLeft)
                    .variant(ButtonVariant::Outline)
                    .tooltip("Move back")
                    .disabled(right.is_empty())
                    .on_click(shift(false)),
            );
        let (available_title, chosen_title) = self.titles.clone();
        div()
            .flex()
            .gap_3()
            .child(side("left", available_title, available, left, cx))
            .child(buttons)
            .child(side("right", chosen_title, picked, right, cx))
    }
}

#[cfg(test)]
mod tests {
    use gpui::SharedString;

    use super::{Choice, moved};

    #[test]
    fn moving_keeps_choice_order() {
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
        assert_eq!(
            moved(&rows, &values(&["c"]), &values(&["a"]), true),
            values(&["a", "c"])
        );
        assert_eq!(
            moved(&rows, &values(&["a", "c"]), &values(&["c"]), false),
            values(&["a"])
        );
    }
}
