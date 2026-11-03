use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, Styled, Window, div, prelude::*,
};

use super::{Cell, Column, body::sized, cell::draw};
use crate::{
    layout::seeded::use_seeded,
    primitives::Disclosure,
    theme::{ActiveTheme, Density, TextSize},
};

/// One row of a tree table: its key, a cell for each column, and the rows under it.
#[derive(Clone, Debug, PartialEq)]
pub struct TreeRow {
    key: SharedString,
    cells: Vec<Cell>,
    children: Vec<TreeRow>,
}

impl TreeRow {
    pub fn new(key: impl Into<SharedString>, cells: impl IntoIterator<Item = Cell>) -> Self {
        Self {
            key: key.into(),
            cells: cells.into_iter().collect(),
            children: Vec::new(),
        }
    }

    pub fn children(mut self, rows: impl IntoIterator<Item = TreeRow>) -> Self {
        self.children.extend(rows);
        self
    }
}

/// Rows in view, depth first through the open ones, each with its depth.
fn flat<'a>(
    rows: &'a [TreeRow],
    open: &[SharedString],
    depth: usize,
    out: &mut Vec<(&'a TreeRow, usize)>,
) {
    for row in rows {
        out.push((row, depth));
        if open.contains(&row.key) {
            flat(&row.children, open, depth + 1, out);
        }
    }
}

/// Rows that open and close under table columns; the first column steps in by depth and carries each row's disclosure.
#[derive(IntoElement)]
pub struct TreeTable {
    id: ElementId,
    columns: Vec<Column>,
    rows: Vec<TreeRow>,
    open: Vec<SharedString>,
}

impl TreeTable {
    pub fn new(
        id: impl Into<ElementId>,
        columns: impl IntoIterator<Item = Column>,
        rows: impl IntoIterator<Item = TreeRow>,
    ) -> Self {
        Self {
            id: id.into(),
            columns: columns.into_iter().collect(),
            rows: rows.into_iter().collect(),
            open: Vec::new(),
        }
    }

    /// The rows open at first; a new list from the owner replaces what was opened since.
    pub fn open(mut self, keys: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.open = keys.into_iter().map(Into::into).collect();
        self
    }
}

impl RenderOnce for TreeTable {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let opened = use_seeded((self.id.clone(), "open"), self.open, window, cx);
        let open = opened.read(cx).value.clone();
        let mut shown = Vec::new();
        flat(&self.rows, &open, 0, &mut shown);
        let theme = cx.theme();
        let colors = &theme.colors;
        let (height, narrowest, indent) = (
            theme.table_row(Density::Standard),
            theme.label_width() * 0.5,
            theme.tree_indent(),
        );
        let columns = Rc::new(self.columns);
        let head = div()
            .flex()
            .items_center()
            .h(height)
            .border_b_1()
            .border_color(colors.border)
            .text_size(theme.text_size(TextSize::Xs))
            .font_weight(FontWeight::MEDIUM)
            .text_color(colors.fg_muted)
            .children(columns.iter().map(|column| {
                sized(div().px_3().flex(), column, narrowest).child(column.title.clone())
            }));
        let rows = shown.into_iter().map(|(row, depth)| {
            assert_eq!(
                row.cells.len(),
                columns.len(),
                "row {} needs a cell per column",
                row.key
            );
            let (toggle, key, is_open) = (opened.clone(), row.key.clone(), open.contains(&row.key));
            let first = div()
                .flex()
                .items_center()
                .gap_1()
                .min_w_0()
                .child(div().flex_none().w(indent * depth as f32))
                .child(
                    div()
                        .id((self.id.clone(), format!("open-{}", row.key)))
                        .flex_none()
                        .w(indent)
                        .flex()
                        .justify_center()
                        .when(!row.children.is_empty(), |slot| {
                            slot.cursor_pointer()
                                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                    toggle.update(cx, |seeded, cx| {
                                        if is_open {
                                            seeded.value.retain(|known| *known != key);
                                        } else {
                                            seeded.value.push(key.clone());
                                        }
                                        cx.notify();
                                    })
                                })
                                .child(Disclosure::new(
                                    (self.id.clone(), format!("chevron-{}", row.key)),
                                    is_open,
                                ))
                        }),
                )
                .child(draw(
                    &row.cells[0],
                    &columns[0],
                    (self.id.clone(), format!("cell-{}-0", row.key)).into(),
                    cx,
                ));
            let mut first = Some(first);
            div()
                .id((self.id.clone(), format!("row-{}", row.key)))
                .flex()
                .items_center()
                .h(height)
                .border_b_1()
                .border_color(colors.border)
                .hover(|style| style.bg(colors.hover))
                .text_size(theme.text_size(TextSize::Sm))
                .children(columns.iter().enumerate().map(|(col, column)| {
                    let cell = sized(
                        div().h_full().flex().items_center().px_3(),
                        column,
                        narrowest,
                    );
                    if col == 0 {
                        cell.children(first.take())
                    } else {
                        cell.child(draw(
                            &row.cells[col],
                            column,
                            (self.id.clone(), format!("cell-{}-{col}", row.key)).into(),
                            cx,
                        ))
                    }
                }))
        });
        div().flex().flex_col().child(head).children(rows)
    }
}
