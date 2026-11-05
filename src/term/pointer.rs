use alacritty_terminal::{
    grid::{Dimensions, Scroll},
    index::{Column, Line, Point as Cell, Side},
    selection::{Selection, SelectionType},
    term::TermMode,
};
use gpui::{
    Context, Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point,
    ScrollDelta, ScrollWheelEvent, Size, Window,
};

use super::{
    links::link_at,
    mouse::{self, Button, Gesture, MouseMode, Report, Tracking},
    view::TerminalView,
    wheel,
};
use crate::settings::SettingsStore;

/// Where the grid sits, and what the mouse is doing to it.
#[derive(Default)]
pub struct Pointer {
    pub(super) hover: Option<Point<Pixels>>,
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
    /// Windows mouse reporting keeps fractional notches apart from local lines.
    report_rest: f32,
}

impl Pointer {
    pub(super) fn hovered(
        &self,
        position: Point<Pixels>,
        size: (usize, usize),
    ) -> Option<(usize, usize)> {
        let delta = position - self.origin;
        if self.cell.width <= Pixels::ZERO
            || self.cell.height <= Pixels::ZERO
            || delta.x < Pixels::ZERO
            || delta.y < Pixels::ZERO
        {
            return None;
        }
        let column = (delta.x / self.cell.width) as usize;
        let row = (delta.y / self.cell.height) as usize;
        (column < size.0 && row < size.1).then_some((column, row))
    }
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
        if event.button == MouseButton::Right && SettingsStore::get(cx).right_click_paste {
            return self.right_click(cx);
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
        let previous = self
            .pointer
            .hover
            .and_then(|position| self.pointer.hovered(position, self.size));
        self.pointer.hover = Some(event.position);
        if previous != self.pointer.hovered(event.position, self.size) {
            cx.notify();
        }
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
        self.pointer.hover = None;
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
        let tracking = self.tracking(&event.modifiers);
        // A touchpad's pixels stay lines.
        let reporting = cfg!(windows)
            && tracking != Tracking::Off
            && matches!(event.delta, ScrollDelta::Lines(_));
        let rest = if reporting {
            &mut self.pointer.report_rest
        } else {
            &mut self.pointer.rest
        };
        let Some(steps) = wheel::lines(rest, event, height, rows, reporting, wheel::settings)
        else {
            return;
        };
        if steps == 0 {
            return;
        }
        let shift = if event.modifiers.shift {
            " with Shift"
        } else {
            ""
        };
        log::trace!(
            "terminal {}: wheel {:?}{shift}, {steps} steps",
            self.pty.pid,
            event.delta
        );
        let (cell, ..) = self.cell_at(event.position);
        let mode = *self.pty.term.lock().mode();
        if tracking != Tracking::Off {
            let turn = if steps > 0 {
                Button::WheelUp
            } else {
                Button::WheelDown
            };
            for _ in 0..steps.unsigned_abs() {
                self.report(Some(turn), Gesture::Press, cell, &event.modifiers, tracking);
            }
        } else if mode.contains(TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL) {
            let arrow: &[u8] = match (steps > 0, mode.contains(TermMode::APP_CURSOR)) {
                (true, true) => b"\x1bOA",
                (true, false) => b"\x1b[A",
                (false, true) => b"\x1bOB",
                (false, false) => b"\x1b[B",
            };
            self.pty.write(arrow.repeat(steps.unsigned_abs() as usize));
        } else {
            self.pty.term.lock().scroll_display(Scroll::Delta(steps));
            cx.notify();
        }
    }
}

#[cfg(test)]
mod hover_tests;

mod copy;
