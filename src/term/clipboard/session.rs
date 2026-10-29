//! A terminal's write, grants and the read it asks about.

use std::time::{Duration, Instant};

use gpui::ClipboardItem;

use super::{Packet, Place, Read, Write, answer, item};

/// What a packet leaves the view to do.
pub enum Step {
    Nothing,
    Answer(Vec<u8>),
    /// A whole write: stored, it answers DONE.
    Store(Place, String, ClipboardItem),
    Read(Read),
}

/// How long a paste's password reads: the spec's short timeout.
pub const PASTE_LIFE: Duration = Duration::from_secs(10);

/// The newest paste's password, for one read of its place.
struct Paste {
    word: String,
    place: Place,
    at: Instant,
}

/// A read waiting on the user, with what it reads.
pub struct Asking {
    pub read: Read,
    pub item: Option<ClipboardItem>,
}

#[derive(Default)]
pub struct Session {
    /// The write under way; none after a failure.
    writing: Option<Write>,
    /// Passwords the user let read their place this session.
    always: Vec<(String, Place)>,
    paste: Option<Paste>,
    pub asking: Option<Asking>,
}

impl Session {
    /// One packet; `primary` says whether a primary selection exists here.
    pub fn heard(&mut self, packet: Packet, primary: bool) -> Step {
        let failed =
            |id: &str, status: &str| Step::Answer(answer("write", status, &[("id", id)], b""));
        match packet {
            Packet::Read(read) => return Step::Read(read),
            Packet::Write { place, id } => {
                if place == Place::Primary && !primary {
                    log::warn!("clipboard: a write to the primary selection, none here");
                    self.writing = None;
                    return failed(&id, "ENOSYS");
                }
                log::debug!("clipboard: a write to the {place:?} starts");
                self.writing = Some(Write::new(place, id));
                return Step::Nothing;
            }
            _ => {}
        }
        let Some(mut write) = self.writing.take() else {
            log::debug!("clipboard: a write's packet with no write under way passed by");
            return Step::Nothing;
        };
        let id = write.id.clone();
        let done = match packet {
            Packet::Data {
                mime: Some(mime),
                payload,
            } => write.add(&mime, &payload),
            Packet::Data { mime: None, .. } => {
                let place = write.place;
                let finished = write.finish().and_then(|parts| item(&parts));
                return match finished {
                    Ok(item) => Step::Store(place, id, item),
                    Err(fail) => {
                        log::warn!("clipboard: a write fails, {}", fail.status());
                        failed(&id, fail.status())
                    }
                };
            }
            Packet::Alias { mime, aliases } => write.alias(mime, aliases),
            Packet::Broken => Err(super::Fail::Invalid),
            Packet::Read(_) | Packet::Write { .. } => unreachable!("taken above"),
        };
        match done {
            Ok(()) => {
                self.writing = Some(write);
                Step::Nothing
            }
            Err(fail) => {
                log::warn!("clipboard: a write fails, {}", fail.status());
                failed(&id, fail.status())
            }
        }
    }

    /// Whether `read` reads unasked; a paste's password is spent.
    pub fn granted(&mut self, read: &Read, now: Instant) -> bool {
        let Some(pass) = &read.pass else {
            return false;
        };
        let pasted = |paste: &mut Paste| paste.word == pass.word && paste.place == read.place;
        if let Some(paste) = self.paste.take_if(pasted) {
            return now.saturating_duration_since(paste.at) < PASTE_LIFE;
        }
        self.always.contains(&(pass.word.clone(), read.place))
    }

    /// Lets `word` read `place` for the rest of the session.
    pub fn always(&mut self, word: String, place: Place) {
        self.always.push((word, place));
    }

    /// A paste event's fresh password; an older one is void.
    pub fn once(&mut self, place: Place, now: Instant) -> Result<String, getrandom::Error> {
        let mut bytes = [0; 16];
        getrandom::fill(&mut bytes)?;
        let word: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        self.paste = Some(Paste {
            word: word.clone(),
            place,
            at: now,
        });
        Ok(word)
    }
}

#[cfg(test)]
mod tests;
