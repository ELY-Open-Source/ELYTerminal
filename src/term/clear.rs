//! Clear: the history goes, and off Windows the rows above the prompt.

use alacritty_terminal::{
    event::EventListener,
    grid::{Dimensions, Scroll},
    term::{Term, TermMode},
    vte::ansi::{ClearMode, Handler},
};
use gpui::Context;

use super::{click::Shell, prompts, view::TerminalView};

/// Drops the history and the screen's rows above `top`; the lines dropped, none on the alternate screen.
fn clear<T: EventListener>(term: &mut Term<T>, top: usize) -> Option<usize> {
    if term.mode().contains(TermMode::ALT_SCREEN) {
        return None;
    }
    let dropped = term.history_size() + top;
    if top > 0 {
        term.scroll_up(top);
        term.grid_mut().cursor.point.line -= top;
    }
    term.clear_screen(ClearMode::Saved);
    term.scroll_display(Scroll::Bottom);
    term.selection = None;
    Some(dropped)
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
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use alacritty_terminal::{
        event::VoidListener,
        index::{Column, Line},
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
        let mut term = parsed(b"1\r\n2\r\n3\r\n4\r\n5\r\n6\r\nnow");
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
