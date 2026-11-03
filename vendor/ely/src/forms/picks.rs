use std::{ops::Range, rc::Rc};

use gpui::{
    AnyElement, App, Context, ElementId, Entity, InteractiveElement, IntoElement, ParentElement,
    Rems, ScrollStrategy, SharedString, StatefulInteractiveElement, Styled,
    UniformListScrollHandle, Window, div, prelude::*, uniform_list,
};

use super::{
    Input, TextInput,
    text::{Down, Enter},
};
use crate::{
    primitives::{Icon, IconName, Tooltip, tab_stop},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
};

/// A row of a glyph grid: a group's title, or a run of glyphs.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Row {
    Title(SharedString),
    Glyphs(Range<usize>),
}

/// Rows for glyph groups given as (title, count), laid in order, `columns` across.
pub(crate) fn rows(
    groups: impl IntoIterator<Item = (Option<SharedString>, usize)>,
    columns: usize,
) -> Vec<Row> {
    let (mut out, mut start) = (Vec::new(), 0);
    for (title, count) in groups.into_iter().filter(|(_, count)| *count > 0) {
        out.extend(title.map(Row::Title));
        let end = start + count;
        while start < end {
            let stop = (start + columns).min(end);
            out.push(Row::Glyphs(start..stop));
            start = stop;
        }
    }
    out
}

fn row_of(rows: &[Row], ix: usize) -> usize {
    rows.iter()
        .position(|row| matches!(row, Row::Glyphs(range) if range.contains(&ix)))
        .expect("every glyph sits in a row")
}

/// The glyph an arrow key moves to from `at`: same column in the next row, or the last one there.
pub(crate) fn stepped(rows: &[Row], at: usize, key: &str) -> Option<usize> {
    let row = row_of(rows, at);
    let Row::Glyphs(here) = &rows[row] else {
        unreachable!("row_of finds glyph rows");
    };
    let column = at - here.start;
    let land = |row: &Row| match row {
        Row::Glyphs(range) => Some((range.start + column).min(range.end - 1)),
        Row::Title(_) => None,
    };
    let last = rows
        .iter()
        .rev()
        .find_map(|row| match row {
            Row::Glyphs(range) => Some(range.end - 1),
            Row::Title(_) => None,
        })
        .expect("a grid with a cursor has glyphs");
    match key {
        "left" => at.checked_sub(1),
        "right" => (at < last).then_some(at + 1),
        "up" => rows[..row].iter().rev().find_map(land),
        "down" => rows[row + 1..].iter().find_map(land),
        _ => None,
    }
}

/// A grid's search field, its cursor, and where its list has scrolled.
pub(crate) struct Finder {
    search: Entity<TextInput>,
    pub query: String,
    cursor: usize,
    scroll: UniformListScrollHandle,
}

pub(crate) fn finder(
    id: &ElementId,
    hint: &'static str,
    window: &mut Window,
    cx: &mut App,
) -> Entity<Finder> {
    let finder = window.use_keyed_state(
        (id.clone(), "finder"),
        cx,
        |window, cx: &mut Context<Finder>| Finder {
            search: cx.new(|cx| TextInput::new(window, cx).placeholder(hint)),
            query: String::new(),
            cursor: 0,
            scroll: UniformListScrollHandle::new(),
        },
    );
    let query = {
        let search = finder.read(cx).search.clone();
        search.read(cx).text().trim().to_lowercase()
    };
    if finder.read(cx).query != query {
        finder.update(cx, |finder, _| {
            finder.query = query;
            finder.cursor = 0;
            finder.scroll.scroll_to_item(0, ScrollStrategy::Top);
        });
    }
    finder
}

pub(crate) type Pick = Rc<dyn Fn(usize, &mut Window, &mut App)>;
pub(crate) type Draw = Rc<dyn Fn(usize, &App) -> AnyElement>;

/// What a glyph grid shows: names for tooltips, rows, a painter, what a pick does, and its cells.
pub(crate) struct Grid {
    pub id: ElementId,
    pub names: Rc<Vec<SharedString>>,
    pub rows: Rc<Vec<Row>>,
    pub selected: Option<usize>,
    pub draw: Draw,
    pub pick: Pick,
    pub none: &'static str,
    pub columns: usize,
    pub cell: Rems,
}

pub(crate) fn grid(
    grid: Grid,
    finder: &Entity<Finder>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let focus = tab_stop((grid.id.clone(), "grid").into(), true, window, cx);
    let focused = focus.is_focused(window);
    let count = grid.names.len();
    let (search, cursor, scroll) = {
        let finder = finder.read(cx);
        let cursor = finder.cursor.min(count.saturating_sub(1));
        (finder.search.clone(), cursor, finder.scroll.clone())
    };
    let theme = cx.theme();
    let colors = theme.colors.clone();
    let (cell, columns) = (grid.cell, grid.columns);
    let (into, enter, keys) = (focus.clone(), grid.pick.clone(), grid.pick.clone());
    let field = div()
        .capture_action(move |_: &Down, window, cx| {
            if count > 0 {
                cx.stop_propagation();
                window.focus(&into, cx);
            }
        })
        .capture_action(move |_: &Enter, window, cx| {
            if count > 0 {
                cx.stop_propagation();
                enter(cursor, window, cx);
            }
        })
        .child(
            Input::new(&search).prefix(
                Icon::new(IconName::Search)
                    .size(IconSize::Sm)
                    .color(colors.fg_subtle),
            ),
        );
    let body = if count == 0 {
        div()
            .h(theme.list_max_height())
            .pt_6()
            .text_center()
            .text_color(colors.fg_subtle)
            .child(grid.none)
            .into_any_element()
    } else {
        let (rows, names, draw, pick) = (grid.rows.clone(), grid.names, grid.draw, grid.pick);
        let selected = grid.selected;
        let (moved, key_rows, place) = (finder.clone(), grid.rows, scroll.clone());
        let clicked = finder.clone();
        let list = uniform_list((grid.id, "rows"), rows.len(), move |range, _, cx| {
            let theme = cx.theme();
            rows[range]
                .iter()
                .map(|row| match row {
                    Row::Title(title) => div()
                        .h(cell)
                        .flex()
                        .items_end()
                        .pb_1()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(colors.fg_subtle)
                        .child(title.clone())
                        .into_any_element(),
                    Row::Glyphs(glyphs) => div()
                        .flex()
                        .children(glyphs.clone().map(|ix| {
                            let (pick, name, clicked) =
                                (pick.clone(), names[ix].clone(), clicked.clone());
                            let ring = if focused && ix == cursor {
                                colors.focus
                            } else if selected == Some(ix) {
                                colors.accent
                            } else {
                                gpui::transparent_black()
                            };
                            div()
                                .id(("glyph", ix))
                                .flex()
                                .items_center()
                                .justify_center()
                                .size(cell)
                                .rounded(theme.radius(Radius::Md))
                                .border_1()
                                .border_color(ring)
                                .when(selected == Some(ix), |cell| cell.bg(colors.active))
                                .cursor_pointer()
                                .hover(|style| style.bg(colors.hover))
                                .tooltip(Tooltip::text(name))
                                .on_click(move |_, window, cx| {
                                    clicked.update(cx, |finder, cx| {
                                        finder.cursor = ix;
                                        cx.notify();
                                    });
                                    pick(ix, window, cx);
                                })
                                .child(draw(ix, cx))
                        }))
                        .into_any_element(),
                })
                .collect()
        })
        .track_scroll(&scroll)
        .h(theme.list_max_height());
        div()
            .track_focus(&focus)
            .on_key_down(move |event, window, cx| {
                let key = event.keystroke.key.as_str();
                if matches!(key, "enter" | "space") {
                    cx.stop_propagation();
                    keys(cursor, window, cx);
                    return;
                }
                let Some(to) = stepped(&key_rows, cursor, key) else {
                    return;
                };
                cx.stop_propagation();
                let strategy = if to < cursor {
                    ScrollStrategy::Top
                } else {
                    ScrollStrategy::Bottom
                };
                place.scroll_to_item(row_of(&key_rows, to), strategy);
                moved.update(cx, |finder, cx| {
                    finder.cursor = to;
                    cx.notify();
                });
            })
            .child(list)
            .into_any_element()
    };
    div()
        .flex()
        .flex_col()
        .gap_2()
        .w(cell * columns as f32)
        .child(field)
        .child(body)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{Row, rows, stepped};

    #[test]
    fn groups_break_into_titled_rows_of_eight() {
        let laid = rows(
            [(Some("A".into()), 10), (None, 0), (Some("B".into()), 3)],
            8,
        );
        assert_eq!(
            laid,
            [
                Row::Title("A".into()),
                Row::Glyphs(0..8),
                Row::Glyphs(8..10),
                Row::Title("B".into()),
                Row::Glyphs(10..13),
            ]
        );
    }

    #[test]
    fn arrows_keep_the_column_and_cross_titles() {
        let laid = rows([(Some("A".into()), 10), (Some("B".into()), 3)], 8);
        assert_eq!(stepped(&laid, 6, "down"), Some(9));
        assert_eq!(stepped(&laid, 9, "down"), Some(11));
        assert_eq!(stepped(&laid, 11, "up"), Some(9));
        assert_eq!(stepped(&laid, 1, "up"), None);
        assert_eq!(stepped(&laid, 12, "right"), None);
        assert_eq!(stepped(&laid, 0, "left"), None);
        assert_eq!(stepped(&laid, 8, "left"), Some(7));
    }
}
