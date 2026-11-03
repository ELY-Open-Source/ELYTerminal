use std::{ops::Range, rc::Rc};

use gpui::{
    App, AppContext as _, Context, ElementId, Entity, Focusable, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, Styled, Window, div, prelude::*,
};

use super::markdown::link_at;
use crate::{
    buttons::{Button, ButtonVariant, IconButton},
    forms::{Enter, Input, Run, TextInput, float, is_url, surface},
    primitives::{FocusScope, IconName, give_back, take_focus},
    theme::{ActiveTheme, ControlSize},
};

/// What the editor edits: a link's range and words, or the selection that becomes one.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Linking {
    pub range: Range<usize>,
    pub label: String,
    /// The address, when the range is already a link.
    pub address: Option<String>,
}

impl Linking {
    /// The link around the selection, or the selection itself.
    pub fn at(text: &str, selection: Range<usize>) -> Self {
        match link_at(text, selection.start) {
            Some((range, label, address)) if selection.end <= range.end => Self {
                range,
                label,
                address: Some(address),
            },
            _ => Self {
                label: text[selection.clone()].to_string(),
                range: selection,
                address: None,
            },
        }
    }

    /// The link written in markdown for `address`; with no words of its own it reads as its address.
    pub fn written(&self, address: &str) -> String {
        let label = if self.label.is_empty() {
            address
        } else {
            &self.label
        };
        format!("[{label}]({address})")
    }
}

/// The address field and whether it took focus yet.
struct Editing {
    address: Entity<TextInput>,
    seeded: bool,
}

/// Replaces `range` of the field's text with `text` as one undo step, and closes.
fn write(field: &Entity<TextInput>, range: Range<usize>, text: &str, cx: &mut App) {
    field.update(cx, |input, cx| {
        input.select(range, cx);
        input.insert(text, cx);
    });
}

/// Edits the link at the caret, or links the selection, in a panel under it: the address is checked as it is typed and applied with Enter; the link can be removed or opened from the same place. Escape or a press outside closes it, and focus returns to the text. Render it only while open.
#[derive(IntoElement)]
pub struct LinkEditor {
    id: ElementId,
    field: Entity<TextInput>,
    on_close: Run,
}

impl LinkEditor {
    pub fn new(
        id: impl Into<ElementId>,
        field: &Entity<TextInput>,
        on_close: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            field: field.clone(),
            on_close: Rc::new(on_close),
        }
    }
}

impl RenderOnce for LinkEditor {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let input = self.field.read(cx);
        let linking = Linking::at(input.text(), input.selection());
        let anchor = input.bounds_for(linking.range.start);
        let takeover = take_focus((self.id.clone(), "takeover"), window, cx);
        let editing = window.use_keyed_state(
            (self.id.clone(), "editing"),
            cx,
            |window, cx: &mut Context<Editing>| Editing {
                address: cx.new(|cx| TextInput::new(window, cx).placeholder("https://")),
                seeded: false,
            },
        );
        let address = editing.read(cx).address.clone();
        if !editing.read(cx).seeded {
            log::info!("link editor: open over {:?}", linking.range);
            let written = linking.address.clone().unwrap_or_default();
            address.update(cx, |field, cx| field.set_text(written, cx));
            window.focus(&address.focus_handle(cx), cx);
            editing.update(cx, |editing, _| editing.seeded = true);
        }
        let focus = takeover.read(cx).focus.clone();
        let close: Run = {
            let (takeover, on_close) = (takeover.clone(), self.on_close.clone());
            Rc::new(move |window, cx| {
                log::info!("link editor: closed");
                give_back(&takeover, window, cx);
                on_close(window, cx);
            })
        };
        let typed = address.read(cx).text().trim().to_string();
        let valid = is_url(&typed);
        let apply: Run = {
            let (field, linking, close) = (self.field.clone(), linking.clone(), close.clone());
            let typed = typed.clone();
            Rc::new(move |window, cx| {
                if !is_url(&typed) {
                    return;
                }
                log::info!("link editor: {typed} over {:?}", linking.range);
                write(&field, linking.range.clone(), &linking.written(&typed), cx);
                close(window, cx);
            })
        };
        let remove: Option<Run> = linking.address.as_ref().map(|_| {
            let (field, linking, close) = (self.field.clone(), linking.clone(), close.clone());
            Rc::new(move |window: &mut Window, cx: &mut App| {
                log::info!("link editor: unlinked {:?}", linking.range);
                write(&field, linking.range.clone(), &linking.label, cx);
                close(window, cx);
            }) as Run
        });
        let Some(anchor) = anchor else {
            return div().id(self.id);
        };
        let (enter, escape, out) = (apply.clone(), close.clone(), close.clone());
        let press = apply.clone();
        let opened = linking.address.clone();
        let panel = div()
            .id((self.id.clone(), "panel"))
            .occlude()
            .capture_action(move |_: &Enter, window, cx| {
                cx.stop_propagation();
                enter(window, cx);
            })
            .on_key_down(move |event, window, cx| {
                if event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    escape(window, cx);
                }
            })
            .on_mouse_down_out(move |_, window, cx| out(window, cx))
            .child(
                FocusScope::new(&focus).trap().child(
                    surface((self.id.clone(), "surface"), cx)
                        .p_2()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div().w(cx.theme().menu_width()).child(
                                Input::new(&address)
                                    .size(ControlSize::Sm)
                                    .invalid(!typed.is_empty() && !valid),
                            ),
                        )
                        .child(
                            Button::new((self.id.clone(), "apply"), "Apply")
                                .variant(ButtonVariant::Primary)
                                .size(ControlSize::Sm)
                                .disabled(!valid)
                                .on_click(move |_, window, cx| press(window, cx)),
                        )
                        .when_some(opened, |row, url| {
                            row.child(
                                IconButton::new((self.id.clone(), "open"), IconName::ExternalLink)
                                    .size(ControlSize::Sm)
                                    .tooltip("Open link")
                                    .on_click(move |_, _, cx| cx.open_url(&url)),
                            )
                        })
                        .when_some(remove, |row, remove| {
                            row.child(
                                IconButton::new((self.id.clone(), "remove"), IconName::Trash2)
                                    .size(ControlSize::Sm)
                                    .tooltip("Remove link")
                                    .on_click(move |_, window, cx| remove(window, cx)),
                            )
                        }),
                ),
            );
        div()
            .id(self.id.clone())
            .child(float(self.id, anchor, 1, panel, window, cx))
    }
}

#[cfg(test)]
mod tests {
    use super::Linking;

    #[test]
    fn a_link_is_edited_whole_and_a_selection_becomes_one() {
        let text = "see [the docs](https://ely.dev) here";
        let inside = Linking::at(text, 8..8);
        assert_eq!(
            (inside.range, inside.address.as_deref()),
            (4..31, Some("https://ely.dev"))
        );
        let plain = Linking::at(text, 32..36);
        assert_eq!(
            (
                plain.range.clone(),
                plain.label.as_str(),
                plain.address.clone()
            ),
            (32..36, "here", None)
        );
        assert_eq!(plain.written("https://x.dev"), "[here](https://x.dev)");
        let bare = Linking::at(text, 0..0);
        assert_eq!(
            bare.written("https://x.dev"),
            "[https://x.dev](https://x.dev)"
        );
    }
}
