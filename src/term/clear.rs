//! Clear: the history goes, and off Windows the rows above the prompt.

use alacritty_terminal::{
    event::EventListener,
    grid::{Dimensions, Row, Scroll},
    index::{Column, Line},
    term::{Term, TermMode, cell::Cell},
    vte::ansi::{ClearMode, Handler},
};
use gpui::Context;

use super::{click::Shell, graphics::PLACEHOLDER, prompts, view::TerminalView};

/// Drops the history and the screen's rows above `top`, its images too; the lines dropped, none on the alternate screen.
fn clear<T: EventListener>(term: &mut Term<T>, top: usize) -> Option<usize> {
    if term.mode().contains(TermMode::ALT_SCREEN) {
        return None;
    }
    let dropped = term.history_size() + top;
    let lines = term.screen_lines();
    // A program's scroll region would bend a scroll, so the kept rows are carried.
    let kept: Vec<Row<Cell>> = (top..lines)
        .map(|line| imageless(term.grid()[Line(line as i32)].clone()))
        .collect();
    term.clear_screen(ClearMode::All);
    term.clear_screen(ClearMode::Saved);
    let grid = term.grid_mut();
    for (line, row) in kept.into_iter().enumerate() {
        grid[Line(line as i32)] = row;
    }
    grid.cursor.point.line -= top;
    term.scroll_display(Scroll::Bottom);
    Some(dropped)
}

/// `row` with its image placeholders blank, as ED 2 would leave them.
fn imageless(mut row: Row<Cell>) -> Row<Cell> {
    for column in 0..row.len() {
        if row[Column(column)].c == PLACEHOLDER {
            row[Column(column)] = Cell::default();
        }
    }
    row
}

/// The screen row Clear keeps first: the prompt the shell reads at, else the cursor's.
fn top<T>(term: &Term<T>, reading: bool) -> usize {
    let cursor = term.grid().cursor.point.line.0;
    let first = reading
        .then(|| prompts::start(term, cursor))
        .flatten()
        .unwrap_or(cursor);
    first.max(0) as usize
}

impl TerminalView {
    /// Clears the history and, off Windows, where ConPTY keeps the screen, the rows above the prompt.
    pub(super) fn clear(&mut self, cx: &mut Context<Self>) {
        let mut term = self.pty.term.lock();
        let top = if cfg!(windows) {
            0
        } else {
            top(&term, self.shell != Shell::Busy)
        };
        let dropped = clear(&mut term, top);
        drop(term);
        match dropped {
            Some(lines) => log::info!("terminal {}: cleared, {lines} lines gone", self.pty.pid),
            None => log::debug!(
                "terminal {}: the alternate screen keeps no history to clear",
                self.pty.pid
            ),
        }
        if self.finder.is_some() {
            self.search(false, cx);
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use alacritty_terminal::{
        event::VoidListener,
        term::{Config, test::TermSize},
        vte::ansi::Processor,
    };

    use super::*;
    use crate::term::prompts::Prompts;

    fn feed(term: &mut Term<VoidListener>, bytes: &[u8]) {
        let mut parser: Processor = Processor::new();
        parser.advance(term, Prompts::default().pass(bytes));
    }

    fn parsed(bytes: &[u8]) -> Term<VoidListener> {
        let mut term = Term::new(Config::default(), &TermSize::new(20, 4), VoidListener);
        feed(&mut term, bytes);
        term
    }

    fn row(term: &Term<VoidListener>, line: i32) -> String {
        let text: String = (0..20)
            .map(|column| term.grid()[Line(line)][Column(column)].c)
            .collect();
        text.trim_end().to_string()
    }

    #[test]
    fn a_running_program_keeps_the_cursor_s_row_alone() {
        let mut term = parsed("1\r\n2\r\n3\r\n4\r\n5\r\n6\r\nnow\u{10EEEE}\x08".as_bytes());
        assert_eq!((term.history_size(), top(&term, false)), (3, 3));
        assert_eq!(clear(&mut term, 3), Some(6));
        assert_eq!(term.history_size(), 0);
        assert_eq!(
            (row(&term, 0), row(&term, 1)),
            ("now".into(), String::new())
        );
        let cursor = term.grid().cursor.point;
        assert_eq!((cursor.line.0, cursor.column.0), (0, 3));
    }

    #[test]
    fn a_program_s_scroll_region_bends_nothing() {
        let mut term = parsed(b"a\r\nb\r\nc\r\nd\x1b[2;4r\x1b[4;1Hnow");
        assert_eq!(clear(&mut term, 3), Some(3));
        assert_eq!(
            (row(&term, 0), row(&term, 1)),
            ("now".into(), String::new())
        );
        feed(&mut term, b"\r\n1\r\n2\r\n3\r\n4");
        assert_eq!(row(&term, 0), "now", "the region still holds the first row");
    }

    #[test]
    fn a_shell_that_reads_keeps_its_whole_prompt() {
        let mut term = parsed(b"out\r\n\x1b]133;A\x07one\r\ntwo> \x1b]133;B\x07ls");
        assert_eq!((top(&term, true), top(&term, false)), (1, 2));
        assert_eq!(clear(&mut term, 1), Some(1));
        assert_eq!(
            (row(&term, 0), row(&term, 1)),
            ("one".into(), "two> ls".into())
        );
        assert_eq!(term.grid().cursor.point.line.0, 1);
    }

    #[test]
    fn the_alternate_screen_stays_as_it_is() {
        let mut term = parsed(b"1\r\n2\r\n3\r\n4\r\n5\r\n\x1b[?1049hfull");
        assert_eq!(clear(&mut term, 0), None);
        assert_eq!(row(&term, 3), "full");
        feed(&mut term, b"\x1b[?1049l");
        assert_eq!((term.history_size(), row(&term, 3)), (2, String::new()));
    }
}
