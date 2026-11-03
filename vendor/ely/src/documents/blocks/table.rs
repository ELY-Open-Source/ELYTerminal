use gpui::{
    AnyElement, Context, FontWeight, InteractiveElement, IntoElement, ParentElement, Styled,
    Window, div, prelude::*,
};

use super::{Block, BlockEditor, BlockKind, view::Look};
use crate::{
    buttons::{ButtonVariant, IconButton},
    primitives::IconName,
    theme::ControlSize,
};

/// Columns, rows, and the cells folded into their left neighbor.
type Shape<'a> = (usize, usize, &'a [(usize, usize)]);

/// A table's shape; none once the block is no table.
fn shape(kind: &BlockKind, fields: usize) -> Option<Shape<'_>> {
    let BlockKind::Table { columns, merged } = kind else {
        log::error!("block editor: a {} block is no table now", kind.label());
        return None;
    };
    Some((*columns, fields / columns, merged))
}

/// How many columns the cell at `(row, column)` spans: itself and every cell folded into it.
pub(crate) fn span(merged: &[(usize, usize)], columns: usize, row: usize, column: usize) -> usize {
    1 + (column + 1..columns)
        .take_while(|next| merged.contains(&(row, *next)))
        .count()
}

/// The cell that shows for `cell`: itself, or the one a merge folds it into.
pub(crate) fn shown_cell(merged: &[(usize, usize)], columns: usize, cell: usize) -> usize {
    let (row, mut column) = (cell / columns, cell % columns);
    while column > 0 && merged.contains(&(row, column)) {
        column -= 1;
    }
    row * columns + column
}

/// Merges after `column` is removed: its own go, the ones right of it move left.
pub(crate) fn without_column(merged: &[(usize, usize)], column: usize) -> Vec<(usize, usize)> {
    merged
        .iter()
        .filter(|(_, at)| *at != column && *at != column + 1)
        .map(|&(row, at)| (row, if at > column { at - 1 } else { at }))
        .collect()
}

/// Merges after `row` is removed.
pub(crate) fn without_row(merged: &[(usize, usize)], row: usize) -> Vec<(usize, usize)> {
    merged
        .iter()
        .filter(|(at, _)| *at != row)
        .map(|&(at, column)| (if at > row { at - 1 } else { at }, column))
        .collect()
}

impl BlockEditor {
    /// Adds a row of empty cells at the table's foot.
    pub(crate) fn add_row(&mut self, key: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.index(key) else {
            return;
        };
        let Some((columns, _, _)) = shape(&self.blocks[ix].kind, self.blocks[ix].fields.len())
        else {
            return;
        };
        self.before_change(cx);
        for _ in 0..columns {
            self.add_field(key, None, window, cx);
        }
        log::info!("table {key}: a row added");
        self.after_change(cx);
    }

    /// Adds a column of empty cells at the table's right.
    pub(crate) fn add_column(&mut self, key: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.index(key) else {
            return;
        };
        let Some((columns, rows, merged)) =
            shape(&self.blocks[ix].kind, self.blocks[ix].fields.len())
        else {
            return;
        };
        let merged = merged.to_vec();
        self.before_change(cx);
        for row in (0..rows).rev() {
            self.add_field(key, Some((row + 1) * columns), window, cx);
        }
        self.blocks[ix].kind = BlockKind::Table {
            columns: columns + 1,
            merged,
        };
        log::info!("table {key}: a column added");
        self.after_change(cx);
    }

    /// Removes the row or column holding cell `cell`; a table keeps a header, a row and a column.
    pub(crate) fn remove_line(
        &mut self,
        key: u64,
        cell: usize,
        column: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(ix) = self.index(key) else {
            return;
        };
        let Some((columns, rows, merged)) =
            shape(&self.blocks[ix].kind, self.blocks[ix].fields.len())
        else {
            return;
        };
        if cell >= self.blocks[ix].fields.len() {
            log::error!("table {key}: cell {cell} left the table");
            return;
        }
        let merged = merged.to_vec();
        let (row, at) = (cell / columns, cell % columns);
        if (column && columns == 1) || (!column && rows <= 2) {
            log::info!("table {key}: its last line stays");
            return;
        }
        self.before_change(cx);
        let (gone, kind): (Vec<usize>, BlockKind) = if column {
            (
                (0..rows).map(|row| row * columns + at).collect(),
                BlockKind::Table {
                    columns: columns - 1,
                    merged: without_column(&merged, at),
                },
            )
        } else {
            (
                (row * columns..(row + 1) * columns).collect(),
                BlockKind::Table {
                    columns,
                    merged: without_row(&merged, row),
                },
            )
        };
        for ix_field in gone.into_iter().rev() {
            self.remove_field(key, ix_field);
        }
        self.blocks[ix].kind = kind;
        log::info!(
            "table {key}: {} removed",
            if column { "a column" } else { "a row" }
        );
        self.after_change(cx);
    }

    /// Folds the cell right of `cell` into it, moving its text over; or, when already merged, parts them again.
    pub(crate) fn merge_right(&mut self, key: u64, cell: usize, cx: &mut Context<Self>) {
        let Some(ix) = self.index(key) else {
            return;
        };
        let Some((columns, _, merged)) = shape(&self.blocks[ix].kind, self.blocks[ix].fields.len())
        else {
            return;
        };
        if cell >= self.blocks[ix].fields.len() {
            log::error!("table {key}: cell {cell} left the table");
            return;
        }
        let (row, column) = (cell / columns, cell % columns);
        let spanned = span(merged, columns, row, column);
        if column + spanned >= columns && spanned == 1 {
            return;
        }
        let mut merged = merged.to_vec();
        self.before_change(cx);
        if spanned > 1 {
            merged
                .retain(|(at_row, at)| !(*at_row == row && *at > column && *at < column + spanned));
            log::info!("table {key}: cell {cell} parted");
        } else {
            let right = self.blocks[ix].fields[cell + 1].read(cx).text().to_string();
            if !right.is_empty() {
                self.blocks[ix].fields[cell].update(cx, |field, cx| {
                    let end = field.text().len();
                    field.select(end..end, cx);
                    field.insert(&format!(" {right}"), cx);
                });
                self.blocks[ix].fields[cell + 1].update(cx, |field, cx| field.set_text("", cx));
            }
            merged.push((row, column + 1));
            log::info!("table {key}: cell {cell} merged right");
        }
        self.blocks[ix].kind = BlockKind::Table { columns, merged };
        self.after_change(cx);
    }
}

/// The table: a header row, then rows; cells merged across span their neighbors; tools above while a cell has focus.
pub(super) fn table(
    editor: &BlockEditor,
    block: &Block,
    look: &Look,
    window: &mut Window,
    cx: &mut Context<BlockEditor>,
) -> AnyElement {
    let (columns, rows, merged) =
        shape(&block.kind, block.fields.len()).expect("a table block draws as a table");
    let merged = merged.to_vec();
    let (key, id) = (block.key, BlockEditor::id(cx));
    let focused = editor
        .focused(window, cx)
        .filter(|(at, _)| *at == key)
        .map(|(_, cell)| cell);
    let colors = &look.colors;
    let tool = |name: &'static str,
                icon: IconName,
                run: fn(&mut BlockEditor, u64, usize, &mut Window, &mut Context<BlockEditor>),
                cell: usize,
                cx: &mut Context<BlockEditor>| {
        IconButton::new((id.clone(), format!("table-{key}-{name}")), icon)
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Sm)
            .tooltip(name)
            .on_click(cx.listener(move |editor, _, window, cx| run(editor, key, cell, window, cx)))
    };
    let tools = focused.map(|cell| {
        div()
            .flex()
            .gap_0p5()
            .child(tool(
                "Add row",
                IconName::Rows2,
                |editor, key, _, window, cx| editor.add_row(key, window, cx),
                cell,
                cx,
            ))
            .child(tool(
                "Add column",
                IconName::Columns2,
                |editor, key, _, window, cx| editor.add_column(key, window, cx),
                cell,
                cx,
            ))
            .child(tool(
                "Remove row",
                IconName::Minus,
                |editor, key, cell, _, cx| editor.remove_line(key, cell, false, cx),
                cell,
                cx,
            ))
            .child(tool(
                "Remove column",
                IconName::X,
                |editor, key, cell, _, cx| editor.remove_line(key, cell, true, cx),
                cell,
                cx,
            ))
            .child(tool(
                "Merge or part cells",
                IconName::Merge,
                |editor, key, cell, _, cx| editor.merge_right(key, cell, cx),
                cell,
                cx,
            ))
    });
    let mut grid = div()
        .flex()
        .flex_col()
        .rounded(look.radius)
        .border_1()
        .border_color(colors.border);
    for row in 0..rows {
        let mut line = div().flex().when(row > 0, |line| {
            line.border_t_1().border_color(colors.border)
        });
        if row == 0 {
            line = line.bg(colors.hover).font_weight(FontWeight::MEDIUM);
        }
        for column in 0..columns {
            if merged.contains(&(row, column)) {
                continue;
            }
            let cell = row * columns + column;
            let wide = span(&merged, columns, row, column);
            line = line.child(
                div()
                    .flex_grow_1()
                    .flex_basis(gpui::relative(wide as f32 / columns as f32))
                    .min_w_0()
                    .px_2()
                    .py_1()
                    .when(column > 0, |cell| {
                        cell.border_l_1().border_color(colors.border)
                    })
                    .child(editor.field_view(block, cell, window, cx)),
            );
        }
        grid = grid.child(line);
    }
    div()
        .id((id.clone(), format!("table-{key}")))
        .flex()
        .flex_col()
        .gap_1()
        .children(tools)
        .child(grid)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_merged_cell_spans_what_folds_into_it() {
        let merged = [(1, 1), (1, 2)];
        assert_eq!(span(&merged, 4, 1, 0), 3);
        assert_eq!(span(&merged, 4, 0, 0), 1);
        assert_eq!(span(&merged, 4, 1, 3), 1);
    }

    #[test]
    fn removing_a_line_moves_the_merges_past_it() {
        let merged = [(0, 1), (1, 3), (2, 2)];
        assert_eq!(
            without_column(&merged, 1),
            [(1, 2)],
            "a cell folded into the removed column parts"
        );
        assert_eq!(without_row(&merged, 1), [(0, 1), (1, 2)]);
    }
}
