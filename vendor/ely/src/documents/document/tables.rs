//! Tables change by rows and columns (MARK-06).

use gpui::Context;
use markdown::mdast::{self, AlignKind};
use unicode_width::UnicodeWidthStr;

use super::{
    MarkdownDocument,
    block::{self, BlockCommand, eol, line_end, line_start, next_line, prev_line, settled},
    format::{Change, EditRefusal},
};
use crate::documents::{
    MarkdownMode,
    markdown::{file_tree, span, walk},
};

/// What a table edit does at the caret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableEdit {
    RowAbove,
    RowBelow,
    ColumnLeft,
    ColumnRight,
    DeleteRow,
    DeleteColumn,
    MoveRowUp,
    MoveRowDown,
    MoveColumnLeft,
    MoveColumnRight,
    /// Every cell padded to its column, as Typora tidies.
    Tidy,
    /// The whole table.
    Delete,
}

/// Why a table edit did not happen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableRefusal {
    /// No table at the caret.
    Outside,
    /// The head row and the last column stay.
    Kept,
    /// The caret is in a cell past the head's columns.
    Beyond,
    /// Its lines carry different quote or list marks.
    Mixed,
    /// Nothing lies past the caret's row or column that way.
    Edge,
}

/// The table around `at` after `edit`, and the caret.
pub(super) fn edit_table(text: &str, at: usize, edit: TableEdit) -> Result<Change, TableRefusal> {
    let root = file_tree(text);
    let mut nodes = Vec::new();
    walk(&root, &mut nodes);
    let (range, table) = nodes
        .into_iter()
        .find_map(|node| match node {
            mdast::Node::Table(table) if around(node, at) => Some((span(node), table)),
            _ => None,
        })
        .ok_or(TableRefusal::Outside)?;
    let prefix = &text[line_start(text, range.start)..range.start];
    let mut line = range.start;
    while let Some(next) = next_line(text, line).filter(|next| *next < range.end) {
        if !text[next..].starts_with(prefix) {
            return Err(TableRefusal::Mixed);
        }
        line = next;
    }
    let eol = eol(&text[range.clone()]);
    let rows: Vec<&mdast::Node> = table.children.iter().collect();
    let row = rows
        .iter()
        .rposition(|row| span(row).start <= at)
        .unwrap_or(0);
    let cells_of = |row: &mdast::Node| row.children().expect("a row holds cells").clone();
    let cells = cells_of(rows[row]);
    let column = cells
        .iter()
        .rposition(|cell| span(cell).start <= at)
        .unwrap_or(0);
    let offset = cells.get(column).map_or(0, |cell| within(cell, at));
    let width = table.align.len();
    // Cells past the head stay as written.
    let mut grid: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            let mut cells: Vec<String> = cells_of(row)
                .iter()
                .map(|cell| written(text, cell))
                .collect();
            if cells.len() < width {
                cells.resize(width, String::new());
            }
            cells
        })
        .collect();
    let mut aligns = table.align.clone();
    let column_edit = matches!(
        edit,
        TableEdit::ColumnLeft
            | TableEdit::ColumnRight
            | TableEdit::DeleteColumn
            | TableEdit::MoveColumnLeft
            | TableEdit::MoveColumnRight
    );
    if column_edit && column >= width {
        return Err(TableRefusal::Beyond);
    }
    let caret = match edit {
        TableEdit::RowAbove | TableEdit::RowBelow => {
            let below = edit == TableEdit::RowBelow || row == 0;
            let at = if below { row + 1 } else { row };
            grid.insert(at, vec![String::new(); width]);
            (at, column.min(width - 1), 0)
        }
        TableEdit::DeleteRow if row > 0 => {
            grid.remove(row);
            (row.min(grid.len() - 1), column.min(width - 1), 0)
        }
        TableEdit::ColumnLeft | TableEdit::ColumnRight => {
            let at = column + usize::from(edit == TableEdit::ColumnRight);
            grid.iter_mut()
                .for_each(|cells| cells.insert(at, String::new()));
            aligns.insert(at, AlignKind::None);
            (row, at, 0)
        }
        TableEdit::DeleteColumn if width > 1 => {
            grid.iter_mut().for_each(|cells| {
                cells.remove(column);
            });
            aligns.remove(column);
            (row, column.min(width - 2), 0)
        }
        TableEdit::MoveRowUp | TableEdit::MoveRowDown => {
            let to = match edit {
                TableEdit::MoveRowUp => row.checked_sub(1),
                _ => Some(row + 1).filter(|to| *to < grid.len()),
            }
            .ok_or(TableRefusal::Edge)?;
            // GFM reads a head of other width as no table.
            if row.min(to) == 0 && grid[row.max(to)].len() > width {
                return Err(TableRefusal::Beyond);
            }
            grid.swap(row, to);
            (to, column, offset)
        }
        TableEdit::MoveColumnLeft | TableEdit::MoveColumnRight => {
            let to = match edit {
                TableEdit::MoveColumnLeft => column.checked_sub(1),
                _ => Some(column + 1).filter(|to| *to < width),
            }
            .ok_or(TableRefusal::Edge)?;
            grid.iter_mut().for_each(|cells| cells.swap(column, to));
            aligns.swap(column, to);
            (row, to, offset)
        }
        TableEdit::Tidy => (row, column, offset),
        _ => return Err(TableRefusal::Kept),
    };
    let (out, caret) = table_text(&grid, &aligns, caret, prefix, eol);
    let caret = range.start + caret;
    Ok((vec![(range, out)], caret..caret))
}

/// Removes the table at the caret, as Delete does.
pub(super) fn delete(text: &str, at: usize) -> Result<Change, EditRefusal> {
    let root = file_tree(text);
    let mut nodes = Vec::new();
    walk(&root, &mut nodes);
    if !nodes.iter().any(|node| around(node, at)) {
        return Err(EditRefusal::Cells(TableRefusal::Outside));
    }
    block::run(text, at..at, BlockCommand::Delete)
}

/// Whether `node` is a table holding `at`.
fn around(node: &mdast::Node, at: usize) -> bool {
    matches!(node, mdast::Node::Table(_)) && (span(node).start..=span(node).end).contains(&at)
}

/// A cell's source between its pipes.
fn written(text: &str, cell: &mdast::Node) -> String {
    let inside = cell.children().expect("a cell holds inlines");
    match (inside.first(), inside.last()) {
        (Some(first), Some(last)) => text[span(first).start..span(last).end].to_string(),
        _ => String::new(),
    }
}

/// `at`'s byte in `cell`'s written text.
fn within(cell: &mdast::Node, at: usize) -> usize {
    let inside = cell.children().expect("a cell holds inlines");
    match (inside.first(), inside.last()) {
        (Some(first), Some(last)) => {
            let start = span(first).start;
            at.clamp(start, span(last).end) - start
        }
        _ => 0,
    }
}

/// Display columns, a tab four and controls none, as Typora's.
fn width(cell: &str) -> usize {
    let tabs = cell.matches('\t').count();
    let shown: usize = cell
        .split(char::is_control)
        .map(UnicodeWidthStr::width)
        .sum();
    shown + 4 * tabs
}

/// The table's text and the caret at row, column, byte.
fn table_text(
    grid: &[Vec<String>],
    aligns: &[AlignKind],
    at: (usize, usize, usize),
    prefix: &str,
    eol: &str,
) -> (String, usize) {
    // Typora pads columns 4 to 60 display columns wide.
    let widths: Vec<usize> = (0..aligns.len())
        .map(|column| {
            (grid.iter())
                .map(|cells| width(&cells[column]))
                .fold(4, usize::max)
                .min(60)
        })
        .collect();
    let mut out = String::new();
    let mut caret = 0;
    for (row, cells) in grid.iter().enumerate() {
        out.push('|');
        for (column, cell) in cells.iter().enumerate() {
            let room = widths
                .get(column)
                .map_or(0, |widest| widest.saturating_sub(width(cell)));
            let before = match aligns.get(column) {
                Some(AlignKind::Right) => room,
                Some(AlignKind::Center) => room / 2,
                _ => 0,
            };
            out.push(' ');
            out.push_str(&" ".repeat(before));
            if (row, column) == (at.0, at.1) {
                caret = out.len() + at.2;
            }
            out.push_str(cell);
            out.push_str(&" ".repeat(room - before));
            out.push_str(" |");
        }
        if row == 0 {
            out.push_str(eol);
            out.push_str(prefix);
            out.push('|');
            for (align, width) in aligns.iter().zip(&widths) {
                let dashes = match align {
                    AlignKind::Left => format!(":{}", "-".repeat(width - 1)),
                    AlignKind::Right => format!("{}:", "-".repeat(width - 1)),
                    AlignKind::Center => format!(":{}:", "-".repeat(width - 2)),
                    AlignKind::None => "-".repeat(*width),
                };
                out.push_str(&format!(" {dashes} |"));
            }
        }
        if row + 1 < grid.len() {
            out.push_str(eol);
            out.push_str(prefix);
        }
    }
    (out, caret)
}

impl MarkdownDocument {
    /// A new table after the caret's line, unless in preview.
    pub fn insert_table(&mut self, cx: &mut Context<Self>) -> Result<(), EditRefusal> {
        if self.mode == MarkdownMode::Preview {
            return Err(EditRefusal::Preview);
        }
        self.commit(cx);
        let text = self.editor.read(cx).text().to_string();
        let at = settled(&text, self.editor.read(cx).primary().range().start);
        let (start, end, eol) = (line_start(&text, at), line_end(&text, at), eol(&text));
        // A blank line gives way; a written one keeps.
        let blank = text[start..end].trim().is_empty();
        let above = prev_line(&text, start).map_or("", |line| &text[line..line_end(&text, line)]);
        let (from, before) = match (blank, above.trim().is_empty()) {
            (true, true) => (start, String::new()),
            (true, false) => (start, eol.to_string()),
            (false, _) => (end, eol.repeat(2)),
        };
        let after = if text[end..].starts_with(&eol.repeat(2)) || end == text.len() {
            ""
        } else {
            eol
        };
        let grid = vec![vec![String::new(); 3]; 3];
        let (table, caret) = table_text(&grid, &[AlignKind::None; 3], (0, 0, 0), "", eol);
        let caret = from + before.len() + caret;
        log::info!("markdown document: a table goes in");
        self.editor.update(cx, |editor, cx| {
            editor.edit([(from..end, format!("{before}{table}{after}"))], cx);
            editor.select(std::iter::once(caret..caret), cx);
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{TableEdit, TableRefusal, edit_table};

    const TABLE: &str = "Intro\n\n| A | B |\n|:--|--:|\n| 1 | 2 |\n| 3 | 4 |\n\nAfter\n";

    /// `text` after `edit` at `at`, the caret as `‸`.
    fn edited_in(text: &str, at: &str, edit: TableEdit) -> Result<String, TableRefusal> {
        let at = text.find(at).expect("in the table");
        let (edits, caret) = edit_table(text, at, edit)?;
        let (range, table) = &edits[0];
        let out = format!("{}{table}{}", &text[..range.start], &text[range.end..]);
        Ok(format!("{}‸{}", &out[..caret.start], &out[caret.start..]))
    }

    fn edited(at: &str, edit: TableEdit) -> Result<String, TableRefusal> {
        edited_in(TABLE, at, edit)
    }

    fn around(table: &str) -> Result<String, TableRefusal> {
        Ok(format!("Intro\n\n{table}\n\nAfter\n"))
    }

    #[test]
    fn rows_and_columns_come_and_go() {
        assert_eq!(
            edited("3", TableEdit::RowAbove),
            around(
                "| A    |    B |\n| :--- | ---: |\n| 1    |    2 |\n| ‸     |      |\n| 3    |    4 |"
            )
        );
        assert_eq!(
            edited("A", TableEdit::RowBelow),
            around(
                "| A    |    B |\n| :--- | ---: |\n| ‸     |      |\n| 1    |    2 |\n| 3    |    4 |"
            )
        );
        assert_eq!(
            edited("2", TableEdit::ColumnLeft),
            around(
                "| A    |      |    B |\n| :--- | ---- | ---: |\n| 1    | ‸     |    2 |\n| 3    |      |    4 |"
            )
        );
        assert_eq!(
            edited("1", TableEdit::DeleteRow),
            around("| A    |    B |\n| :--- | ---: |\n| ‸3    |    4 |")
        );
        assert_eq!(
            edited("B", TableEdit::DeleteColumn),
            around("| ‸A    |\n| :--- |\n| 1    |\n| 3    |")
        );
    }

    #[test]
    fn rows_and_columns_move_with_the_caret() {
        assert_eq!(
            edited("3", TableEdit::MoveRowUp),
            around("| A    |    B |\n| :--- | ---: |\n| ‸3    |    4 |\n| 1    |    2 |")
        );
        assert_eq!(
            edited("A", TableEdit::MoveRowDown),
            around("| 1    |    2 |\n| :--- | ---: |\n| ‸A    |    B |\n| 3    |    4 |")
        );
        assert_eq!(
            edited("2", TableEdit::MoveColumnLeft),
            around("|    B | A    |\n| ---: | :--- |\n|    ‸2 | 1    |\n|    4 | 3    |")
        );
        assert_eq!(edited("A", TableEdit::MoveRowUp), Err(TableRefusal::Edge));
        assert_eq!(edited("3", TableEdit::MoveRowDown), Err(TableRefusal::Edge));
        assert_eq!(
            edited("1", TableEdit::MoveColumnLeft),
            Err(TableRefusal::Edge)
        );
        assert_eq!(
            edited("4", TableEdit::MoveColumnRight),
            Err(TableRefusal::Edge)
        );
    }

    #[test]
    fn tidy_pads_by_display_width_and_keeps_the_caret() {
        let text = "| 名前 | x |\n|:-:|-|\n| 長い名前です | yy‸z |\n";
        let at = text.find('‸').unwrap();
        let text = text.replace('‸', "");
        let (edits, caret) = edit_table(&text, at, TableEdit::Tidy).unwrap();
        let out = format!(
            "{}‸{}",
            &edits[0].1[..caret.start],
            &edits[0].1[caret.start..]
        );
        assert_eq!(
            out,
            "|     名前     | x    |\n| :----------: | ---- |\n| 長い名前です | yy‸z  |"
        );
    }

    #[test]
    fn a_rewrite_keeps_the_tables_line_ending() {
        let crlf = "Before\r\n\r\n| A | B |\r\n| - | - |\r\n| 1 | 2 |\r\n\r\nAfter\r\n";
        assert_eq!(
            edited_in(crlf, "1", TableEdit::Tidy),
            Ok("Before\r\n\r\n| A    | B    |\r\n| ---- | ---- |\r\n| ‸1    | 2    |\r\n\r\nAfter\r\n".into())
        );
        let cr = "Before\r\r| A | B |\r| - | - |\r| 1 | 2 |\r\rAfter\r";
        assert_eq!(
            edited_in(cr, "1", TableEdit::Tidy),
            Ok("Before\r\r| A    | B    |\r| ---- | ---- |\r| ‸1    | 2    |\r\rAfter\r".into())
        );
        let moved = "| A | B |\r\n| - | - |\r\n| 1 | 2 |\r\n";
        assert_eq!(
            edited_in(moved, "1", TableEdit::MoveRowUp),
            Ok("| ‸1    | 2    |\r\n| ---- | ---- |\r\n| A    | B    |\r\n".into())
        );
        let quoted = "> | A |\r> | - |\r> | 1 |\r";
        assert_eq!(
            edited_in(quoted, "1", TableEdit::RowBelow),
            Ok("> | A    |\r> | ---- |\r> | 1    |\r> | ‸     |\r".into())
        );
    }

    #[test]
    fn a_tab_counts_four_columns_as_in_typora() {
        let text = "| a\tb | x |\n| - | - |\n| q | r |\n";
        assert_eq!(
            edited_in(text, "q", TableEdit::Tidy),
            Ok("| a\tb | x    |\n| ------ | ---- |\n| ‸q      | r    |\n".into())
        );
    }

    #[test]
    fn the_head_stays_and_outside_is_no_table() {
        assert_eq!(edited("A", TableEdit::DeleteRow), Err(TableRefusal::Kept));
        assert_eq!(
            edit_table(TABLE, 1, TableEdit::RowBelow).err(),
            Some(TableRefusal::Outside)
        );
        let single = "| A |\n| - |\n| 1 |\n";
        assert_eq!(
            edit_table(single, 2, TableEdit::DeleteColumn).err(),
            Some(TableRefusal::Kept)
        );
    }

    #[test]
    fn cells_past_the_head_stay_and_take_no_column_edit() {
        let text = "| A | B |\n| - | - |\n| 1 | 2 | 3 |\n";
        assert_eq!(
            edited_in(text, "3", TableEdit::DeleteColumn),
            Err(TableRefusal::Beyond)
        );
        assert_eq!(
            edited_in(text, "3", TableEdit::MoveColumnLeft),
            Err(TableRefusal::Beyond)
        );
        assert_eq!(
            edited_in(text, "1", TableEdit::MoveRowUp),
            Err(TableRefusal::Beyond),
            "a head wider than its delimiter is no table"
        );
        assert_eq!(
            edited_in(text, "1", TableEdit::RowBelow),
            Ok("| A    | B    |\n| ---- | ---- |\n| 1    | 2    | 3 |\n| ‸     |      |\n".into())
        );
    }

    #[test]
    fn a_quoted_table_keeps_its_marks() {
        let text = "> | A | B |\n> | - | - |\n> | 1 | 2 |\n";
        assert_eq!(
            edited_in(text, "1", TableEdit::RowBelow),
            Ok(
                "> | A    | B    |\n> | ---- | ---- |\n> | 1    | 2    |\n> | ‸     |      |\n"
                    .into()
            )
        );
        let mixed = "- | A |\n  | - |\n  | 1 |\n";
        assert_eq!(
            edited_in(mixed, "1", TableEdit::RowBelow),
            Err(TableRefusal::Mixed)
        );
    }
}
