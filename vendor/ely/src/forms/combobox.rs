use std::rc::Rc;

use gpui::{
    App, Context, ElementId, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Subscription, Window, div, prelude::*,
};

use super::{
    Choice, Input, InputEvent, TextInput,
    options::{OnValue, Pick, Popup, step},
    select::{Picker, measure_anchor},
    text::{Down, Enter, Up},
};
use crate::{
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize},
};

/// Rows whose label or note holds `query`, ignoring case. All rows for an empty query.
pub(crate) fn matching(choices: &[Choice], query: &str) -> Vec<Choice> {
    let query = query.trim().to_lowercase();
    let holds = |text: &SharedString| text.to_lowercase().contains(&query);
    choices
        .iter()
        .filter(|choice| holds(&choice.label) || choice.note.as_ref().is_some_and(holds))
        .cloned()
        .collect()
}

/// What a combobox restores on blur, and whether Escape closed it.
struct Typing {
    restore: Option<SharedString>,
    dismissed: bool,
    last: String,
    shown: Option<Option<SharedString>>,
    _blur: Subscription,
}

/// A text field that filters a list as you type. Enter or a click picks a row.
#[derive(IntoElement)]
pub struct Combobox {
    id: ElementId,
    state: Entity<TextInput>,
    choices: Vec<Choice>,
    selected: Option<SharedString>,
    free: bool,
    size: ControlSize,
    on_change: Option<OnValue>,
}

impl Combobox {
    pub fn new(
        id: impl Into<ElementId>,
        state: &Entity<TextInput>,
        choices: impl IntoIterator<Item = Choice>,
    ) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            choices: choices.into_iter().collect(),
            selected: None,
            free: false,
            size: ControlSize::default(),
            on_change: None,
        }
    }

    pub fn selected(mut self, value: impl Into<SharedString>) -> Self {
        self.selected = Some(value.into());
        self
    }

    /// Keeps any text; the list only completes it, as an autocomplete does.
    pub fn free(mut self) -> Self {
        self.free = true;
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Runs with the value of a picked row.
    pub fn on_change(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Combobox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let input = self.state.clone();
        let typing = window.use_keyed_state(
            (self.id.clone(), "typing"),
            cx,
            |window, cx: &mut Context<Typing>| {
                let blur = cx.subscribe_in(&input, window, |typing, input, event, _, cx| {
                    if *event != InputEvent::Blur {
                        return;
                    }
                    typing.dismissed = false;
                    if let Some(label) = typing.restore.clone()
                        && input.read(cx).text() != label.as_ref()
                    {
                        log::info!("combobox: kept {label:?}");
                        input.update(cx, |input, cx| input.set_text(label, cx));
                    }
                });
                Typing {
                    restore: None,
                    dismissed: false,
                    last: String::new(),
                    shown: None,
                    _blur: blur,
                }
            },
        );
        let picker =
            window.use_keyed_state((self.id.clone(), "picker"), cx, |_, _| Picker::default());
        let label = self
            .selected
            .as_ref()
            .and_then(|value| self.choices.iter().find(|choice| choice.value == *value))
            .map(|choice| choice.label.clone());
        let focused = self.state.read(cx).focus().is_focused(window);
        if typing.read(cx).shown.as_ref() != Some(&self.selected) {
            typing.update(cx, |typing, _| typing.shown = Some(self.selected.clone()));
            let wanted = match (&label, self.free) {
                (Some(label), _) => Some(label.clone()),
                (None, false) => Some(SharedString::default()),
                (None, true) => None,
            };
            if let Some(label) = wanted
                && !focused
                && self.state.read(cx).text() != label.as_ref()
            {
                log::info!("combobox {:?}: shows {label:?}", self.id);
                self.state.update(cx, |input, cx| input.set_text(label, cx));
            }
        }
        let text = self.state.read(cx).text().to_string();
        let restore = (!self.free).then(|| label.clone().unwrap_or_default());
        typing.update(cx, |typing, _| {
            typing.restore = restore;
            if typing.last != text {
                typing.last = text.clone();
                typing.dismissed = false;
            }
        });
        let query = if label.as_ref().is_some_and(|label| label.as_ref() == text) {
            ""
        } else {
            text.as_str()
        };
        let rows = Rc::new(matching(&self.choices, query));
        let open = focused && !typing.read(cx).dismissed && !rows.is_empty();
        if open != picker.read(cx).open {
            let chosen = self.selected.as_ref().and_then(|value| {
                rows.iter()
                    .position(|row| !row.disabled && row.value == *value)
            });
            picker.update(cx, |picker, _| {
                picker.open = open;
                if let Some(at) = chosen.filter(|_| open) {
                    picker.highlighted = at;
                }
            });
        }
        let kept = picker.read(cx).highlighted;
        let highlighted = if rows.get(kept).is_some_and(|row| !row.disabled) {
            kept
        } else {
            step(&rows, rows.len().saturating_sub(1), 1)
        };
        let (anchor, scroll) = (picker.read(cx).anchor, picker.read(cx).scroll.clone());
        let reveal = Picker::reveal(&picker, open, highlighted, cx);
        let pick: Pick = {
            let (id, rows, state, typing, on_change) = (
                self.id.clone(),
                rows.clone(),
                self.state.clone(),
                typing.clone(),
                self.on_change,
            );
            Rc::new(move |ix, window, cx| {
                let choice = rows[ix].clone();
                if choice.disabled {
                    return;
                }
                log::info!("combobox {id:?}: {}", choice.value);
                state.update(cx, |input, cx| input.set_text(choice.label.clone(), cx));
                typing.update(cx, |typing, cx| {
                    typing.last = choice.label.to_string();
                    typing.dismissed = true;
                    cx.notify();
                });
                if let Some(on_change) = &on_change {
                    on_change(&choice.value, window, cx);
                }
            })
        };
        let chosen: Vec<SharedString> = self.selected.iter().cloned().collect();
        let subtle = cx.theme().colors.fg_subtle;
        let (up, down, enter, escape) = (
            (picker.clone(), rows.clone()),
            (picker.clone(), rows.clone(), typing.clone()),
            pick.clone(),
            typing.clone(),
        );
        div()
            .id(self.id.clone())
            .relative()
            .capture_action(move |_: &Up, _, cx| {
                if open {
                    cx.stop_propagation();
                    let at = step(&up.1, highlighted, -1);
                    Picker::show(&up.0, true, at, cx);
                }
            })
            .capture_action(move |_: &Down, _, cx| {
                let (picker, rows, typing) = &down;
                if open {
                    cx.stop_propagation();
                    Picker::show(picker, true, step(rows, highlighted, 1), cx);
                } else if typing.read(cx).dismissed {
                    cx.stop_propagation();
                    typing.update(cx, |typing, cx| {
                        typing.dismissed = false;
                        cx.notify();
                    });
                }
            })
            .capture_action(move |_: &Enter, window, cx| {
                if open {
                    cx.stop_propagation();
                    enter(highlighted, window, cx);
                }
            })
            .on_key_down(move |event, _, cx| {
                if open && event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    escape.update(cx, |typing, cx| {
                        typing.dismissed = true;
                        cx.notify();
                    });
                }
            })
            .child(
                Input::new(&self.state).size(self.size).suffix(
                    Icon::new(IconName::ChevronsUpDown)
                        .size(IconSize::Xs)
                        .color(subtle),
                ),
            )
            .child(measure_anchor(picker))
            .when(open, |field| {
                field.child(
                    Popup {
                        id: (self.id, "list").into(),
                        anchor,
                        rows: &rows,
                        highlighted: Some(highlighted),
                        checked: Some(&chosen),
                        pick,
                        dismiss: None,
                        scroll: Some(&scroll),
                        reveal,
                    }
                    .render(window, cx),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{Choice, matching};

    #[test]
    fn matching_ignores_case_and_keeps_order() {
        let rows = [
            Choice::new("ber", "Berlin"),
            Choice::new("lis", "Lisbon"),
            Choice::new("oslo", "Oslo").note("NO"),
        ];
        let labels = |query| {
            matching(&rows, query)
                .into_iter()
                .map(|c| c.label)
                .collect::<Vec<_>>()
        };
        assert_eq!(labels("LI"), ["Berlin", "Lisbon"]);
        assert_eq!(labels(""), ["Berlin", "Lisbon", "Oslo"]);
        assert!(labels("xyz").is_empty());
        assert_eq!(labels("no"), ["Oslo"]);
    }
}
