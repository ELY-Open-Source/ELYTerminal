use std::rc::Rc;

use gpui::{
    AnyElement, App, Context, Div, ElementId, Entity, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Point, RenderOnce, ScrollHandle, SharedString, StatefulInteractiveElement,
    Styled, Window, div, prelude::*,
};

use crate::{
    data_display::color_mark,
    forms::{
        Choice, Down, Enter, OnValue, OnValues, Pick, SearchInput, TextInput, Up, marked_row,
        reveal, revealer,
    },
    layout::on_axis,
    primitives::IconName,
    theme::{ActiveTheme, TextSize},
};

/// A label: its key, its name, and its hue among the theme's chart colors.
#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    pub key: SharedString,
    pub name: SharedString,
    pub hue: usize,
}

impl Label {
    pub fn new(key: impl Into<SharedString>, name: impl Into<SharedString>, hue: usize) -> Self {
        Self {
            key: key.into(),
            name: name.into(),
            hue,
        }
    }
}

/// Label `name`'s dot in the chart color at `hue`, centered in an icon's box.
pub(super) fn hue_dot(name: &str, hue: usize, cx: &App) -> Div {
    color_mark(cx.theme().colors.hue(hue, format_args!("label {name}")), cx)
}

/// The search field, the row under the keyboard, the query it was set for, and the list's scroll.
struct Finding {
    search: Entity<TextInput>,
    at: usize,
    query: String,
    revealed: Option<usize>,
    scroll: ScrollHandle,
}

/// Moves the row under the keyboard by `by` among `count`, wrapping.
fn step(finding: &Entity<Finding>, count: usize, by: isize, cx: &mut App) {
    if count == 0 {
        return;
    }
    finding.update(cx, |finding, cx| {
        let at = finding.at.min(count - 1) as isize;
        finding.at = (at + by).rem_euclid(count as isize) as usize;
        cx.notify();
    })
}

/// Labels to put on a message: the field finds them by name, arrows move between them, and a press or Enter turns one on or off. With `on_create`, a name no label has can be made into one.
#[derive(IntoElement)]
pub struct LabelPicker {
    id: ElementId,
    labels: Vec<Label>,
    selected: Vec<SharedString>,
    on_change: Option<OnValues>,
    on_create: Option<OnValue>,
}

impl LabelPicker {
    pub fn new(id: impl Into<ElementId>, labels: impl IntoIterator<Item = Label>) -> Self {
        let labels: Vec<Label> = labels.into_iter().collect();
        for (ix, label) in labels.iter().enumerate() {
            let before = &labels[..ix];
            let key = before.iter().any(|other| other.key == label.key);
            assert!(!key, "label {} twice", label.key);
            let name = label.name.to_lowercase();
            let named = before.iter().any(|other| other.name.to_lowercase() == name);
            assert!(!named, "label name {} twice", label.name);
        }
        Self {
            id: id.into(),
            labels,
            selected: Vec::new(),
            on_change: None,
            on_create: None,
        }
    }

    /// The keys of the labels on.
    pub fn selected(mut self, keys: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.selected = keys.into_iter().map(Into::into).collect();
        self
    }

    /// Gets the keys on, in the labels' order, each time one turns on or off.
    pub fn on_change(
        mut self,
        handler: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    /// Offers to make a label of a name no label has, and gets the name. The owner adds the label, on if it should be.
    pub fn on_create(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_create = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for LabelPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        for key in &self.selected {
            let known = self.labels.iter().any(|label| label.key == *key);
            if !known {
                log::error!(
                    "label picker {:?}: no label {key}; it is not ticked",
                    self.id
                );
            }
        }
        let finding = window.use_keyed_state(
            (self.id.clone(), "finding"),
            cx,
            |window, cx: &mut Context<Finding>| Finding {
                search: cx.new(|cx| TextInput::new(window, cx).placeholder("Find or make a label")),
                at: 0,
                query: String::new(),
                revealed: None,
                scroll: ScrollHandle::new(),
            },
        );
        let search = finding.read(cx).search.clone();
        let query = search.read(cx).text().trim().to_string();
        if finding.read(cx).query != query {
            finding.update(cx, |finding, _| {
                finding.query = query.clone();
                finding.at = 0;
                finding.scroll.set_offset(Point::default());
            });
        }
        let lower = query.to_lowercase();
        let fits: Vec<&Label> = self
            .labels
            .iter()
            .filter(|label| label.name.to_lowercase().contains(&lower))
            .collect();
        let fresh = self.on_create.is_some()
            && !query.is_empty()
            && !self
                .labels
                .iter()
                .any(|label| label.name.to_lowercase() == lower);
        let count = fits.len() + usize::from(fresh);
        let at = finding.read(cx).at.min(count.saturating_sub(1));
        let keyed = search.read(cx).focus().is_focused(window);
        let shown = reveal(&finding, |finding| &mut finding.revealed, count > 0, at, cx);
        let scroll = finding.read(cx).scroll.clone();
        let selected = Rc::new(self.selected);
        let pick: Pick = {
            let keys: Vec<SharedString> = fits.iter().map(|label| label.key.clone()).collect();
            let all: Vec<SharedString> =
                self.labels.iter().map(|label| label.key.clone()).collect();
            let (id, selected, search) = (self.id.clone(), selected.clone(), search.clone());
            let (on_change, on_create) = (self.on_change, self.on_create.clone());
            let name = SharedString::from(query.clone());
            Rc::new(move |ix, window, cx| {
                let Some(key) = keys.get(ix) else {
                    log::info!("label picker {id:?}: create {name}");
                    search.update(cx, |input, cx| input.set_text(String::new(), cx));
                    if let Some(on_create) = &on_create {
                        on_create(&name, window, cx);
                    }
                    return;
                };
                let on = selected.contains(key);
                log::info!(
                    "label picker {id:?}: {key} {}",
                    if on { "off" } else { "on" }
                );
                let next: Vec<SharedString> = all
                    .iter()
                    .filter(|each| {
                        if *each == key {
                            !on
                        } else {
                            selected.contains(each)
                        }
                    })
                    .cloned()
                    .collect();
                if let Some(on_change) = &on_change {
                    on_change(&next, window, cx);
                }
            })
        };
        let row = |ix: usize, choice: Choice, mark: Option<AnyElement>, on: bool| {
            let pick = pick.clone();
            let key = choice.value.clone();
            marked_row(
                (self.id.clone(), format!("label-{key}")),
                &choice,
                mark,
                keyed && ix == at,
                Some(on),
                cx,
            )
            .debug_selector(move || format!("label {key}"))
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .on_click(move |_, window, cx| pick(ix, window, cx))
        };
        let mut rows: Vec<AnyElement> = fits
            .iter()
            .enumerate()
            .map(|(ix, label)| {
                let choice = Choice::new(label.key.clone(), label.name.clone());
                let dot = hue_dot(&label.name, label.hue, cx).into_any_element();
                row(ix, choice, Some(dot), selected.contains(&label.key)).into_any_element()
            })
            .collect();
        if fresh {
            let choice = Choice::new("+create", format!("Create “{query}”")).icon(IconName::Plus);
            rows.push(row(fits.len(), choice, None, false).into_any_element());
        }
        let theme = cx.theme();
        let none = rows.is_empty().then(|| {
            div()
                .px_2()
                .py_2()
                .text_color(theme.colors.fg_subtle)
                .child(match (self.labels.is_empty(), self.on_create.is_some()) {
                    (true, true) => "No labels yet. Type a name to make one.",
                    (true, false) => "No labels yet.",
                    (false, _) => "No label fits.",
                })
        });
        let (up, down, chosen) = (finding.clone(), finding.clone(), finding);
        div()
            .id(self.id.clone())
            .flex()
            .flex_col()
            .gap_2()
            .capture_action(move |_: &Up, _, cx| {
                cx.stop_propagation();
                step(&up, count, -1, cx);
            })
            .capture_action(move |_: &Down, _, cx| {
                cx.stop_propagation();
                step(&down, count, 1, cx);
            })
            .capture_action(move |_: &Enter, window, cx| {
                if count > 0 {
                    cx.stop_propagation();
                    let at = chosen.read(cx).at.min(count - 1);
                    pick(at, window, cx);
                }
            })
            .child(SearchInput::new((self.id.clone(), "search"), &search))
            .child(
                on_axis(
                    div()
                        .id((self.id.clone(), "labels"))
                        .track_scroll(&scroll)
                        .max_h(theme.list_max_height())
                        .overflow_y_scroll(),
                )
                .flex()
                .flex_col()
                .text_size(theme.text_size(TextSize::Sm))
                .children(rows)
                .children(none)
                .children(shown.map(|(ix, done)| revealer(&scroll, ix, done))),
            )
    }
}
