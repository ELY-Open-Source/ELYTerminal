use std::cell::RefCell;

use gpui::{
    AnyElement, App, FontWeight, IntoElement, MouseButton, ParentElement, Styled, div, prelude::*,
};
use markdown::mdast::AlignKind;

use super::Draw;
use crate::{
    documents::tree::Node,
    theme::{ActiveTheme, Radius},
};

/// A cell: table in drawing order, row, column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CellAt {
    pub table: usize,
    pub row: usize,
    pub column: usize,
}

/// The cell open for typing and its field, drawn once.
pub(crate) struct OpenCell {
    pub at: CellAt,
    pub field: RefCell<Option<AnyElement>>,
}

impl Draw<'_> {
    pub(super) fn table(
        &self,
        aligns: &[AlignKind],
        head: &[Vec<Node>],
        rows: &[Vec<Vec<Node>>],
        cx: &App,
    ) -> AnyElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let table = self.tables.get();
        self.tables.set(table + 1);
        let cell = |blocks: &[Node], row: usize, column: usize, cx: &App| {
            let at = CellAt { table, row, column };
            let open = (self.hooks.cell)
                .filter(|open| open.at == at)
                .and_then(|open| open.field.borrow_mut().take());
            let drawn = div()
                .debug_selector(move || format!("markdown-cell-{table}-{row}-{column}"))
                .flex_1()
                .min_w_0()
                .flex()
                .px_3()
                .py_1p5()
                .when(aligns.get(column) == Some(&AlignKind::Right), |cell| {
                    cell.justify_end()
                })
                .when(aligns.get(column) == Some(&AlignKind::Center), |cell| {
                    cell.justify_center()
                })
                .when(row == 0, |cell| {
                    cell.font_weight(FontWeight::MEDIUM)
                        .text_color(colors.fg_muted)
                });
            let inside = match open {
                // Presses on the open field stay with it.
                Some(field) => div()
                    .w_full()
                    .child(field)
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation()),
                None => div()
                    .max_w_full()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(self.nodes(blocks, cx)),
            };
            self.pressed_cell(drawn.child(inside), at)
        };
        div()
            .flex()
            .flex_col()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.border)
            .overflow_hidden()
            .child(
                div().flex().bg(colors.hover).children(
                    head.iter()
                        .enumerate()
                        .map(|(column, blocks)| cell(blocks, 0, column, cx)),
                ),
            )
            .children(rows.iter().enumerate().map(|(row, cells)| {
                div()
                    .flex()
                    .border_t_1()
                    .border_color(colors.border)
                    .children(
                        cells
                            .iter()
                            .enumerate()
                            .map(|(column, blocks)| cell(blocks, row + 1, column, cx)),
                    )
            }))
            .into_any_element()
    }
}
