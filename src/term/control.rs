//! What another program reads of a terminal and types into it, through the control door.

use alacritty_terminal::{
    grid::Dimensions,
    index::{Column, Line},
    term::{Term, cell::Flags},
};
use gpui::{Context, Keystroke};

use super::view::TerminalView;

/// The last `count` rows of `term` as text, the blank rows after the last line left out.
fn tail<T>(term: &Term<T>, count: usize) -> String {
    let grid = term.grid();
    let mut rows = Vec::new();
    for line in (grid.topmost_line().0..=grid.bottommost_line().0).rev() {
        if rows.len() == count {
            break;
        }
        let mut text = String::new();
        for column in 0..grid.columns() {
            let cell = &grid[Line(line)][Column(column)];
            if !cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                text.push(cell.c);
                text.extend(cell.zerowidth().into_iter().flatten());
            }
        }
        let text = text.trim_end();
        if !rows.is_empty() || !text.is_empty() {
            rows.push(text.to_string());
        }
    }
    rows.reverse();
    rows.join("\n")
}

impl TerminalView {
    /// Its last `count` rows, its screen's without a count.
    pub fn tail(&self, count: Option<usize>) -> String {
        let term = self.pty.term.lock();
        tail(&term, count.unwrap_or(term.screen_lines()))
    }

    /// Types `text` and Enter, as the keyboard would.
    pub fn typed(&mut self, text: &str, cx: &mut Context<Self>) {
        log::info!(
            "terminal {}: another program types {} bytes",
            self.pty.pid,
            text.len()
        );
        let mut bytes = text.as_bytes().to_vec();
        bytes.push(b'\r');
        self.input(bytes, cx);
    }

    /// Presses each of `keys`, named as gpui names them; a key that types text is refused before any is pressed.
    pub fn keyed(&mut self, keys: &[String], cx: &mut Context<Self>) -> anyhow::Result<()> {
        let mut strokes = Vec::new();
        for key in keys {
            let stroke = Keystroke::parse(key).map_err(|_| anyhow::anyhow!("{key} is no key"))?;
            let bare =
                !stroke.modifiers.control && !stroke.modifiers.alt && !stroke.modifiers.platform;
            anyhow::ensure!(
                !(bare && stroke.key.chars().count() == 1),
                "{key} types text: use send for text"
            );
            strokes.push(stroke);
        }
        log::info!(
            "terminal {}: another program presses {keys:?}",
            self.pty.pid
        );
        for (key, stroke) in keys.iter().zip(&strokes) {
            anyhow::ensure!(
                self.press(stroke, cx),
                "{key} names no key the terminal sends"
            );
        }
        Ok(())
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

    #[test]
    fn the_tail_is_the_last_rows_without_the_blank_end() {
        let mut term = Term::new(Config::default(), &TermSize::new(10, 4), VoidListener);
        let mut parser: Processor = Processor::new();
        parser.advance(&mut term, "1\r\n2\r\n\r\n四 x\r\n5\r\n$ ".as_bytes());
        assert_eq!(
            tail(&term, 4),
            "\n四 x\n5\n$",
            "a wide character once, an inner blank row kept"
        );
        assert_eq!(tail(&term, 2), "5\n$");
        assert_eq!(tail(&term, 99), "1\n2\n\n四 x\n5\n$", "history too");
        parser.advance(&mut term, b"\x1b[2J");
        assert_eq!(
            tail(&term, 1),
            "$",
            "ED 2 keeps what it scrolled into the history"
        );
    }
}
