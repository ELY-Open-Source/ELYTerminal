//! Kitty graphics commands, as the terminal hands them over.

mod command;
mod load;
mod media;
mod store;
#[cfg(test)]
mod tests;

use alacritty_terminal::index::Point;

use command::{Command, Quiet};
use load::Loading;
use store::Store;

/// A failure, as its reply names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Error {
    Both,
    Invalid,
    Read,
    Decompress,
    Format,
    Medium,
    NoSize,
    TooLarge,
    Space,
}

impl Error {
    /// Ghostty's words; kitty's for a file that does not read.
    fn message(self) -> &'static str {
        match self {
            Self::Both => "EINVAL: image ID and number are mutually exclusive",
            Self::Invalid => "EINVAL: invalid data",
            Self::Read => "EBADF:Failed to read image file",
            Self::Decompress => "EINVAL: decompression failed",
            Self::Format => "EINVAL: unsupported format",
            Self::Medium => "EINVAL: unsupported medium",
            Self::NoSize => "EINVAL: dimensions required",
            Self::TooLarge => "EINVAL: dimensions too large",
            Self::Space => "ENOMEM: out of memory",
        }
    }
}

/// What a command names its image by, echoed back.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Ids {
    id: u32,
    number: u32,
    placement: u32,
}

#[derive(Debug, PartialEq, Eq)]
struct Reply {
    ids: Ids,
    message: &'static str,
}

impl Reply {
    fn ok(ids: Ids) -> Self {
        Self { ids, message: "OK" }
    }

    fn failed(ids: Ids, error: Error) -> Self {
        Self {
            ids,
            message: error.message(),
        }
    }

    /// The reply's escape; none without an id or a number.
    fn text(&self) -> Option<String> {
        let Ids {
            id,
            number,
            placement,
        } = self.ids;
        if id == 0 && number == 0 {
            return None;
        }
        let keys: Vec<String> = [("i", id), ("I", number), ("p", placement)]
            .into_iter()
            .filter(|(_, value)| *value > 0)
            .map(|(key, value)| format!("{key}={value}"))
            .collect();
        Some(format!("\x1b_G{};{}\x1b\\", keys.join(","), self.message))
    }
}

/// One session's kitty graphics: its images and a transmission.
pub struct Graphics {
    pid: u32,
    store: Store,
    loading: Option<Loading>,
}

impl Graphics {
    pub fn new(pid: u32) -> Self {
        Self {
            pid,
            store: Store::new(store::LIMIT),
            loading: None,
        }
    }

    fn execute(&mut self, command: &Command) -> Option<String> {
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
                b't' | b'T' => {
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
                    self.transmit(command, ids)
                }
                b'd' => {
                    self.delete(command);
                    None
                }
                b'p' | b'f' | b'a' | b'c' => {
                    log::debug!(
                        "graphics {pid}: action {} is not taken yet",
                        command.action as char
                    );
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

    fn transmit(&mut self, command: &Command, ids: Ids) -> Option<Reply> {
        let pid = self.pid;
        let loading = match self.loading.take() {
            Some(mut loading) => match loading.add(&command.payload, pid) {
                Ok(()) => loading,
                Err(error) => return Some(Reply::failed(loading.ids, error)),
            },
            None => {
                // A reused id drops its image as the new starts.
                if ids.id > 0 && self.store.remove(ids.id) {
                    log::debug!("graphics {pid}: image {} replaced", ids.id);
                }
                match Loading::start(command, ids, pid) {
                    Ok(loading) => loading,
                    Err(error) => return Some(Reply::failed(ids, error)),
                }
            }
        };
        if command.more() {
            self.loading = Some(loading);
            return None;
        }
        let (ids, local) = (loading.ids, loading.local());
        let mut image = match loading.finish(pid) {
            Ok(image) => image,
            Err(error) => return Some(Reply::failed(ids, error)),
        };
        image.id = match ids.id {
            0 => self.store.free(ids.number > 0),
            id => id,
        };
        image.number = ids.number;
        let stored = Ids {
            id: image.id,
            ..ids
        };
        if let Err(error) = self.store.add(image, pid) {
            // A local image's size tells of its file.
            let error = if local { Error::Read } else { error };
            return Some(Reply::failed(ids, error));
        }
        // An image named by neither id nor number hears nothing.
        (ids.id > 0 || ids.number > 0).then(|| Reply::ok(stored))
    }

    /// Deletes the image data `d` names; placements come later.
    fn delete(&mut self, command: &Command) {
        let pid = self.pid;
        if self.loading.take().is_some() {
            log::warn!("graphics {pid}: a delete dropped a transmission unfinished");
        }
        let what = command.letter(b'd').unwrap_or(b'a');
        let ids = match what.to_ascii_lowercase() {
            // A placement id selects a placement, and none stands yet.
            b'i' if command.int(b'p') == 0 => vec![command.int(b'i')],
            b'n' if command.int(b'p') == 0 => {
                self.store.newest(command.int(b'I')).into_iter().collect()
            }
            b'r' => self.store.within(command.int(b'x'), command.int(b'y')),
            b'i' | b'n' | b'a' | b'c' | b'p' | b'q' | b'x' | b'y' | b'z' | b'f' => Vec::new(),
            _ => {
                log::warn!(
                    "graphics {pid}: delete {} is none kitty names",
                    what as char
                );
                return;
            }
        };
        if !what.is_ascii_uppercase() {
            return;
        }
        for id in ids {
            if self.store.remove(id) {
                log::debug!("graphics {pid}: image {id} deleted");
            }
        }
    }
}

impl alacritty_terminal::term::Graphics for Graphics {
    fn command(&mut self, command: &[u8], cursor: Point) -> Option<String> {
        let shown = &command[..command.len().min(64)];
        log::debug!(
            "graphics {}: {} bytes at {}, {}: {}",
            self.pid,
            command.len(),
            cursor.line.0,
            cursor.column.0,
            String::from_utf8_lossy(shown)
        );
        match Command::parse(command) {
            Ok(command) => self.execute(&command),
            Err(reason) => {
                log::warn!("graphics {}: a command drops: {reason}", self.pid);
                None
            }
        }
    }
}
