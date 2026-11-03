use std::{rc::Rc, time::Duration};

use gpui::{
    App, Div, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce,
    Role, ScrollHandle, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window,
    div, prelude::FluentBuilder as _,
};
use web_time::Instant;

use super::ListItem;
use crate::primitives::tab_stop;

/// How a press or a key picks rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pick {
    One,
    Toggle,
    Range,
    All,
}

/// The selection after picking row `at`, a range reaching back to `anchor`; in row order.
pub(crate) fn picked(
    keys: &[SharedString],
    selected: &[SharedString],
    anchor: usize,
    at: usize,
    pick: Pick,
) -> Vec<SharedString> {
    let (low, high) = (anchor.min(at), anchor.max(at));
    keys.iter()
        .enumerate()
        .filter(|(ix, key)| match pick {
            Pick::One => *ix == at,
            Pick::Toggle => selected.contains(key) != (*ix == at),
            Pick::Range => (low..=high).contains(ix),
            Pick::All => true,
        })
        .map(|(_, key)| key.clone())
        .collect()
}

/// The row the keyboard is on and where a Shift range starts, by index and by key so they follow their rows when the owner reorders them, and the scroll that follows them; the owner's last selection, and the one the list last sent; whether Enter's press fell here; the letters typed and when the last one came.
#[derive(Default)]
struct Cursor {
    at: usize,
    anchor: usize,
    keys: Option<(SharedString, SharedString)>,
    scroll: ScrollHandle,
    seen: Option<Vec<SharedString>>,
    sent: Option<Vec<SharedString>>,
    armed: bool,
    typed: String,
    typed_at: Option<Instant>,
}

/// Letters typed within this pause join one prefix.
const TYPING: Duration = Duration::from_millis(800);

/// The row typed letters go to: one letter moves past the cursor to the next row that starts with it, round to the top; more letters stay on the cursor's row while it still starts with them. Disabled rows are passed over.
pub(crate) fn typed_to(
    titles: &[SharedString],
    off: &[bool],
    at: usize,
    typed: &str,
) -> Option<usize> {
    let count = titles.len();
    let from = if typed.chars().count() == 1 {
        at + 1
    } else {
        at
    };
    (0..count)
        .map(|step| (from + step) % count)
        .find(|ix| !off[*ix] && titles[*ix].to_lowercase().starts_with(typed))
}

type OnSelect = Rc<dyn Fn(&[SharedString], &mut Window, &mut App)>;
type OnActivate = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;
type Picker = Rc<dyn Fn(usize, Pick, &mut Window, &mut App)>;

/// Rows you select. A press picks one; with `multiple`, Cmd-press adds or drops one and Shift-press takes the range from the last pick. Up and Down move, Shift with them extends, Space toggles, Cmd-A takes all, typing a row's first letters moves to it, and Enter activates the row on its release, so an activation that moves focus leaves the release nothing to press. Keys pass over disabled rows, and no pick adds one.
#[derive(IntoElement)]
pub struct SelectableList {
    id: ElementId,
    base: Div,
    rows: Vec<(SharedString, ListItem)>,
    selected: Vec<SharedString>,
    multiple: bool,
    label: Option<SharedString>,
    on_change: Option<OnSelect>,
    on_activate: Option<OnActivate>,
}

impl SelectableList {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            rows: Vec::new(),
            selected: Vec::new(),
            multiple: false,
            label: None,
            on_change: None,
            on_activate: None,
        }
    }

    /// A row under `key`, which the selection names it by.
    pub fn row(mut self, key: impl Into<SharedString>, item: ListItem) -> Self {
        self.rows.push((key.into(), item));
        self
    }

    pub fn selected(mut self, keys: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.selected = keys.into_iter().map(Into::into).collect();
        self
    }

    /// Names the list for assistive tech.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn multiple(mut self) -> Self {
        self.multiple = true;
        self
    }

    /// Enter, or a double press, on a row.
    pub fn on_activate(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_activate = Some(Rc::new(handler));
        self
    }

    /// Gets the keys now selected, in row order.
    pub fn on_change(
        mut self,
        handler: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl Styled for SelectableList {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for SelectableList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let count = self.rows.len();
        let focus = tab_stop((self.id.clone(), "focus").into(), true, window, cx);
        let focused = focus.is_focused(window);
        let cursor =
            window.use_keyed_state((self.id.clone(), "cursor"), cx, |_, _| Cursor::default());
        let keys: Rc<[SharedString]> = self.rows.iter().map(|(key, _)| key.clone()).collect();
        if cursor.read(cx).seen.as_ref() != Some(&self.selected) {
            cursor.update(cx, |cursor, _| {
                let echo = cursor.sent.take().as_ref() == Some(&self.selected);
                if let Some(first) = self.selected.first().filter(|_| !echo) {
                    cursor.keys = Some((first.clone(), first.clone()));
                }
                cursor.seen = Some(self.selected.clone());
            });
        }
        let (at, anchor, scroll) = {
            let cursor = cursor.read(cx);
            let place = |key: Option<&SharedString>, ix: usize| {
                key.and_then(|key| keys.iter().position(|row| row == key))
                    .unwrap_or(ix.min(count.saturating_sub(1)))
            };
            let held = cursor.keys.as_ref();
            (
                place(held.map(|keys| &keys.0), cursor.at),
                place(held.map(|keys| &keys.1), cursor.anchor),
                cursor.scroll.clone(),
            )
        };
        if (at, anchor) != (cursor.read(cx).at, cursor.read(cx).anchor) {
            cursor.update(cx, |cursor, _| (cursor.at, cursor.anchor) = (at, anchor));
        }
        let titles: Rc<[SharedString]> = self
            .rows
            .iter()
            .map(|(_, item)| item.title_text().clone())
            .collect();
        let off: Rc<[bool]> = self
            .rows
            .iter()
            .map(|(_, item)| item.is_disabled())
            .collect();
        let barred: Vec<SharedString> = self
            .rows
            .iter()
            .filter(|(_, item)| item.is_disabled())
            .map(|(key, _)| key.clone())
            .collect();
        let selected = self.selected;
        let multiple = self.multiple;
        let pick: Picker = {
            let (id, keys, selected, cursor, on_change) = (
                self.id.clone(),
                keys.clone(),
                selected.clone(),
                cursor.clone(),
                self.on_change,
            );
            Rc::new(move |ix, pick, window, cx| {
                let pick = match (multiple, pick) {
                    (true, pick) => pick,
                    (false, Pick::All) => return,
                    (false, _) => Pick::One,
                };
                let next: Vec<SharedString> =
                    picked(&keys, &selected, cursor.read(cx).anchor, ix, pick)
                        .into_iter()
                        .filter(|key| !barred.contains(key) || selected.contains(key))
                        .collect();
                cursor.update(cx, |cursor, cx| {
                    cursor.at = ix;
                    if pick != Pick::Range {
                        cursor.anchor = ix;
                    }
                    cursor.keys = Some((
                        keys[ix].clone(),
                        keys[cursor.anchor.min(keys.len() - 1)].clone(),
                    ));
                    cursor.scroll.scroll_to_item(ix);
                    cursor.sent = Some(next.clone());
                    cx.notify();
                });
                log::info!("selectable list {id:?}: {next:?}");
                if let Some(on_change) = &on_change {
                    on_change(&next, window, cx);
                }
            })
        };
        let activate: OnActivate = {
            let (id, on_activate) = (self.id.clone(), self.on_activate);
            Rc::new(move |key, window, cx| {
                log::info!("selectable list {id:?}: activated {key}");
                if let Some(on_activate) = &on_activate {
                    on_activate(key, window, cx);
                }
            })
        };
        let rows: Vec<_> = self
            .rows
            .into_iter()
            .enumerate()
            .map(|(ix, (key, item))| {
                let (pick, focus, activate) = (pick.clone(), focus.clone(), activate.clone());
                item.selected(selected.contains(&key))
                    .place(ix + 1, count)
                    .current(focused && ix == at)
                    .on_click(move |event, window, cx| {
                        let held = event.modifiers();
                        let how = match (held.shift, held.platform) {
                            (true, _) => Pick::Range,
                            (false, true) => Pick::Toggle,
                            _ => Pick::One,
                        };
                        window.focus(&focus, cx);
                        pick(ix, how, window, cx);
                        if event.click_count() == 2 {
                            activate(&key, window, cx);
                        }
                    })
            })
            .collect();
        let (lifted, released, barred_rows, keyed) =
            (cursor.clone(), activate.clone(), off.clone(), keys.clone());
        self.base
            .id(self.id)
            .role(Role::List)
            .when_some(self.label, |list, label| list.aria_label(label))
            .track_focus(&focus)
            .track_scroll(&scroll)
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_0p5()
            .on_key_up(move |event, window, cx| {
                if event.keystroke.key != "enter" || count == 0 {
                    return;
                }
                let (armed, at) = lifted.update(cx, |cursor, _| {
                    (std::mem::take(&mut cursor.armed), cursor.at.min(count - 1))
                });
                if !armed {
                    return;
                }
                cx.stop_propagation();
                if barred_rows[at] {
                    return;
                }
                released(&keyed[at], window, cx);
            })
            .on_key_down(move |event, window, cx| {
                if count == 0 {
                    return;
                }
                let held = &event.keystroke.modifiers;
                let at = cursor.read(cx).at.min(count - 1);
                let extend = if held.shift { Pick::Range } else { Pick::One };
                let open = |ix: &usize| !off[*ix];
                let letter = event
                    .keystroke
                    .key_char
                    .as_ref()
                    .filter(|typed| !typed.trim().is_empty())
                    .filter(|_| !held.platform && !held.control && !held.alt && !held.function);
                if let Some(letter) = letter {
                    let now = cx.background_executor().now();
                    let typed = cursor.update(cx, |cursor, _| {
                        if cursor
                            .typed_at
                            .is_none_or(|at| now.duration_since(at) > TYPING)
                        {
                            cursor.typed.clear();
                        }
                        cursor.typed.push_str(&letter.to_lowercase());
                        cursor.typed_at = Some(now);
                        cursor.typed.clone()
                    });
                    if let Some(to) = typed_to(&titles, &off, at, &typed) {
                        cx.stop_propagation();
                        pick(to, Pick::One, window, cx);
                    }
                    return;
                }
                let (to, how) = match event.keystroke.key.as_str() {
                    "down" => ((at + 1..count).find(open), extend),
                    "up" => ((0..at).rev().find(open), extend),
                    "home" => ((0..count).find(open), extend),
                    "end" => ((0..count).rev().find(open), extend),
                    "space" => (Some(at).filter(open), Pick::Toggle),
                    "enter" => {
                        cx.stop_propagation();
                        cursor.update(cx, |cursor, _| cursor.armed = !held.modified());
                        return;
                    }
                    "a" if held.platform => (Some(at), Pick::All),
                    _ => return,
                };
                cx.stop_propagation();
                if let Some(to) = to {
                    pick(to, how, window, cx);
                }
            })
            .children(rows)
    }
}

/// A button inside a pressable row: its press and its Space or Enter stay its own, so the row neither selects nor activates.
pub(crate) fn row_action(button: impl IntoElement) -> gpui::Div {
    div()
        .flex_none()
        .on_mouse_down(MouseButton::Left, |_, window, cx| {
            window.prevent_default();
            cx.stop_propagation();
        })
        .on_key_down(|event, _, cx| {
            let key = event.keystroke.key.as_str();
            if matches!(key, "space" | "enter") && !event.keystroke.modifiers.modified() {
                cx.stop_propagation();
            }
        })
        .child(button)
}

#[cfg(test)]
mod tests {
    use gpui::SharedString;

    use super::{Pick, picked, typed_to};

    fn keys(list: &[&'static str]) -> Vec<SharedString> {
        list.iter().map(|key| SharedString::from(*key)).collect()
    }

    #[test]
    fn picks_one_toggle_a_range_or_all_in_row_order() {
        let rows = keys(&["a", "b", "c", "d"]);
        assert_eq!(picked(&rows, &keys(&["a"]), 0, 2, Pick::One), keys(&["c"]));
        assert_eq!(
            picked(&rows, &keys(&["c", "a"]), 0, 1, Pick::Toggle),
            keys(&["a", "b", "c"])
        );
        assert_eq!(
            picked(&rows, &keys(&["a", "c"]), 0, 0, Pick::Toggle),
            keys(&["c"])
        );
        assert_eq!(
            picked(&rows, &keys(&["a"]), 1, 3, Pick::Range),
            keys(&["b", "c", "d"])
        );
        assert_eq!(
            picked(&rows, &keys(&[]), 3, 1, Pick::Range),
            keys(&["b", "c", "d"])
        );
        assert_eq!(picked(&rows, &keys(&["b"]), 0, 0, Pick::All), rows);
    }

    #[test]
    fn typed_letters_go_to_the_next_row_that_starts_with_them() {
        let titles = keys(&["Apple", "Banana", "Blueberry", "Cherry"]);
        let open = [false; 4];
        assert_eq!(typed_to(&titles, &open, 0, "b"), Some(1));
        assert_eq!(
            typed_to(&titles, &open, 1, "b"),
            Some(2),
            "one letter moves on"
        );
        assert_eq!(
            typed_to(&titles, &open, 2, "b"),
            Some(1),
            "round to the top"
        );
        assert_eq!(
            typed_to(&titles, &open, 1, "ba"),
            Some(1),
            "a prefix stays while it fits"
        );
        assert_eq!(typed_to(&titles, &open, 1, "bl"), Some(2));
        assert_eq!(typed_to(&titles, &open, 0, "z"), None);
        let off = [false, true, false, false];
        assert_eq!(
            typed_to(&titles, &off, 0, "b"),
            Some(2),
            "a disabled row is passed over"
        );
    }
}
