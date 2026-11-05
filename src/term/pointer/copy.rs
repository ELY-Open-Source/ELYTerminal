use alacritty_terminal::{
    grid::Dimensions,
    index::{Column, Line, Point as Cell, Side},
    selection::{Selection, SelectionType},
    term::{Term, TermMode},
};
use gpui::{ClipboardItem, Context};

use super::super::{keys, view::TerminalView};
use crate::settings::SettingsStore;

impl TerminalView {
    /// The selected text without alacritty's final newline, as Ghostty copies it.
    pub(super) fn selected(&self) -> Option<String> {
        let mut text = self.pty.term.lock().selection_to_string()?;
        if text.ends_with('\n') {
            text.pop();
        }
        (!text.is_empty()).then_some(text)
    }

    /// Selects all text; copy on select takes it.
    pub(in crate::term) fn select_all(&mut self, cx: &mut Context<Self>) {
        {
            let mut term = self.pty.term.lock();
            let Some(all) = everything(&*term) else {
                log::debug!("terminal {}: nothing to select", self.pty.pid);
                return;
            };
            term.selection = Some(all);
        }
        let selected = self.selected();
        let count = selected.as_ref().map_or(0, |text| text.chars().count());
        log::debug!("terminal {}: all {count} characters selected", self.pty.pid);
        if let Some(text) = selected.filter(|_| SettingsStore::get(cx).copy_on_select) {
            self.copy_text(text, true, cx);
        }
        cx.notify();
    }

    /// Windows Terminal's right press: copy and clear a selection, else paste.
    pub(super) fn right_click(&mut self, cx: &mut Context<Self>) {
        match self.selected() {
            Some(text) => {
                self.copy_text(text, false, cx);
                self.pty.term.lock().selection = None;
                cx.notify();
            }
            None => self.paste(cx),
        }
    }

    /// The selection to the clipboard.
    pub(in crate::term) fn copy(&mut self, cx: &mut Context<Self>) {
        match self.selected() {
            Some(text) => self.copy_text(text, false, cx),
            None => log::debug!("terminal {}: nothing selected to copy", self.pty.pid),
        }
    }

    /// Text to the clipboard; `primary` text goes to Linux's selection, as Ghostty puts it.
    pub(in crate::term) fn copy_text(&self, text: String, primary: bool, cx: &mut Context<Self>) {
        let count = text.chars().count();
        let item = ClipboardItem::new_string(text);
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        if primary {
            log::info!(
                "terminal {}: copied {count} characters to the selection",
                self.pty.pid
            );
            return cx.write_to_primary(item);
        }
        log::info!(
            "terminal {}: copied {count} characters{}",
            self.pty.pid,
            if primary { ", no primary here" } else { "" }
        );
        cx.write_to_clipboard(item);
    }

    /// The clipboard's text, bracketed if asked; under 5522, its types.
    pub(in crate::term) fn paste(&mut self, cx: &mut Context<Self>) {
        if self.pty.term.lock().mode().contains(TermMode::PASTE_EVENTS) {
            return self.paste_event(cx);
        }
        match cx.read_from_clipboard().and_then(|item| item.text()) {
            Some(text) => self.paste_text(&text, cx),
            None if cfg!(windows)
                && self
                    .pty
                    .term
                    .lock()
                    .mode()
                    .contains(TermMode::BRACKETED_PASTE) =>
            {
                self.paste_text("", cx);
            }
            None => log::debug!("terminal {}: no text to paste", self.pty.pid),
        }
    }

    /// `text` to the program as a paste, bracketed when it asked.
    pub(in crate::term) fn paste_text(&mut self, text: &str, cx: &mut Context<Self>) {
        let bracketed = self
            .pty
            .term
            .lock()
            .mode()
            .contains(TermMode::BRACKETED_PASTE);
        log::info!(
            "terminal {}: pastes {} characters{}",
            self.pty.pid,
            text.chars().count(),
            if bracketed { ", bracketed" } else { "" }
        );
        self.input(keys::paste(text, bracketed), cx);
    }
}

/// Ghostty's select all: first to last non-blank cell.
fn everything<T>(term: &Term<T>) -> Option<Selection> {
    let grid = term.grid();
    let cells = (grid.topmost_line().0..=grid.bottommost_line().0).flat_map(|line| {
        (0..grid.columns()).map(move |column| Cell::new(Line(line), Column(column)))
    });
    let inked = |point: &Cell| {
        let cell = &grid[*point];
        let marked = cell.zerowidth().is_some_and(|marks| !marks.is_empty());
        marked || !matches!(cell.c, ' ' | '\t')
    };
    let first = cells.clone().find(inked)?;
    let last = cells.rev().find(inked)?;
    let mut all = Selection::new(SelectionType::Simple, first, Side::Left);
    all.update(last, Side::Right);
    Some(all)
}

#[cfg(test)]
mod tests {
    use alacritty_terminal::{
        event::VoidListener,
        term::{Config, test::TermSize},
        vte::ansi::Processor,
    };

    use super::*;

    /// What select all copies after `bytes`.
    fn all(bytes: &[u8]) -> Option<String> {
        let mut term = Term::new(Config::default(), &TermSize::new(10, 3), VoidListener);
        let mut parser: Processor = Processor::new();
        parser.advance(&mut term, bytes);
        term.selection = everything(&term);
        term.selection_to_string()
    }

    #[test]
    fn everything_runs_from_the_first_character_in_the_history_to_the_last() {
        assert_eq!(all(b""), None, "a blank grid selects nothing");
        let text = all(b"\r\n  one\r\n\r\ntwo  six\t\r\n\r\n");
        assert_eq!(text.as_deref(), Some("one\n\ntwo  six"));
        let mark = all(" \u{301}".as_bytes());
        assert_eq!(mark.as_deref(), Some(" \u{301}"), "a mark on a space");
        let last = all("text \u{301}".as_bytes());
        assert_eq!(last.as_deref(), Some("text \u{301}"), "a mark ends it");
        let link = all(b"\x1b]8;;http://a\x07 \x1b]8;;\x07text");
        assert_eq!(link.as_deref(), Some("text"), "a linked space is blank");
    }
}
