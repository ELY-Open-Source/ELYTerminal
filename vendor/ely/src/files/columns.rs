use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, ScrollHandle, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use super::{FileIcon, OnEntry, explorer::OnPath};
use crate::{
    forms::{OnValue, reveal, revealer},
    layout::on_axis,
    lists::{DirEntry, ListItem, SelectableList},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize, Radius},
};

/// The columns' scroll, and the deepest column it last brought into view.
#[derive(Default)]
struct Walk {
    scroll: ScrollHandle,
    revealed: Option<usize>,
}

/// Each folder along the path as a column, the next folder on the path lit in it and the pick lit in the last. A pick of a folder goes into it; a pick of a file closes the columns past its own and picks it; Enter or a double press opens a file. A new deepest column scrolls into view.
pub(super) fn columns(
    id: ElementId,
    path: Rc<[SharedString]>,
    levels: Vec<Vec<DirEntry>>,
    selected: Option<SharedString>,
    (go, select, open): (OnPath, OnValue, OnEntry),
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let walk = window.use_keyed_state((id.clone(), "walk"), cx, |_, _| Walk::default());
    let last = levels.len() - 1;
    let shown = reveal(&walk, |walk| &mut walk.revealed, true, last, cx);
    let scroll = walk.read(cx).scroll.clone();
    let theme = cx.theme();
    let colors = &theme.colors;
    let sizes = theme.files();
    let levels = Rc::new(levels);
    let columns: Vec<_> = levels
        .iter()
        .enumerate()
        .map(|(depth, entries)| {
            let folder = path[..=depth].join("/");
            let rows = entries.iter().enumerate().fold(
                SelectableList::new((id.clone(), format!("column-{folder}"))),
                |list, (ix, entry)| {
                    let row = ListItem::new(
                        (id.clone(), format!("row-{folder}-{ix}")),
                        entry.name().clone(),
                    )
                    .leading(FileIcon::entry(entry))
                    .when(entry.is_folder(), |row| {
                        row.trailing(
                            Icon::new(IconName::ChevronRight)
                                .size(IconSize::Xs)
                                .color(colors.fg_subtle),
                        )
                    });
                    list.row(entry.name().clone(), row)
                },
            );
            let lit = match depth < last {
                true => Some(path[depth + 1].clone()).filter(|name| {
                    entries
                        .iter()
                        .any(|entry| entry.is_folder() && entry.name() == name)
                }),
                false => selected.clone(),
            };
            let (go, select, path, picked) =
                (go.clone(), select.clone(), path.clone(), levels.clone());
            let (open, opened) = (open.clone(), levels.clone());
            div()
                .flex_none()
                .w(sizes.column)
                .h_full()
                .p_1()
                .border_r_1()
                .border_color(colors.border)
                .child(
                    rows.selected(lit)
                        .size_full()
                        .on_change(move |keys, window, cx| {
                            let Some(name) = keys.first() else {
                                return;
                            };
                            let entry = picked[depth]
                                .iter()
                                .find(|entry| entry.name() == name)
                                .expect("a listed entry");
                            if entry.is_folder() {
                                let inside: Vec<SharedString> = path[..=depth]
                                    .iter()
                                    .cloned()
                                    .chain([name.clone()])
                                    .collect();
                                return go(&inside, window, cx);
                            }
                            if depth < last {
                                go(&path[..=depth], window, cx);
                            }
                            select(name, window, cx);
                        })
                        .on_activate(move |name, window, cx| {
                            let entry = opened[depth]
                                .iter()
                                .find(|entry| entry.name() == name)
                                .expect("a listed entry");
                            if !entry.is_folder() {
                                open(entry, window, cx);
                            }
                        }),
                )
        })
        .collect();
    on_axis(
        div()
            .id(id)
            .debug_selector(|| "file-columns".into())
            .h(sizes.columns)
            .flex()
            .overflow_x_scroll()
            .track_scroll(&scroll)
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.border)
            .children(columns)
            .when_some(shown, |row, (ix, done)| {
                row.child(revealer(&scroll, ix, done))
            }),
    )
}
