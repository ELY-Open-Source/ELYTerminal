//! The view's side: the system clipboard read and written.

use std::time::Instant;

use gpui::{ClipboardItem, Context};

use super::{Asking, LIST, Packet, Place, Read, Step, answer, answers, found, holds};
use crate::term::view::TerminalView;

/// Whether a primary selection exists: Linux's, unless Wayland lacks it.
fn primary() -> bool {
    #[cfg(target_os = "linux")]
    return crate::blur::primary_selection();
    #[cfg(not(target_os = "linux"))]
    false
}

/// Whether another program holds Windows' clipboard open.
#[cfg(windows)]
fn busy() -> bool {
    use windows_sys::Win32::System::DataExchange::{CloseClipboard, OpenClipboard};
    // SAFETY: opened for no window, closed at once.
    if unsafe { OpenClipboard(std::ptr::null_mut()) } == 0 {
        return true;
    }
    unsafe { CloseClipboard() };
    false
}

#[cfg(not(windows))]
fn busy() -> bool {
    false
}

impl TerminalView {
    /// One OSC 5522 packet.
    pub(in crate::term) fn clipboard(&mut self, packet: Packet, cx: &mut Context<Self>) {
        match self.clips.heard(packet, primary()) {
            Step::Nothing => {}
            Step::Answer(bytes) => self.pty.write(bytes),
            Step::Store(_, id, _) if busy() => {
                log::warn!(
                    "terminal {}: the clipboard is busy; a write waits",
                    self.pty.pid
                );
                self.pty
                    .write(answer("write", "EBUSY", &[("id", &id)], b""));
            }
            Step::Store(place, id, item) => {
                log::info!(
                    "terminal {}: a program writes {} to the {place:?}",
                    self.pty.pid,
                    super::types(&item).join(" ")
                );
                store(place, item.clone(), cx);
                // gpui reports no failed write; the read back tells.
                let status = if holds(load(place, cx).as_ref(), &item) {
                    "DONE"
                } else {
                    log::warn!(
                        "terminal {}: the {place:?} did not take the write",
                        self.pty.pid
                    );
                    "EIO"
                };
                self.pty.write(answer("write", status, &[("id", &id)], b""));
            }
            Step::Read(read) => self.read_clipboard(read, cx),
        }
    }

    fn read_clipboard(&mut self, read: Read, cx: &mut Context<Self>) {
        let pid = self.pty.pid;
        let refuse = |status| answer("read", status, &[("id", &read.id)], b"");
        if read.place == Place::Primary && !primary() {
            log::warn!("terminal {pid}: a read of the primary selection, none here");
            return self.pty.write(refuse("ENOSYS"));
        }
        if busy() {
            log::warn!("terminal {pid}: the clipboard is busy; a read waits");
            return self.pty.write(refuse("EBUSY"));
        }
        let item = load(read.place, cx);
        let listing = read.mimes.iter().all(|mime| mime == LIST);
        if listing || self.clips.granted(&read, Instant::now()) {
            log::info!("terminal {pid}: a program reads {}", read.mimes.join(" "));
            return self
                .pty
                .write(answers(&read, &found(item.as_ref(), &read.mimes), ""));
        }
        if self.clips.asking.is_some() {
            log::warn!("terminal {pid}: a read while another asks, refused busy");
            return self.pty.write(refuse("EBUSY"));
        }
        log::info!(
            "terminal {pid}: a read of {} asks the user",
            read.mimes.join(" ")
        );
        self.clips.asking = Some(Asking { read, item });
        cx.notify();
    }

    /// The user's answer: allowed, for the session too, or not.
    pub(in crate::term) fn decide(&mut self, allow: bool, always: bool, cx: &mut Context<Self>) {
        let Some(Asking { read, item }) = self.clips.asking.take() else {
            return;
        };
        let pid = self.pty.pid;
        cx.notify();
        if !allow {
            log::info!("terminal {pid}: the user denied a clipboard read");
            return self
                .pty
                .write(answer("read", "EPERM", &[("id", &read.id)], b""));
        }
        if always && let Some(pass) = &read.pass {
            log::info!("terminal {pid}: {} may read for this session", pass.name);
            self.clips.always(pass.word.clone(), read.place);
        }
        log::info!(
            "terminal {pid}: the user allowed a read of {}",
            read.mimes.join(" ")
        );
        self.pty
            .write(answers(&read, &found(item.as_ref(), &read.mimes), ""));
    }

    /// A paste under mode 5522: types, and a one-read password.
    pub(in crate::term) fn paste_event(&mut self, cx: &mut Context<Self>) {
        let pid = self.pty.pid;
        if busy() {
            return log::warn!("terminal {pid}: the clipboard is busy; no paste event");
        }
        let word = match self.clips.once(Place::Clipboard, Instant::now()) {
            Ok(word) => word,
            Err(error) => return log::error!("terminal {pid}: no paste event password: {error}"),
        };
        let read = Read {
            place: Place::Clipboard,
            mimes: vec![LIST.to_string()],
            id: String::new(),
            pass: None,
        };
        let item = load(Place::Clipboard, cx);
        log::info!("terminal {pid}: a paste event lists the clipboard's types");
        self.pty
            .write(answers(&read, &found(item.as_ref(), &read.mimes), &word));
    }
}

fn load(place: Place, cx: &mut Context<TerminalView>) -> Option<ClipboardItem> {
    match place {
        Place::Clipboard => cx.read_from_clipboard(),
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        Place::Primary => cx.read_from_primary(),
        #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
        Place::Primary => unreachable!("a primary read is refused before"),
    }
}

fn store(place: Place, item: ClipboardItem, cx: &mut Context<TerminalView>) {
    match place {
        Place::Clipboard => cx.write_to_clipboard(item),
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        Place::Primary => cx.write_to_primary(item),
        #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
        Place::Primary => unreachable!("a primary write is refused before"),
    }
}

#[cfg(all(windows, test))]
mod tests;
