use std::rc::Rc;

use gpui::{
    App, Context, ElementId, Entity, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, Styled, Subscription, Window, div, prelude::*,
};

use super::{
    Choice, InputEvent, TextInput,
    combobox::matching,
    options::{Pick, Popup, step},
    select::{Picker, measure_anchor},
    text::{Backspace, Down, Enter, Up},
};
use crate::{
    data_display::{Tag, Tone},
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
};

type OnChange = Rc<dyn Fn(Vec<SharedString>, &mut Window, &mut App)>;

struct Tags {
    input: Entity<TextInput>,
    tags: Vec<SharedString>,
    on_change: Option<OnChange>,
    dismissed: bool,
    _events: Subscription,
}

impl Tags {
    /// Adds the field's text as a tag, unless blank or already there.
    fn take(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let typed = self
            .input
            .read(cx)
            .text()
            .trim()
            .trim_end_matches(',')
            .trim()
            .to_string();
        self.add(typed, window, cx);
    }

    /// Adds `value` as a tag and clears the field, unless blank or already there.
    fn add(&mut self, value: String, window: &mut Window, cx: &mut Context<Self>) {
        self.input.update(cx, |input, cx| input.set_text("", cx));
        if value.is_empty() || self.tags.iter().any(|tag| tag.as_ref() == value) {
            return;
        }
        let mut next = self.tags.clone();
        next.push(value.into());
        log::info!("tag input: {} tags", next.len());
        if let Some(on_change) = self.on_change.clone() {
            on_change(next, window, cx);
        }
    }
}

/// A chip per tag, then a field. Enter or a comma adds; Backspace on empty drops the last. With suggestions, the rows that hold the typed words float under it: Up and Down move, Enter or a press adds one, Escape sets them aside.
#[derive(IntoElement)]
pub struct TagInput {
    id: ElementId,
    tags: Vec<SharedString>,
    placeholder: SharedString,
    check: Option<fn(&str) -> bool>,
    suggestions: Vec<Choice>,
    on_change: Option<OnChange>,
}

impl TagInput {
    pub fn new(
        id: impl Into<ElementId>,
        tags: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        Self {
            id: id.into(),
            tags: tags.into_iter().map(Into::into).collect(),
            placeholder: "Add a tag".into(),
            check: None,
            suggestions: Vec::new(),
            on_change: None,
        }
    }

    pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Marks each tag that fails `check` as danger, as text that is not an address.
    pub fn check(mut self, check: fn(&str) -> bool) -> Self {
        self.check = Some(check);
        self
    }

    /// Rows to offer as you type, each adding its value; a chip shows the label of the row whose value it holds.
    pub fn suggestions(mut self, choices: impl IntoIterator<Item = Choice>) -> Self {
        self.suggestions = choices.into_iter().collect();
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(Vec<SharedString>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for TagInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let placeholder = self.placeholder.clone();
        let state =
            window.use_keyed_state(self.id.clone(), cx, |window, cx: &mut Context<Tags>| {
                let input = cx.new(|cx| TextInput::new(window, cx).placeholder(placeholder));
                let events = cx.subscribe_in(&input, window, |tags, input, event, window, cx| {
                    if *event == InputEvent::Changed {
                        tags.dismissed = false;
                    }
                    let comma =
                        *event == InputEvent::Changed && input.read(cx).text().ends_with(',');
                    if comma || *event == InputEvent::Submit {
                        tags.take(window, cx);
                    }
                });
                Tags {
                    input,
                    tags: Vec::new(),
                    on_change: None,
                    dismissed: false,
                    _events: events,
                }
            });
        state.update(cx, |tags, _| {
            tags.tags = self.tags.clone();
            tags.on_change = self.on_change.clone();
        });
        let input = state.read(cx).input.clone();
        let focused = input.read(cx).focus().is_focused(window);
        let focus = input.read(cx).focus().clone();
        let picker =
            window.use_keyed_state((self.id.clone(), "picker"), cx, |_, _| Picker::default());
        let typed = input.read(cx).text().trim().to_string();
        let rows: Rc<Vec<Choice>> = Rc::new(match typed.is_empty() {
            true => Vec::new(),
            false => matching(&self.suggestions, &typed)
                .into_iter()
                .filter(|row| !self.tags.contains(&row.value))
                .collect(),
        });
        let open = focused && !state.read(cx).dismissed && !rows.is_empty();
        if open != picker.read(cx).open {
            Picker::show(&picker, open, 0, cx);
        }
        let kept = picker.read(cx).highlighted;
        let highlighted = if kept < rows.len() { kept } else { 0 };
        let (anchor, scroll) = (picker.read(cx).anchor, picker.read(cx).scroll.clone());
        let reveal = Picker::reveal(&picker, open, highlighted, cx);
        let pick: Pick = {
            let (rows, state) = (rows.clone(), state.clone());
            Rc::new(move |ix, window, cx| {
                let value = rows[ix].value.to_string();
                log::info!("tag input: suggestion {value}");
                state.update(cx, |tags, cx| tags.add(value, window, cx));
            })
        };
        let (up, down, enter, escape) =
            (picker.clone(), picker.clone(), pick.clone(), state.clone());
        let (up_rows, down_rows) = (rows.clone(), rows.clone());
        let theme = cx.theme();
        let colors = &theme.colors;
        let owner = self.id.clone();
        let chips = self.tags.iter().enumerate().map(|(ix, tag)| {
            let (tags, on_change) = (self.tags.clone(), self.on_change.clone());
            let failed = self.check.is_some_and(|check| !check(tag));
            let shown = self
                .suggestions
                .iter()
                .find(|row| row.value == *tag)
                .map_or(tag.clone(), |row| row.label.clone());
            Tag::new((owner.clone(), format!("tag-{ix}")), shown)
                .when(failed, |tag| tag.tone(Tone::Danger))
                .on_remove(move |window, cx| {
                    let mut next = tags.clone();
                    next.remove(ix);
                    log::info!("tag input: removed one, {} left", next.len());
                    if let Some(on_change) = &on_change {
                        on_change(next, window, cx);
                    }
                })
        });
        let (tags, on_change, empty_check) =
            (self.tags.clone(), self.on_change.clone(), input.clone());
        let popup = open.then(|| {
            Popup {
                id: (self.id.clone(), "suggestions").into(),
                anchor,
                rows: &rows,
                highlighted: Some(highlighted),
                checked: None,
                pick,
                dismiss: None,
                scroll: Some(&scroll),
                reveal,
            }
            .render(window, cx)
        });
        let field = self.id.clone();
        div()
            .id(self.id)
            .debug_selector(move || format!("tag-input {field}"))
            .relative()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .min_h(theme.control_height(ControlSize::Md))
            .px_1()
            .py_0p5()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(if focused {
                colors.focus
            } else {
                colors.border_strong
            })
            .bg(colors.surface)
            .text_size(theme.text_size(TextSize::Base))
            .text_color(colors.fg)
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                window.focus(&focus, cx)
            })
            .capture_action(move |_: &Up, _, cx| {
                if open {
                    cx.stop_propagation();
                    Picker::show(&up, true, step(&up_rows, highlighted, -1), cx);
                }
            })
            .capture_action(move |_: &Down, _, cx| {
                if open {
                    cx.stop_propagation();
                    Picker::show(&down, true, step(&down_rows, highlighted, 1), cx);
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
                    escape.update(cx, |tags, cx| {
                        tags.dismissed = true;
                        cx.notify();
                    });
                }
            })
            .capture_action(move |_: &Backspace, window, cx| {
                if !empty_check.read(cx).is_empty() || tags.is_empty() {
                    return;
                }
                cx.stop_propagation();
                let mut next = tags.clone();
                next.pop();
                log::info!("tag input: backspace dropped the last tag");
                if let Some(on_change) = &on_change {
                    on_change(next, window, cx);
                }
            })
            .children(chips)
            .child(
                div()
                    .flex_1()
                    .min_w(theme.pane_min() / 2.0)
                    .px_1()
                    .child(input),
            )
            .child(measure_anchor(picker))
            .children(popup)
    }
}
