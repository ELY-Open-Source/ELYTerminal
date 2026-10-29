//! The view's side: the system clipboard read and written.

use std::time::Instant;

use gpui::{ClipboardItem, Context};

use super::{Asking, LIST, Packet, Place, Read, Step, answer, answers, found, holds};
use crate::term::view::TerminalView;

/// Linux alone has a primary selection.
const PRIMARY: bool = cfg!(any(target_os = "linux", target_os = "freebsd"));

impl TerminalView {
    /// One OSC 5522 packet.
    pub(in crate::term) fn clipboard(&mut self, packet: Packet, cx: &mut Context<Self>) {
        match self.clips.heard(packet, PRIMARY) {
            Step::Nothing => {}
            Step::Answer(bytes) => self.pty.write(bytes),
            Step::Store(place, id, item) => {
                log::info!(
                    "terminal {}: a program writes {} to the {place:?}",
                    self.pty.pid,
                    super::types(&item).join(" ")
                );
                store(place, item.clone(), cx);
                // The read back tells a failed write; Wayland's comes later.
                let wayland = cx.compositor_name() == "Wayland";
                let status = if wayland || holds(load(place, cx).as_ref(), &item) {
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
        if read.place == Place::Primary && !PRIMARY {
            log::warn!("terminal {pid}: a read of the primary selection, none here");
            return self.pty.write(refuse("ENOSYS"));
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
