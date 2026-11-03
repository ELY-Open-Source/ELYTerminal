//! A table cell opens alone, as Typora edits tables.

use std::ops::Range;

use markdown::mdast::Node;

use crate::documents::{
    markdown::{file_tree, span, walk},
    render::CellAt,
};

/// A cell: which, its source, its caret.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Cell {
    pub at: CellAt,
    pub range: Range<usize>,
    pub caret: usize,
}

/// What names a cell in a block.
#[derive(Clone, Copy, Debug)]
pub(super) enum Aim {
    /// The cell holding this byte of the text.
    Byte(usize),
    /// This cell, its caret at the end.
    At(CellAt),
    First,
    Last,
}

/// A way out of a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Move {
    Next,
    Previous,
    Up,
    Down,
}

/// The rows of each table in `root`, in drawing order.
fn tables(root: &Node) -> Vec<Vec<&Node>> {
    let mut nodes = Vec::new();
    walk(root, &mut nodes);
    nodes
        .into_iter()
        .filter(|node| matches!(node, Node::Table(_)))
        .map(|table| {
            table
                .children()
                .expect("a table holds rows")
                .iter()
                .collect()
        })
        .collect()
}

fn cells(row: &Node) -> &[Node] {
    row.children().expect("a row holds cells")
}

/// The cell `aim` names in `block` of `text`.
pub(super) fn find(text: &str, block: Range<usize>, aim: Aim) -> Option<Cell> {
    let root = file_tree(&text[block.clone()]);
    let tables = tables(&root);
    let at = match aim {
        Aim::At(at) => at,
        Aim::First if !tables.is_empty() => CellAt {
            table: 0,
            row: 0,
            column: 0,
        },
        Aim::Last => {
            let table = tables.len().checked_sub(1)?;
            let row = tables[table].len() - 1;
            let column = cells(tables[table][row]).len().checked_sub(1)?;
            CellAt { table, row, column }
        }
        Aim::First => return None,
        Aim::Byte(byte) => {
            let byte = byte.checked_sub(block.start)?;
            let table = (tables.iter()).position(|rows| {
                let (first, last) = (rows[0], rows[rows.len() - 1]);
                (span(first).start..=span(last).end).contains(&byte)
            })?;
            let rows = &tables[table];
            let row = rows.iter().rposition(|row| span(row).start <= byte)?;
            let column = (cells(rows[row]).iter()).rposition(|cell| span(cell).start <= byte)?;
            CellAt { table, row, column }
        }
    };
    let cell = cells(tables.get(at.table)?.get(at.row)?).get(at.column)?;
    let range = written(&text[block.clone()], cell);
    let caret = match aim {
        Aim::Byte(byte) => (byte - block.start).clamp(range.start, range.end) - range.start,
        _ => range.len(),
    };
    Some(Cell {
        at,
        range: block.start + range.start..block.start + range.end,
        caret,
    })
}

/// The cell `by` reaches from `at`; none past the edge.
pub(super) fn beside(text: &str, block: Range<usize>, at: CellAt, by: Move) -> Option<CellAt> {
    let root = file_tree(&text[block]);
    let tables = tables(&root);
    let rows = tables.get(at.table)?;
    let width = |row: usize| rows.get(row).map_or(0, |row| cells(row).len());
    let (row, column) = match by {
        Move::Next if at.column + 1 < width(at.row) => (at.row, at.column + 1),
        Move::Next => (at.row + 1, 0),
        Move::Previous if at.column > 0 => (at.row, at.column - 1),
        Move::Previous => {
            let row = at.row.checked_sub(1)?;
            (row, width(row).checked_sub(1)?)
        }
        Move::Up => (at.row.checked_sub(1)?, at.column),
        Move::Down => (at.row + 1, at.column),
    };
    let column = column.min(width(row).checked_sub(1)?);
    Some(CellAt {
        table: at.table,
        row,
        column,
    })
}

/// A cell's source; empty, just past its opening pipe.
fn written(text: &str, cell: &Node) -> Range<usize> {
    let inside = cell.children().expect("a cell holds inlines");
    if let (Some(first), Some(last)) = (inside.first(), inside.last()) {
        return span(first).start..span(last).end;
    }
    let span = span(cell);
    let mut at = span.start;
    if text[at..span.end].starts_with('|') {
        at += 1;
    }
    if text[at..span.end].starts_with([' ', '\t']) {
        at += 1;
    }
    at..at
}

/// `typed` with each bare `|` escaped.
fn guarded(typed: &str) -> String {
    let mut out = String::with_capacity(typed.len());
    let mut slashes = 0;
    for ch in typed.chars() {
        if ch == '|' && slashes % 2 == 0 {
            out.push('\\');
        }
        slashes = if ch == '\\' { slashes + 1 } else { 0 };
        out.push(ch);
    }
    out
}

/// What borders a cell's source in its row.
#[derive(Clone, Copy, Debug)]
pub(super) struct Edges {
    /// A pipe follows at once.
    closed: bool,
    /// No pipe before it on its line.
    first: bool,
    /// No pipe after it on its line.
    last: bool,
}

/// The borders of the cell source at `range` of `text`.
pub(super) fn edges(text: &str, range: Range<usize>) -> Edges {
    let line = text[..range.start].rfind('\n').map_or(0, |at| at + 1);
    let end = (text[range.end..].find('\n')).map_or(text.len(), |at| range.end + at);
    let (before, after) = (&text[line..range.start], &text[range.end..end]);
    Edges {
        closed: after.starts_with('|'),
        first: !before.contains('|'),
        last: !after.contains('|'),
    }
}

/// `typed` as a cell keeps it, its row's pipes intact.
pub(super) fn kept(typed: &str, edges: Edges) -> String {
    let mut out = guarded(typed);
    let slashes = out.len() - out.trim_end_matches('\\').len();
    if edges.closed && slashes % 2 == 1 {
        out.push('\\');
    }
    // An emptied bare edge cell keeps its column.
    if out.is_empty() && (edges.first || edges.last) {
        out.push('|');
    }
    out
}

/// Byte `at` of `typed` within `kept(typed, edges)`.
pub(super) fn kept_at(typed: &str, at: usize, edges: Edges) -> usize {
    match at == typed.len() {
        true if typed.is_empty() && !edges.first => 0,
        true => kept(typed, edges).len(),
        false => guarded(&typed[..at]).len(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Aim, CellAt, Edges, Move, beside, edges, find, guarded, kept, kept_at};

    const TABLE: &str = "intro\n\n| a | **b** |\n| --- | --- |\n| c |  |\n| d | e |\n";

    fn block() -> std::ops::Range<usize> {
        7..TABLE.len() - 1
    }

    fn at(row: usize, column: usize) -> CellAt {
        CellAt {
            table: 0,
            row,
            column,
        }
    }

    #[test]
    fn a_byte_opens_its_cell_with_the_caret_there() {
        let byte = TABLE.find("**b**").expect("in the table") + 2;
        let cell = find(TABLE, block(), Aim::Byte(byte)).expect("a cell");
        assert_eq!(cell.at, at(0, 1));
        assert_eq!(&TABLE[cell.range.clone()], "**b**");
        assert_eq!(cell.caret, 2);
    }

    #[test]
    fn an_empty_cell_opens_between_its_pipes() {
        let cell = find(TABLE, block(), Aim::At(at(1, 1))).expect("a cell");
        assert!(cell.range.is_empty());
        let written = [&TABLE[..cell.range.start], "x", &TABLE[cell.range.end..]].concat();
        assert!(written.contains("| c | x |"), "{written:?}");
    }

    #[test]
    fn ends_open_the_first_and_last_cells() {
        let first = find(TABLE, block(), Aim::First).expect("a cell");
        assert_eq!((first.at, &TABLE[first.range]), (at(0, 0), "a"));
        let last = find(TABLE, block(), Aim::Last).expect("a cell");
        assert_eq!((last.at, &TABLE[last.range]), (at(2, 1), "e"));
        assert_eq!(find("plain\n", 0..5, Aim::First), None);
    }

    #[test]
    fn moves_wrap_rows_and_stop_at_edges() {
        let to = |from, by| beside(TABLE, block(), from, by);
        assert_eq!(to(at(0, 1), Move::Next), Some(at(1, 0)));
        assert_eq!(to(at(1, 0), Move::Previous), Some(at(0, 1)));
        assert_eq!(to(at(0, 0), Move::Previous), None);
        assert_eq!(to(at(2, 1), Move::Next), None);
        assert_eq!(to(at(1, 1), Move::Up), Some(at(0, 1)));
        assert_eq!(to(at(2, 0), Move::Down), None);
    }

    #[test]
    fn bare_pipes_escape_and_escaped_stay() {
        assert_eq!(guarded("a|b"), "a\\|b");
        assert_eq!(guarded("a\\|b"), "a\\|b");
        assert_eq!(guarded("a\\\\|b"), "a\\\\\\|b");
    }

    /// Cell `(row, column)` of `text`'s one table, written as `typed`.
    fn write(text: &str, row: usize, column: usize, typed: &str) -> String {
        let block = 0..text.len() - 1;
        let cell = find(text, block, Aim::At(at(row, column))).expect("a cell");
        let kept = kept(typed, edges(text, cell.range.clone()));
        [&text[..cell.range.start], &kept, &text[cell.range.end..]].concat()
    }

    /// The source of each cell of `text`'s one table.
    fn grid(text: &str) -> Vec<Vec<String>> {
        let block = 0..text.len() - 1;
        (0..)
            .map_while(|row| {
                let cells: Vec<String> = (0..)
                    .map_while(|column| {
                        let cell = find(text, block.clone(), Aim::At(at(row, column)))?;
                        Some(text[cell.range].to_string())
                    })
                    .collect();
                (!cells.is_empty()).then_some(cells)
            })
            .collect()
    }

    #[test]
    fn a_closing_backslash_keeps_the_next_pipe() {
        let shut = Edges {
            closed: true,
            first: false,
            last: false,
        };
        assert_eq!(kept("a\\", shut), "a\\\\");
        assert_eq!(kept("a\\\\", shut), "a\\\\");
        assert_eq!(kept_at("a\\", 2, shut), 3);
        assert_eq!(kept_at("a\\b", 2, shut), 2);
        let written = write("|a|b|\n|---|---|\n|c|d|\n", 0, 0, "a\\");
        assert_eq!(grid(&written)[0], ["a\\\\", "b"]);
    }

    #[test]
    fn a_backslash_before_padding_stays_single() {
        let text = "| a\\ | b |\n| --- | --- |\n";
        assert_eq!(write(text, 0, 0, "a\\"), text);
    }

    #[test]
    fn an_emptied_bare_edge_cell_keeps_its_column() {
        let text = "alpha | beta\n--- | ---\nc | d\n";
        let rows = |written: &str| grid(written).iter().map(Vec::len).collect::<Vec<_>>();
        for (row, column) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
            let written = write(text, row, column, "");
            assert_eq!(rows(&written), [2, 2], "{written:?}");
            assert_eq!(grid(&written)[row][column], "", "{written:?}");
        }
        assert_eq!(grid(&write(text, 1, 0, ""))[1], ["", "d"]);
        let empty = edges(text, 0..5);
        assert_eq!(kept_at("", 0, empty), 1, "past its new pipe");
        let last = edges(text, 8..12);
        assert_eq!(kept_at("", 0, last), 0, "before its new pipe");
    }

    #[test]
    fn typed_pipes_shift_later_bytes() {
        let open = Edges {
            closed: false,
            first: false,
            last: false,
        };
        assert_eq!(kept_at("||x", 2, open), 4);
    }
}
