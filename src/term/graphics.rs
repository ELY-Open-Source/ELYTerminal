//! Kitty graphics commands, as the terminal hands them over.

mod animate;
mod command;
mod draw;
mod frames;
mod load;
mod media;
mod place;
mod put;
mod relate;
mod reply;
mod store;
#[cfg(test)]
mod tests;
mod unicode;

use std::{
    ops::Range,
    sync::{Arc, Mutex, MutexGuard},
};

use alacritty_terminal::{
    index::{Column, Line, Point},
    term::{Answer, Change},
};

use command::{Command, Quiet};
pub use draw::{Layer, Pictures, Shown};
use load::Loading;
use place::{Display, Pick, Screen};
use reply::{Error, Ids, Reply};
pub use unicode::{PLACEHOLDER, runs};

/// Images and placements: the parser writes them, the view draws.
#[derive(Debug)]
pub struct Shared {
    screens: [Screen; 2],
    alt: bool,
    /// A cell in pixels, as the pty reports it.
    pub cell: (u32, u32),
    /// Columns and lines.
    size: (usize, usize),
    limit: usize,
}

impl Shared {
    fn screen(&mut self) -> &mut Screen {
        &mut self.screens[usize::from(self.alt)]
    }
}

/// One session's kitty graphics: the shared state and a transmission.
pub struct Graphics {
    pid: u32,
    shared: Arc<Mutex<Shared>>,
    loading: Option<Loading>,
    /// Where the last placement left the cursor.
    moved: Option<(usize, Column)>,
}

impl Graphics {
    pub fn new(pid: u32, size: (usize, usize), cell: (u16, u16)) -> Self {
        Self::limited(pid, size, cell, store::LIMIT)
    }

    fn limited(pid: u32, size: (usize, usize), cell: (u16, u16), limit: usize) -> Self {
        let shared = Shared {
            screens: [Screen::new(limit), Screen::new(limit)],
            alt: false,
            cell: (u32::from(cell.0), u32::from(cell.1)),
            size,
            limit,
        };
        Self {
            pid,
            shared: Arc::new(Mutex::new(shared)),
            loading: None,
            moved: None,
        }
    }

    /// The state the view draws from.
    pub fn shared(&self) -> Arc<Mutex<Shared>> {
        Arc::clone(&self.shared)
    }

    fn lock(&self) -> MutexGuard<'_, Shared> {
        self.shared.lock().expect("graphics: a holder panicked")
    }

    fn execute(&mut self, command: &Command, cursor: Point) -> Option<String> {
        let pid = self.pid;
        let ids = Ids {
            id: command.int(b'i'),
            number: command.int(b'I'),
            placement: command.int(b'p'),
        };
        let mut quiet = command.quiet;
        let reply = if ids.id > 0 && ids.number > 0 {
            Some(Reply::failed(ids, Error::Both))
        } else {
            match command.action {
                b'q' => self.query(command, ids),
                b't' | b'T' | b'f' => {
                    // Only a direct chunk goes on with a transmission.
                    let direct = command.letter(b't').is_none_or(|medium| medium == b'd');
                    if !direct && self.loading.take().is_some() {
                        log::warn!("graphics {pid}: a transmission was dropped unfinished");
                    }
                    if let Some(loading) = &mut self.loading {
                        match quiet {
                            Quiet::No => quiet = loading.quiet,
                            asked => loading.quiet = asked,
                        }
                    }
                    if self.loading.is_none() && command.action == b'f' {
                        self.frame(command, ids)
                    } else {
                        self.transmit(command, ids, cursor)
                    }
                }
                b'a' => self.control(command, ids),
                b'c' => self.compose(command, ids),
                b'p' => self.display(Display::of(command), ids, cursor),
                b'd' => {
                    self.delete(command, cursor);
                    None
                }
                other => {
                    log::warn!(
                        "graphics {pid}: action {} is none kitty names",
                        other as char
                    );
                    None
                }
            }
        };
        let reply = reply?;
        let ok = reply.message == "OK";
        if !ok {
            log::warn!("graphics {pid}: {:?} fails: {}", reply.ids, reply.message);
        }
        let shown = match quiet {
            Quiet::No => true,
            Quiet::Ok => !ok,
            Quiet::Failures => false,
        };
        shown.then(|| reply.text()).flatten()
    }

    /// Loads the image as a transmission would, and keeps nothing.
    fn query(&self, command: &Command, ids: Ids) -> Option<Reply> {
        if ids.id == 0 {
            log::warn!("graphics {}: a query names no image id", self.pid);
            return None;
        }
        let read =
            Loading::start(command, ids, self.pid).and_then(|loading| loading.finish(self.pid));
        Some(match read {
            Ok(_) => Reply::ok(ids),
            Err(error) => Reply::failed(ids, error),
        })
    }

    fn transmit(&mut self, command: &Command, ids: Ids, cursor: Point) -> Option<Reply> {
        let pid = self.pid;
        let loading = match self.loading.take() {
            Some(mut loading) => match loading.add(&command.payload, pid) {
                Ok(()) => loading,
                Err(error) => return Some(Reply::failed(loading.ids, error)),
            },
            None => {
                // A reused id drops its image as the new starts.
                if ids.id > 0 && self.lock().screen().remove(ids.id) {
                    log::debug!("graphics {pid}: image {} replaced", ids.id);
                }
                match Loading::start(command, ids, pid) {
                    Ok(mut loading) => {
                        loading.display = (command.action == b'T').then(|| Display::of(command));
                        loading
                    }
                    Err(error) => return Some(Reply::failed(ids, error)),
                }
            }
        };
        if command.more() {
            self.loading = Some(loading);
            return None;
        }
        if loading.frame.is_some() {
            return self.framed(loading);
        }
        let (ids, local, display) = (loading.ids, loading.local(), loading.display);
        let mut image = match loading.finish(pid) {
            Ok(image) => image,
            Err(error) => return Some(Reply::failed(ids, error)),
        };
        let mut shared = self.lock();
        let screen = shared.screen();
        image.id = match ids.id {
            0 => screen.store.free(ids.number > 0),
            id => id,
        };
        image.number = ids.number;
        let stored = Ids {
            id: image.id,
            ..ids
        };
        if let Err(error) = screen.store(image, pid) {
            // A local image's size tells of its file.
            let error = if local { Error::Read } else { error };
            return Some(Reply::failed(ids, error));
        }
        drop(shared);
        let shown = display.and_then(|display| self.display(display, stored, cursor));
        // An image named by neither id nor number hears nothing.
        (ids.id > 0 || ids.number > 0).then(|| shown.unwrap_or(Reply::ok(stored)))
    }

    /// Deletes what `d` names; capitals free images left unplaced.
    fn delete(&mut self, command: &Command, cursor: Point) {
        let pid = self.pid;
        if self.loading.take().is_some() {
            log::warn!("graphics {pid}: a delete dropped a transmission unfinished");
        }
        let what = command.letter(b'd').unwrap_or(b'a');
        if what.eq_ignore_ascii_case(&b'f') {
            return self.delete_frame(command, what == b'F');
        }
        let (x, y, z) = (
            command.int(b'x'),
            command.int(b'y'),
            command.int(b'z') as i32,
        );
        let (image, placement) = (command.int(b'i'), command.int(b'p'));
        // Cells count from 1, as cursor positions do.
        let at = |z| {
            (x > 0 && y > 0).then(|| Pick::Cell {
                line: y as i32 - 1,
                column: x as usize - 1,
                z,
            })
        };
        let mut shared = self.lock();
        let (cell, lines) = (shared.cell, shared.size.1);
        let screen = shared.screen();
        let pick = match what.to_ascii_lowercase() {
            b'a' => Some(Pick::Shown),
            b'i' => Some(Pick::Image {
                id: image,
                placement,
            }),
            b'n' => screen
                .store
                .newest(command.int(b'I'))
                .map(|id| Pick::Image { id, placement }),
            b'c' => Some(Pick::Cell {
                line: cursor.line.0,
                column: cursor.column.0,
                z: None,
            }),
            b'p' => at(None),
            b'q' => at(Some(z)),
            b'x' => (x > 0).then(|| Pick::Column(x as usize - 1)),
            b'y' => (y > 0).then(|| Pick::Row(y as i32 - 1)),
            b'z' => Some(Pick::Z(z)),
            b'r' => Some(Pick::Range(x, y)),
            _ => {
                log::warn!(
                    "graphics {pid}: delete {} is none kitty names",
                    what as char
                );
                None
            }
        };
        if let Some(pick) = pick {
            log::debug!("graphics {pid}: delete {pick:?}");
            screen.delete(pick, what.is_ascii_uppercase(), lines, cell);
        }
    }

    #[cfg(test)]
    fn image(&self, id: u32) -> Option<store::Image> {
        self.lock().screen().store.get(id).cloned()
    }

    #[cfg(test)]
    fn ids(&self) -> Vec<u32> {
        self.lock().screen().store.within(1, u32::MAX)
    }
}

impl alacritty_terminal::term::Graphics for Graphics {
    fn command(&mut self, command: &[u8], cursor: Point) -> Answer {
        let shown = &command[..command.len().min(64)];
        log::debug!(
            "graphics {}: {} bytes at {}, {}: {}",
            self.pid,
            command.len(),
            cursor.line.0,
            cursor.column.0,
            String::from_utf8_lossy(shown)
        );
        let reply = match Command::parse(command) {
            Ok(command) => self.execute(&command, cursor),
            Err(reason) => {
                log::warn!("graphics {}: a command drops: {reason}", self.pid);
                None
            }
        };
        Answer {
            reply,
            cursor: self.moved.take(),
        }
    }

    fn scrolled(&mut self, region: &Range<Line>, lines: i32, oldest: Line) {
        let mut shared = self.lock();
        let (cell, screen) = (shared.cell, shared.size.1);
        shared
            .screen()
            .scrolled(region, lines, oldest, cell, screen);
    }

    fn changed(&mut self, change: Change) {
        let pid = self.pid;
        let mut shared = self.lock();
        let (cell, lines) = (shared.cell, shared.size.1);
        match change {
            Change::Screen => shared.screen().screen_cleared(lines, cell),
            Change::History => shared.screen().history_cleared(cell),
            Change::Reset => {
                for screen in &mut shared.screens {
                    screen.placements.clear();
                }
                shared.alt = false;
            }
            // The alternate screen starts empty, as its text does.
            Change::Alt(true) => {
                shared.screens[1] = Screen::new(shared.limit);
                shared.alt = true;
            }
            Change::Alt(false) => shared.alt = false,
        }
        log::debug!("graphics {pid}: {change:?}");
    }

    fn anchors(&self) -> Vec<Point> {
        let shared = self.lock();
        shared.screens[usize::from(shared.alt)].anchors()
    }

    fn resized(&mut self, moved: &[Option<Point>], columns: usize, lines: usize) {
        let mut shared = self.lock();
        shared.screen().resized(moved);
        shared.size = (columns, lines);
        log::debug!("graphics {}: resized to {columns} by {lines}", self.pid);
    }
}
