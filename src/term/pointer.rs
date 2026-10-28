use alacritty_terminal::{
    grid::{Dimensions, Scroll},
    index::{Column, Line, Point as Cell, Side},
    selection::{Selection, SelectionType},
    term::{Term, TermMode},
};
use gpui::{
    ClipboardItem, Context, Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    Pixels, Point, ScrollWheelEvent, Size, Window,
};

use super::{
    keys,
    links::link_at,
    mouse::{self, Button, Gesture, MouseMode, Report, Tracking},
    view::TerminalView,
    wheel,
};
use crate::settings::SettingsStore;

/// Where the grid sits, and what the mouse is doing to it.
#[derive(Default)]
pub struct Pointer {
    /// The grid's top-left in the window, from the last layout.
    pub origin: Point<Pixels>,
    pub cell: Size<Pixels>,
    /// The left button is down and drawing a selection.
    selecting: bool,
    /// A plain press's cell, until it drags off it; its release may move the shell's cursor.
    click: Option<Cell>,
    /// Buttons whose press went to the program.
    pressed: Vec<Button>,
    /// The cell of the last report; motion inside it sends nothing.
    last: Option<(usize, usize)>,
    /// Wheel travel short of a line, in lines.
    rest: f32,
}

fn button(button: MouseButton) -> Option<Button> {
    match button {
        MouseButton::Left => Some(Button::Left),
        MouseButton::Middle => Some(Button::Middle),
        MouseButton::Right => Some(Button::Right),
        MouseButton::Navigate(_) => None,
    }
}

impl TerminalView {
    /// The screen cell under a window position, clamped to the grid; its grid point; its half.
    fn cell_at(&self, position: Point<Pixels>) -> ((usize, usize), Cell, Side) {
        let Pointer { origin, cell, .. } = self.pointer;
        let x = ((position.x - origin.x) / cell.width).max(0.);
        let y = ((position.y - origin.y) / cell.height).max(0.);
        let (columns, lines) = self.size;
        let (column, row) = ((x as usize).min(columns - 1), (y as usize).min(lines - 1));
        let side = if x as usize >= columns || x.fract() >= 0.5 {
            Side::Right
        } else {
            Side::Left
        };
        let offset = self.pty.term.lock().grid().display_offset() as i32;
        let point = Cell::new(Line(row as i32 - offset), Column(column));
        ((column, row), point, side)
    }

    /// What the program tracks; none while Shift holds the mouse for selecting.
    fn tracking(&self, modifiers: &Modifiers) -> Tracking {
        if modifiers.shift {
            Tracking::Off
        } else {
            self.mouse.tracking
        }
    }

    fn report(
        &mut self,
        button: Option<Button>,
        gesture: Gesture,
        cell: (usize, usize),
        modifiers: &Modifiers,
        tracking: Tracking,
    ) -> bool {
        let wheel = matches!(button, Some(Button::WheelUp | Button::WheelDown));
        // herdr's link key is Ctrl; on macOS Cmd stands for it, the wheel aside.
        let link = cfg!(target_os = "macos") && self.herdr && modifiers.platform && !wheel;
        let report = Report {
            button,
            gesture,
            cell,
            shift: modifiers.shift,
            alt: modifiers.alt,
            ctrl: modifiers.control || link,
        };
        let format = self.mouse.format;
        let Some(bytes) = mouse::encode(&report, MouseMode { tracking, format }) else {
            return false;
        };
        log::trace!("terminal {}: {report:?} sends {bytes:?}", self.pty.pid);
        self.pty.write(bytes);
        self.pointer.last = Some(cell);
        true
    }

    pub(super) fn mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus, cx);
        let (cell, point, side) = self.cell_at(event.position);
        let opens = if cfg!(target_os = "macos") {
            event.modifiers.platform
        } else {
            event.modifiers.control
        };
        let tracking = self.tracking(&event.modifiers);
        // herdr opens its panes' links itself, whole where its drawing wraps them.
        let herdr = self.herdr && tracking != Tracking::Off;
        if event.button == MouseButton::Left && opens && herdr {
            log::debug!(
                "terminal {}: herdr takes the link press at {cell:?}",
                self.pty.pid
            );
        } else if event.button == MouseButton::Left && opens {
            let link = link_at(&self.pty.term.lock(), point);
            if let Some(link) = link {
                log::info!("terminal {}: opens {link}", self.pty.pid);
                cx.open_url(&link);
                return;
            }
        }
        if tracking != Tracking::Off {
            let Some(pressed) = button(event.button) else {
                return;
            };
            if self.report(
                Some(pressed),
                Gesture::Press,
                cell,
                &event.modifiers,
                tracking,
            ) && !self.pointer.pressed.contains(&pressed)
            {
                self.pointer.pressed.push(pressed);
            }
            return;
        }
        if event.button != MouseButton::Left {
            return;
        }
        let plain = event.click_count == 1 && !event.modifiers.modified();
        self.pointer.click = plain.then_some(point);
        let pid = self.pty.pid;
        let mut term = self.pty.term.lock();
        match (
            term.selection.as_mut(),
            event.modifiers.shift,
            event.click_count,
        ) {
            (Some(selection), true, 1) => {
                log::debug!("terminal {pid}: the selection extends to {point:?}");
                selection.update(point, side);
            }
            _ => {
                let kind = match event.click_count {
                    2 => SelectionType::Semantic,
                    3.. => SelectionType::Lines,
                    _ => SelectionType::Simple,
                };
                log::debug!("terminal {pid}: a {kind:?} selection starts at {point:?}");
                term.selection = Some(Selection::new(kind, point, side));
            }
        }
        drop(term);
        self.pointer.selecting = true;
        cx.notify();
    }

    pub(super) fn mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (cell, point, side) = self.cell_at(event.position);
        if self.pointer.selecting && event.pressed_button == Some(MouseButton::Left) {
            if self.pointer.click.is_some_and(|click| click != point) {
                self.pointer.click = None;
            }
            if let Some(selection) = self.pty.term.lock().selection.as_mut() {
                log::trace!("terminal {}: the selection runs to {point:?}", self.pty.pid);
                selection.update(point, side);
            }
            cx.notify();
            return;
        }
        let tracking = self.tracking(&event.modifiers);
        if tracking != Tracking::Off && self.pointer.last != Some(cell) {
            let held = [Button::Left, Button::Right, Button::Middle]
                .into_iter()
                .find(|button| self.pointer.pressed.contains(button));
            self.report(held, Gesture::Motion, cell, &event.modifiers, tracking);
        }
    }

    pub(super) fn mouse_up(
        &mut self,
        event: &MouseUpEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.button == MouseButton::Left {
            self.end_selection(cx);
            let (_, point, _) = self.cell_at(event.position);
            match self.pointer.click.take() {
                Some(click) if click != point => log::debug!(
                    "terminal {}: a press at {click:?} let go at {point:?}",
                    self.pty.pid
                ),
                Some(click) if self.selected().is_none() => self.click_to_move(click, cx),
                _ => {}
            }
        }
        if let Some(released) = button(event.button) {
            let (cell, ..) = self.cell_at(event.position);
            self.release(released, cell, &event.modifiers);
        }
    }

    /// Ends the mouse's gesture as releases would: a hidden terminal hears none.
    pub fn let_go(&mut self, cx: &mut Context<Self>) {
        self.end_selection(cx);
        self.pointer.click = None;
        for held in self.pointer.pressed.clone() {
            log::debug!("terminal {}: hidden with {held:?} down", self.pty.pid);
            let cell = self
                .pointer
                .last
                .expect("a press the program heard set a cell");
            self.release(held, cell, &Modifiers::default());
        }
    }

    /// Ends a selection drag; copy on select takes its text.
    fn end_selection(&mut self, cx: &mut Context<Self>) {
        if !std::mem::take(&mut self.pointer.selecting) {
            return;
        }
        let selected = self.selected();
        let count = selected.as_ref().map_or(0, |text| text.chars().count());
        log::debug!(
            "terminal {}: the selection ends on {count} characters",
            self.pty.pid
        );
        if let Some(text) = selected.filter(|_| SettingsStore::get(cx).copy_on_select) {
            self.copy_text(text, true, cx);
        }
    }

    /// Reports `released` coming up when its press went to the program.
    fn release(&mut self, released: Button, cell: (usize, usize), modifiers: &Modifiers) {
        if self.pointer.pressed.contains(&released) {
            self.pointer.pressed.retain(|held| *held != released);
            let tracking = self.mouse.tracking;
            self.report(Some(released), Gesture::Release, cell, modifiers, tracking);
        }
    }

    pub(super) fn wheel(
        &mut self,
        event: &ScrollWheelEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let height = self.pointer.cell.height;
        if height <= Pixels::ZERO {
            return;
        }
        cx.stop_propagation();
        let rows = self.pty.term.lock().screen_lines();
        let rest = &mut self.pointer.rest;
        let Some(lines) = wheel::lines(rest, event, height, rows, wheel::settings) else {
            return;
        };
        if lines == 0 {
            return;
        }
        let shift = if event.modifiers.shift {
            " with Shift"
        } else {
            ""
        };
        log::trace!(
            "terminal {}: wheel {:?}{shift}, {lines} lines",
            self.pty.pid,
            event.delta
        );
        let (cell, ..) = self.cell_at(event.position);
        let tracking = self.tracking(&event.modifiers);
        let mode = *self.pty.term.lock().mode();
        if tracking != Tracking::Off {
            let turn = if lines > 0 {
                Button::WheelUp
            } else {
                Button::WheelDown
            };
            for _ in 0..lines.unsigned_abs() {
                self.report(Some(turn), Gesture::Press, cell, &event.modifiers, tracking);
            }
        } else if mode.contains(TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL) {
            let arrow: &[u8] = match (lines > 0, mode.contains(TermMode::APP_CURSOR)) {
                (true, true) => b"\x1bOA",
                (true, false) => b"\x1b[A",
                (false, true) => b"\x1bOB",
                (false, false) => b"\x1b[B",
            };
            self.pty.write(arrow.repeat(lines.unsigned_abs() as usize));
        } else {
            self.pty.term.lock().scroll_display(Scroll::Delta(lines));
            cx.notify();
        }
    }

    /// The selected text without alacritty's final newline, as Ghostty copies it.
    fn selected(&self) -> Option<String> {
        let mut text = self.pty.term.lock().selection_to_string()?;
        if text.ends_with('\n') {
            text.pop();
        }
        (!text.is_empty()).then_some(text)
    }

    /// Selects all text; copy on select takes it.
    pub(super) fn select_all(&mut self, cx: &mut Context<Self>) {
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

    /// The selection to the clipboard.
    pub(super) fn copy(&mut self, cx: &mut Context<Self>) {
        match self.selected() {
            Some(text) => self.copy_text(text, false, cx),
            None => log::debug!("terminal {}: nothing selected to copy", self.pty.pid),
        }
    }

    /// Text to the clipboard; `primary` text goes to Linux's selection, as Ghostty puts it.
    pub(super) fn copy_text(&self, text: String, primary: bool, cx: &mut Context<Self>) {
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

    /// The clipboard's text to the program, bracketed when it asked.
    pub(super) fn paste(&mut self, cx: &mut Context<Self>) {
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            log::debug!("terminal {}: no text to paste", self.pty.pid);
            return;
        };
        self.paste_text(&text, cx);
    }

    /// `text` to the program as a paste, bracketed when it asked.
    pub(super) fn paste_text(&mut self, text: &str, cx: &mut Context<Self>) {
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
