//! A prompt its shell draws again keeps its rows unreflowed through a resize, as kitty keeps them.

use alacritty_terminal::{
    grid::{Dimensions, GridCell, Row},
    index::{Column, Line},
    term::{
        Term, TermMode,
        cell::{Cell, Flags},
    },
};

use super::{part, redraw};

/// The prompt's rows taken out before a resize, and how many of them lay above the cursor.
pub(crate) struct Kept {
    rows: Vec<Row<Cell>>,
    above: i32,
}

/// Takes out the rows of the prompt the cursor is in, when its shell draws it again, so the resize reflows the rest alone.
pub(crate) fn keep<T>(term: &mut Term<T>, columns: usize) -> Option<Kept> {
    if columns == term.columns() || term.mode().contains(TermMode::ALT_SCREEN) {
        return None;
    }
    let cursor = term.grid().cursor.point;
    let ours = redraw(&term.grid().cursor.template)?;
    // Up from the cursor to the nearest prompt it is in; a secondary prompt has its own.
    let mut first = None;
    for line in (0..=cursor.line.0).rev() {
        let (mut current, mut other) = (false, false);
        for cell in &term.grid()[Line(line)] {
            if part(cell).is_some() {
                let same = redraw(cell) == Some(ours);
                (current, other) = (current || same, other || !same);
            }
        }
        if current {
            first = Some(line);
        } else if other {
            break;
        }
    }
    let first = first?;
    // A prompt starting above the screen reflows, as in kitty.
    let above = Line(first - 1);
    if first == 0
        && term.topmost_line() <= above
        && term.grid()[above]
            .into_iter()
            .any(|cell| redraw(cell) == Some(ours))
    {
        return None;
    }
    // Output before the prompt on its first row would be cut.
    let row = &term.grid()[Line(first)];
    if row
        .into_iter()
        .take_while(|cell| redraw(cell) != Some(ours))
        .any(|cell| !cell.is_empty())
    {
        return None;
    }
    let last = term.bottommost_line().0;
    let top = term.topmost_line().0;
    let grid = term.grid_mut();
    // A soft wrap into the prompt joins output to it.
    if first > top {
        let edge = grid.columns() - 1;
        grid[Line(first - 1)][Column(edge)]
            .flags
            .remove(Flags::WRAPLINE);
    }
    let rows = (first..=last)
        .map(|line| {
            let row = grid[Line(line)].clone();
            grid[Line(line)].reset(&Cell::default());
            row
        })
        .collect();
    grid.cursor.point.column = Column(cursor.column.0.min(columns - 1));
    grid.cursor.input_needs_wrap = false;
    log::debug!(
        "prompts: {} rows of prompt r{ours} kept from the reflow",
        last - first + 1
    );
    Some(Kept {
        rows,
        above: cursor.line.0 - first,
    })
}

/// Puts the kept rows back where they stood from the cursor, cut or padded to the new width.
pub(crate) fn restore<T>(term: &mut Term<T>, kept: Kept) {
    let (top, bottom) = (term.topmost_line().0, term.bottommost_line().0);
    let columns = term.columns();
    let grid = term.grid_mut();
    let start = grid.cursor.point.line.0 - kept.above;
    for (line, row) in (start..).zip(kept.rows) {
        if line > bottom {
            break;
        }
        if line < top {
            continue;
        }
        let target = &mut grid[Line(line)];
        target.reset(&Cell::default());
        for column in 0..columns.min(row.len()) {
            let mut cell = row[Column(column)].clone();
            cell.flags.remove(Flags::WRAPLINE);
            // A wide character whose spacer the cut took goes whole.
            if column + 1 == columns && cell.flags.contains(Flags::WIDE_CHAR) {
                cell = Cell::default();
            }
            target[Column(column)] = cell;
        }
    }
}

#[cfg(test)]
mod tests;
