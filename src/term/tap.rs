use std::{io, path::PathBuf, sync::Arc};

use alacritty_terminal::{
    event::{OnResize, WindowSize},
    tty::{ChildEvent, EventedPty, EventedReadWrite},
};
use futures::channel::mpsc::UnboundedSender;
use polling::{Event, PollMode, Poller};

use super::{
    marks::{self, Mark},
    notify::{self, Kitty},
    prompts::{self, Prompts},
    pty::PtyEvent,
    status::{self, Holder},
    titles::{self, Titles},
};

/// The longest OSC body kept; longer ones are dropped whole.
pub(super) const MOST: usize = 4096;

/// alacritty's `MAX_LOCKED_READ`: the bytes one turn of its loop reads.
#[cfg(windows)]
pub(super) const TURN: usize = u16::MAX as usize;

/// A pty whose reader shows each byte to its scanners before the parser.
pub(crate) struct Tapped<P> {
    /// The master's copy, naming the foreground group; drops before `inner`.
    #[cfg(unix)]
    master: Option<std::fs::File>,
    pub inner: P,
    scanner: Osc,
    modes: Modes,
    prompts: Prompts,
    kitty: Kitty,
    titles: Titles,
    events: UnboundedSender<PtyEvent>,
    pid: u32,
    last: Option<PathBuf>,
    /// The session's own shell names its prompts.
    named: bool,
    /// The poller alacritty's event loop waits on.
    #[cfg(windows)]
    poller: Option<Arc<Poller>>,
    /// Bytes since the pty ran dry or the loop woke.
    #[cfg(windows)]
    unwoken: usize,
}

impl<P> Tapped<P> {
    pub fn new(inner: P, events: UnboundedSender<PtyEvent>, pid: u32) -> Self {
        Self {
            inner,
            scanner: Osc::default(),
            modes: Modes::default(),
            prompts: Prompts::default(),
            kitty: Kitty::default(),
            titles: Titles::default(),
            events,
            pid,
            last: None,
            named: false,
            #[cfg(windows)]
            poller: None,
            #[cfg(windows)]
            unwoken: 0,
            #[cfg(unix)]
            master: None,
        }
    }

    /// Names, by `master`, the process group each report comes from.
    #[cfg(unix)]
    pub fn holding(mut self, master: std::fs::File) -> Self {
        self.master = Some(master);
        self
    }
}

impl<P: EventedReadWrite> io::Read for Tapped<P> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            // What a rewrite adds stays inside the buffer, with room for the held sequence's own.
            let room = buf
                .len()
                .saturating_sub(self.prompts.held() + prompts::SLACK);
            let take = room / prompts::GROWTH;
            if take == 0 {
                return Ok(0);
            }
            let (count, dry) = self.fill(&mut buf[..take])?;
            let passed = self.pass(buf, count);
            #[cfg(windows)]
            self.keep_waking(passed, dry);
            // Zero would end alacritty's turn while bytes wait.
            if passed > 0 || dry {
                return Ok(passed);
            }
        }
    }
}

impl<P: EventedReadWrite> Tapped<P> {
    /// Fills `buf`; true once the pty ran dry.
    fn fill(&mut self, buf: &mut [u8]) -> io::Result<(usize, bool)> {
        let mut count = 0;
        while count < buf.len() {
            match io::Read::read(self.inner.reader(), &mut buf[count..]) {
                Ok(0) => return Ok((count, true)),
                Ok(got) => count += got,
                Err(error) if count == 0 => return Err(error),
                // The error comes again at the next read.
                Err(_) => return Ok((count, true)),
            }
        }
        Ok((count, false))
    }

    /// Scans `buf[..count]`; the parser's bytes start `buf`, their count returned.
    fn pass(&mut self, buf: &mut [u8], count: usize) -> usize {
        let read = &buf[..count];
        let (events, pid, last) = (&self.events, self.pid, &mut self.last);
        let (kitty, titles, named) = (&mut self.kitty, &mut self.titles, &mut self.named);
        #[cfg(unix)]
        let (holder, prompts) = (
            || holder(self.master.as_ref(), pid),
            |group: Option<i32>, click| group.is_some_and(|group| prompts(group, click, pid)),
        );
        #[cfg(windows)]
        let (holder, prompts) = (
            || None,
            |group: Option<i32>, _| group.is_none() && nested(pid),
        );
        self.scanner.feed(read, &mut |found| {
            report(
                found,
                events,
                pid,
                last,
                (kitty, titles, named),
                &holder,
                &prompts,
            )
        });
        #[cfg(windows)]
        let win32 = self.modes.win32;
        if let Some(mode) = self.modes.feed(read) {
            log::info!("pty {pid}: mouse {mode:?}");
            if events.unbounded_send(PtyEvent::Mouse(mode)).is_err() {
                log::debug!("pty {pid}: a mouse mode after its session closed");
            }
        }
        #[cfg(windows)]
        if self.modes.win32 != win32 {
            log::info!("pty {pid}: win32-input-mode {}", self.modes.win32);
            if events
                .unbounded_send(PtyEvent::Win32(self.modes.win32))
                .is_err()
            {
                log::debug!("pty {pid}: win32-input-mode after its session closed");
            }
        }
        let out = self.prompts.pass(&buf[..count]);
        buf[..out.len()].copy_from_slice(out);
        out.len()
    }

    /// Wakes alacritty's loop when bytes wait unread (alacritty#8147).
    #[cfg(windows)]
    fn keep_waking(&mut self, passed: usize, dry: bool) {
        use polling::os::iocp::{CompletionPacket, PollerIocpExt};
        if dry {
            self.unwoken = 0;
            return;
        }
        self.unwoken += passed;
        if self.unwoken < TURN {
            return;
        }
        self.unwoken = 0;
        let Some(poller) = &self.poller else {
            log::error!("pty {}: read before the event loop registered it", self.pid);
            return;
        };
        let packet = CompletionPacket::new(Event::readable(
            alacritty_terminal::tty::PTY_READ_WRITE_TOKEN,
        ));
        match poller.post(packet) {
            Ok(()) => log::debug!(
                "pty {}: a turn's bytes read, more may wait; woke the loop",
                self.pid
            ),
            Err(error) => log::error!("pty {}: cannot wake the event loop: {error}", self.pid),
        }
    }
}

/// Sends the session what the scanner found, with `holder`'s group.
fn report(
    found: Found,
    events: &UnboundedSender<PtyEvent>,
    pid: u32,
    last: &mut Option<PathBuf>,
    (kitty, titles, named): (&mut Kitty, &mut Titles, &mut bool),
    holder: &dyn Fn() -> Option<i32>,
    prompts: &dyn Fn(Option<i32>, marks::Click) -> bool,
) {
    let event = match found {
        Found::Reset => {
            log::debug!("pty {pid}: a full reset");
            titles.reset();
            PtyEvent::Reset
        }
        Found::Push => {
            titles.push();
            return;
        }
        Found::Pop => match titles.pop() {
            Some(title) => PtyEvent::Title(title),
            None => return,
        },
        Found::Osc(body, length) => {
            if let Some(mark) = body.strip_prefix(b"133;").and_then(marks::read) {
                let holder = match mark {
                    Mark::Prompt(_) if body.strip_prefix(b"133;").is_some_and(marks::named) => {
                        // A nested shell's name stays its own.
                        *named |= holder().is_some_and(|group| u32::try_from(group) == Ok(pid));
                        Holder::Shell
                    }
                    Mark::Prompt(click) => prompter(click, holder(), pid, *named, prompts),
                    _ => Holder::Unknown,
                };
                log::debug!("pty {pid}: {mark:?} held by {holder:?}");
                PtyEvent::Mark(mark, holder)
            } else if let Some(status) = body
                .strip_prefix(b"7501;")
                .and_then(|rest| status::read(rest, length))
            {
                let group = holder();
                log::debug!("pty {pid}: {status:?} from group {group:?}");
                PtyEvent::Status(status, group)
            } else if let Some(ask) = ask(body, kitty) {
                let group = holder();
                log::debug!("pty {pid}: {ask:?} from group {group:?}");
                PtyEvent::Ask(ask, group)
            } else if let Some(folder) = body.strip_prefix(b"7;").and_then(folder) {
                if last.as_ref() == Some(&folder) {
                    log::debug!("pty {pid}: folder {} again", folder.display());
                } else {
                    log::info!("pty {pid}: folder {}", folder.display());
                    *last = Some(folder.clone());
                }
                PtyEvent::Folder(folder)
            } else if let Some(rest) = body
                .strip_prefix(b"0;")
                .or_else(|| body.strip_prefix(b"2;"))
            {
                PtyEvent::Title(titles.set(rest))
            } else {
                return;
            }
        }
    };
    if events.unbounded_send(event).is_err() {
        log::debug!("pty {pid}: an event after its session closed");
    }
}

/// A notification or progress an OSC 99, 777 or 9 body asks for.
fn ask(body: &[u8], kitty: &mut Kitty) -> Option<notify::Ask> {
    if let Some(rest) = body.strip_prefix(b"99;") {
        kitty.read(rest)
    } else if let Some(rest) = body.strip_prefix(b"777;") {
        notify::rxvt(rest)
    } else {
        body.strip_prefix(b"9;").and_then(notify::nine)
    }
}

impl<P: EventedReadWrite> EventedReadWrite for Tapped<P> {
    type Reader = Self;
    type Writer = P::Writer;

    unsafe fn register(
        &mut self,
        poll: &Arc<Poller>,
        event: Event,
        mode: PollMode,
    ) -> io::Result<()> {
        #[cfg(windows)]
        {
            self.poller = Some(poll.clone());
        }
        unsafe { self.inner.register(poll, event, mode) }
    }

    fn reregister(&mut self, poll: &Arc<Poller>, event: Event, mode: PollMode) -> io::Result<()> {
        #[cfg(windows)]
        {
            self.poller = Some(poll.clone());
        }
        self.inner.reregister(poll, event, mode)
    }

    fn deregister(&mut self, poll: &Arc<Poller>) -> io::Result<()> {
        #[cfg(windows)]
        {
            self.poller = None;
        }
        self.inner.deregister(poll)
    }

    fn reader(&mut self) -> &mut Self {
        self
    }

    fn writer(&mut self) -> &mut P::Writer {
        self.inner.writer()
    }
}

impl<P: EventedPty> EventedPty for Tapped<P> {
    fn next_child_event(&mut self) -> Option<ChildEvent> {
        self.inner.next_child_event()
    }
}

impl<P: OnResize> OnResize for Tapped<P> {
    fn on_resize(&mut self, size: WindowSize) {
        self.inner.on_resize(size);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum State {
    #[default]
    Ground,
    Escape,
    Osc,
    Csi,
}

/// What the scanner finds: an OSC's body with the sequence's length from OSC through ST, a full reset, or the title stack's push and pop.
pub(crate) enum Found<'a> {
    Osc(&'a [u8], usize),
    Reset,
    Push,
    Pop,
}

/// Finds each OSC body across reads, ended as vte ends it: at BEL, ESC, CAN or SUB, each full reset, and CSI 22 t and 23 t.
#[derive(Default)]
pub(crate) struct Osc {
    state: State,
    body: Vec<u8>,
    /// The sequence's bytes so far, the controls the body leaves out too.
    length: usize,
    overflow: bool,
    /// A CSI's first parameter while it reads as a plain number, and its fields so far.
    first: Option<u32>,
    fields: usize,
}

impl Osc {
    pub fn feed(&mut self, bytes: &[u8], found: &mut impl FnMut(Found)) {
        let mut rest = bytes;
        while !rest.is_empty() {
            if self.state == State::Ground {
                match rest.iter().position(|&byte| byte == 0x1b) {
                    Some(at) => {
                        self.state = State::Escape;
                        rest = &rest[at + 1..];
                    }
                    None => return,
                }
                continue;
            }
            let byte = rest[0];
            rest = &rest[1..];
            self.state = match (self.state, byte) {
                (State::Escape, b']') => {
                    self.body.clear();
                    self.length = 2;
                    self.overflow = false;
                    State::Osc
                }
                (State::Escape, b'c') => {
                    found(Found::Reset);
                    State::Ground
                }
                (State::Escape, b'[') => {
                    self.first = Some(0);
                    self.fields = 1;
                    State::Csi
                }
                // vte runs controls in an escape, passes DEL and high bytes by, and stays there.
                (State::Escape, 0x00..=0x17 | 0x19 | 0x1b..=0x1f | 0x7f..=0xff) => State::Escape,
                (State::Escape, _) => State::Ground,
                (State::Osc, 0x07 | 0x18 | 0x1a) => {
                    self.finish(1, found);
                    State::Ground
                }
                // ESC ends the body, then reads as an escape: `\` makes ST, counted so.
                (State::Osc, 0x1b) => {
                    self.finish(2, found);
                    State::Escape
                }
                (State::Osc, 0x00..=0x06 | 0x08..=0x17 | 0x19 | 0x1c..=0x1f) => {
                    self.length += 1;
                    State::Osc
                }
                (State::Osc, byte) => {
                    self.length += 1;
                    if self.body.len() < self.most() {
                        self.body.push(byte);
                    } else {
                        self.overflow = true;
                    }
                    State::Osc
                }
                // In a CSI, as in vte: CAN and SUB cancel, ESC restarts, controls run, DEL and high bytes pass by.
                (State::Csi, 0x18 | 0x1a) => State::Ground,
                (State::Csi, 0x1b) => State::Escape,
                (State::Csi, 0x00..=0x1f | 0x7f..=0xff) => State::Csi,
                (State::Csi, b'0'..=b'9') => {
                    if self.fields == 1 {
                        self.first = self.first.map(|first| {
                            first
                                .saturating_mul(10)
                                .saturating_add(u32::from(byte - b'0'))
                        });
                    }
                    State::Csi
                }
                (State::Csi, b';' | b':') => {
                    self.fields += 1;
                    State::Csi
                }
                (State::Csi, b't') => {
                    match self.first.filter(|_| self.fields <= PARAMS) {
                        Some(22) => found(Found::Push),
                        Some(23) => found(Found::Pop),
                        _ => {}
                    }
                    State::Ground
                }
                (State::Csi, 0x40..=0x7e) => State::Ground,
                // A private marker or an intermediate makes another sequence.
                (State::Csi, _) => {
                    self.first = None;
                    State::Csi
                }
                (State::Ground, _) => unreachable!("ground scans by position"),
            };
        }
    }

    /// The longest body kept: an OSC 99's holds its payload past its metadata, a title's more.
    fn most(&self) -> usize {
        if self.body.starts_with(b"99;") {
            notify::BODY
        } else if self.body.starts_with(b"0;") || self.body.starts_with(b"2;") {
            titles::LONGEST
        } else {
            MOST
        }
    }

    fn finish(&mut self, end: usize, found: &mut impl FnMut(Found)) {
        if self.overflow {
            log::warn!("pty: an OSC over {} bytes dropped", self.most());
        } else {
            found(Found::Osc(&self.body, self.length + end));
        }
        self.body.clear();
    }
}

/// The most values, subparameters too, one sequence holds before vte drops it.
const PARAMS: usize = 32;

mod folder;
mod holder;
mod modes;
#[cfg(test)]
mod tests;

use folder::folder;
#[cfg(windows)]
use holder::nested;
use holder::prompter;
#[cfg(unix)]
use holder::{holder, prompts};
pub(crate) use modes::Modes;
