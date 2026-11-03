use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, Pixels, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, canvas, div, prelude::*, relative,
    rems, transparent_black,
};

use super::{FileIcon, OnEntry};
use crate::{
    forms::OnValue,
    layout::{columns_for, seeded::use_seeded},
    lists::DirEntry,
    primitives::{FocusRing, tab_stop},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
    typography::{LEADING, MiddleEllipsis},
};

/// The tile a key moves to from `at` in `count` tiles laid `columns` a row: across by one, up and down by a row, Home and End to the ends; down from over a short last row lands on its last tile.
pub(crate) fn stepped(at: usize, count: usize, columns: usize, key: &str) -> Option<usize> {
    let last = count.checked_sub(1)?;
    match key {
        "left" => at.checked_sub(1),
        "right" => (at < last).then_some(at + 1),
        "up" => at.checked_sub(columns),
        "down" if at + columns <= last => Some(at + columns),
        "down" => (at / columns < last / columns).then_some(last),
        "home" => Some(0),
        "end" => Some(last),
        _ => None,
    }
}

/// Files and folders as tiles, in the order given: a large icon over the name in two lines at most. A press picks a tile and a double press opens it; with focus the arrows move by tile and by row, Home and End go to the ends, and Enter opens.
#[derive(IntoElement)]
pub struct FileGrid {
    id: ElementId,
    entries: Vec<DirEntry>,
    selected: Option<SharedString>,
    on_select: Option<OnValue>,
    on_open: Option<OnEntry>,
}

impl FileGrid {
    pub fn new(id: impl Into<ElementId>, entries: impl IntoIterator<Item = DirEntry>) -> Self {
        let entries: Vec<DirEntry> = entries.into_iter().collect();
        for (ix, entry) in entries.iter().enumerate() {
            assert!(
                !entries[..ix]
                    .iter()
                    .any(|other| other.name() == entry.name()),
                "{} twice",
                entry.name()
            );
        }
        Self {
            id: id.into(),
            entries,
            selected: None,
            on_select: None,
            on_open: None,
        }
    }

    /// The entry picked, by name.
    pub fn selected(mut self, name: impl Into<SharedString>) -> Self {
        let name = name.into();
        if !self.entries.iter().any(|entry| *entry.name() == name) {
            log::error!("file grid: no entry {name}; none selected");
        }
        self.selected = Some(name);
        self
    }

    /// Gets the name of the entry picked.
    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    pub fn on_open(mut self, handler: impl Fn(&DirEntry, &mut Window, &mut App) + 'static) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for FileGrid {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus = tab_stop((self.id.clone(), "focus").into(), true, window, cx);
        let cursor = use_seeded((self.id.clone(), "cursor"), self.selected, window, cx);
        let width = window.use_keyed_state((self.id.clone(), "width"), cx, |_, _| Pixels::ZERO);
        let entries: Rc<[DirEntry]> = self.entries.into();
        let count = entries.len();
        let at = cursor
            .read(cx)
            .value
            .as_ref()
            .and_then(|name| entries.iter().position(|entry| entry.name() == name));
        let theme = cx.theme();
        let colors = &theme.colors;
        let rem = window.rem_size();
        let (tile, gap) = (theme.files().tile, rems(0.5));
        let columns =
            columns_for(*width.read(cx), tile.to_pixels(rem), gap.to_pixels(rem)) as usize;
        let pick = {
            let (id, entries, cursor, on_select) = (
                self.id.clone(),
                entries.clone(),
                cursor.clone(),
                self.on_select,
            );
            Rc::new(move |ix: usize, window: &mut Window, cx: &mut App| {
                let name = entries[ix].name().clone();
                cursor.update(cx, |cursor, cx| {
                    cursor.value = Some(name.clone());
                    cx.notify();
                });
                log::info!("file grid {id:?}: picked {name}");
                if let Some(on_select) = &on_select {
                    on_select(&name, window, cx);
                }
            })
        };
        let open = {
            let (id, entries, on_open) = (self.id.clone(), entries.clone(), self.on_open);
            Rc::new(move |ix: usize, window: &mut Window, cx: &mut App| {
                log::info!("file grid {id:?}: open {}", entries[ix].name());
                if let Some(on_open) = &on_open {
                    on_open(&entries[ix], window, cx);
                }
            })
        };
        let tiles = entries.iter().enumerate().map(|(ix, entry)| {
            let (pick, open) = (pick.clone(), open.clone());
            div()
                .id((self.id.clone(), format!("tile-{ix}")))
                .flex_none()
                .w(tile)
                .flex()
                .flex_col()
                .items_center()
                .gap_1()
                .px_1()
                .py_2()
                .rounded(theme.radius(Radius::Md))
                .cursor_pointer()
                .when(at == Some(ix), |tile| tile.bg(colors.active))
                .when(at != Some(ix), |tile| {
                    tile.hover(|style| style.bg(colors.hover))
                })
                .child(FileIcon::entry(entry).size(IconSize::Xxl))
                .child(
                    div()
                        .debug_selector(|| format!("file-name {ix}"))
                        .w_full()
                        .flex()
                        .justify_center()
                        .text_size(theme.text_size(TextSize::Sm))
                        .line_height(relative(LEADING))
                        .text_color(colors.fg)
                        .child(MiddleEllipsis::new(entry.name().clone())),
                )
                .on_click(move |event, window, cx| {
                    pick(ix, window, cx);
                    if event.click_count() == 2 {
                        open(ix, window, cx);
                    }
                })
        });
        let tiles: Vec<_> = tiles.collect();
        let measured = width.clone();
        div()
            .id(self.id)
            .debug_selector(|| "file-grid".into())
            .relative()
            .flex()
            .flex_wrap()
            .gap(gap)
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(transparent_black())
            .track_focus(&focus)
            .focus_ring(cx)
            .child(
                canvas(
                    move |bounds, window, cx| {
                        if *measured.read(cx) != bounds.size.width {
                            measured.update(cx, |width, cx| {
                                *width = bounds.size.width;
                                cx.notify();
                            });
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .children(tiles)
            .on_key_down(move |event, window, cx| {
                let held = &event.keystroke.modifiers;
                if held.platform || held.control {
                    return;
                }
                let key = event.keystroke.key.as_str();
                if key == "enter" {
                    if let Some(at) = at {
                        cx.stop_propagation();
                        open(at, window, cx);
                    }
                    return;
                }
                let to = match at {
                    Some(at) => stepped(at, count, columns, key),
                    None if key == "end" => count.checked_sub(1),
                    None => stepped(0, count, columns, "home")
                        .filter(|_| matches!(key, "left" | "right" | "up" | "down" | "home")),
                };
                if let Some(to) = to {
                    cx.stop_propagation();
                    pick(to, window, cx);
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::stepped;

    #[test]
    fn keys_move_across_by_one_and_down_by_a_row() {
        assert_eq!(stepped(2, 7, 3, "right"), Some(3));
        assert_eq!(stepped(0, 7, 3, "left"), None);
        assert_eq!(stepped(1, 7, 3, "down"), Some(4));
        assert_eq!(stepped(4, 7, 3, "down"), Some(6), "onto the short last row");
        assert_eq!(stepped(6, 7, 3, "down"), None);
        assert_eq!(stepped(5, 7, 3, "up"), Some(2));
        assert_eq!(stepped(1, 7, 3, "up"), None);
        assert_eq!(stepped(3, 7, 3, "end"), Some(6));
        assert_eq!(stepped(0, 0, 3, "home"), None);
    }
}
