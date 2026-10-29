//! A terminal's write, grants and the read it asks about.

use std::collections::HashMap;

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

/// A password's grant: every read this session, or one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Grant {
    Always,
    Once,
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
    grants: HashMap<String, Grant>,
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
            Packet::Alias { mime, aliases } => {
                write.alias(mime, aliases);
                Ok(())
            }
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

    /// Whether `read` reads unasked; a one-time grant is spent.
    pub fn granted(&mut self, read: &Read) -> bool {
        let Some(pass) = &read.pass else {
            return false;
        };
        match self.grants.get(&pass.word) {
            Some(Grant::Always) => true,
            Some(Grant::Once) => {
                self.grants.remove(&pass.word);
                true
            }
            None => false,
        }
    }

    /// Lets `word` read for the rest of the session.
    pub fn always(&mut self, word: String) {
        self.grants.insert(word, Grant::Always);
    }

    /// A fresh password for one read, as paste events give.
    pub fn once(&mut self) -> Result<String, getrandom::Error> {
        let mut bytes = [0; 16];
        getrandom::fill(&mut bytes)?;
        let word: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        self.grants.insert(word.clone(), Grant::Once);
        Ok(word)
    }
}
