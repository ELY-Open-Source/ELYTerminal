use base64::{
    Engine,
    alphabet::STANDARD,
    engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig},
};

use super::{marks::Mark, notify::Progress};

/// What a program says it is doing, OSC 7501.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Idle,
    Working,
    Done,
    Blocked,
    Error,
}

/// What a blocked program waits for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Permission,
    Question,
    Auth,
}

/// A record, as the last report for its id left it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub state: State,
    pub kind: Option<Kind>,
    pub progress: Option<u8>,
    pub app: Option<String>,
    pub title: Option<String>,
    pub msg: Option<String>,
}

/// An OSC 7501 sequence, checked whole; an id is empty for the root record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Report(String, Record),
    /// Drops the record at the id and those under it; with no id, all.
    Clear(Option<String>),
    /// Asks which states and kinds the terminal takes.
    Query,
}

/// The answer to the query: every state and kind this terminal takes.
pub const REPLY: &str =
    "\x1b]7501;?:states=idle,working,done,blocked,error:kinds=permission,question,auth\x1b\\";

/// The spec's limits: a whole sequence, a key, an app, an id, its segments and levels.
const SEQUENCE: usize = 4096;
const KEY: usize = 16;
const APP: usize = 32;
const ID: usize = 128;
const SEGMENT: usize = 32;
const LEVELS: usize = 8;

/// The ids herdr's agents take, one by pane: no OSC 7501 id holds a colon, so none is taken for theirs.
pub(super) const HERDR: &str = "herdr:";

/// Records a terminal keeps; past them, the one updated longest ago goes.
pub const RECORDS: usize = 256;

/// Standard base64, its padding optional.
pub(super) const BASE64: GeneralPurpose = GeneralPurpose::new(
    &STANDARD,
    GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

/// The sequence in an OSC 7501 body past `7501;`, `length` bytes from OSC through ST; none for one the spec drops, logged with why.
pub fn read(body: &[u8], length: usize) -> Option<Status> {
    parse(body, length)
        .inspect_err(|why| log::warn!("pty: OSC 7501 dropped: {why}"))
        .ok()
}

/// A value's bytes, as the spec allows them.
fn valued(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"_.,+/=-".contains(&byte)
}

/// An id's or an app's bytes.
fn named(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"_.+-".contains(&byte)
}

fn parse(body: &[u8], length: usize) -> Result<Status, String> {
    if length > SEQUENCE {
        return Err(format!("{length} bytes, over {SEQUENCE}"));
    }
    if body.trim_ascii() == b"?" {
        return Ok(Status::Query);
    }
    // Every pair is checked before the last of each key wins.
    let (mut state, mut id, mut kind, mut progress, mut app, mut title, mut msg) =
        (None, None, None, None, None, None, None);
    for pair in body.split(|&byte| byte == b':') {
        // A malformed pair is skipped: no `=`, no key, or a byte outside the set.
        let Some(at) = pair.iter().position(|&byte| byte == b'=') else {
            continue;
        };
        let (key, value) = (pair[..at].trim_ascii(), pair[at + 1..].trim_ascii());
        if key.is_empty()
            || !key.iter().all(u8::is_ascii_lowercase)
            || !value.iter().all(|&byte| valued(byte))
        {
            continue;
        }
        if key.len() > KEY {
            return Err(format!("a key over {KEY} bytes"));
        }
        let value = String::from_utf8(value.to_vec()).expect("ASCII");
        match key {
            b"state" => state = Some(value),
            b"id" => id = Some(self::id(&value)?),
            b"kind" => kind = Some(value),
            b"progress" => progress = Some(value),
            b"app" if value.len() > APP => return Err(format!("an app over {APP} bytes")),
            b"app" => app = Some(value),
            b"title" => title = Some(text(&value, "title", 256, 192)?),
            b"msg" => msg = Some(text(&value, "msg", 2732, 2048)?),
            _ => {}
        }
    }
    let state = match state.as_deref().ok_or("no state")? {
        "idle" => State::Idle,
        "working" => State::Working,
        "done" => State::Done,
        "blocked" => State::Blocked,
        "error" => State::Error,
        "clear" => return Ok(Status::Clear(id)),
        other => return Err(format!("state {other} is not the spec's")),
    };
    let kind = match (state, kind.as_deref()) {
        (State::Blocked, Some("permission")) => Some(Kind::Permission),
        (State::Blocked, Some("question")) => Some(Kind::Question),
        (State::Blocked, Some("auth")) => Some(Kind::Auth),
        _ => None,
    };
    let progress = match state {
        State::Working | State::Blocked => progress
            .filter(|value| value.bytes().all(|byte| byte.is_ascii_digit()))
            .and_then(|value| value.parse::<u8>().ok())
            .filter(|&value| value <= 100),
        _ => None,
    };
    let app = app.filter(|app| !app.is_empty() && app.bytes().all(named));
    let record = Record {
        state,
        kind,
        progress,
        app,
        title,
        msg,
    };
    Ok(Status::Report(id.unwrap_or_default(), record))
}

/// An id that keeps to the spec's grammar and limits.
fn id(value: &str) -> Result<String, String> {
    let segments: Vec<&str> = value.split('/').collect();
    let fits = value.len() <= ID
        && segments.len() <= LEVELS
        && segments
            .iter()
            .all(|segment| (1..=SEGMENT).contains(&segment.len()) && segment.bytes().all(named));
    if fits {
        Ok(value.to_string())
    } else {
        Err(format!("id {value} is not the spec's"))
    }
}

/// The text a base64 value names, one line, within `encoded` and `decoded` bytes.
fn text(value: &str, key: &str, encoded: usize, decoded: usize) -> Result<String, String> {
    if value.len() > encoded {
        return Err(format!("a {key} over {encoded} bytes"));
    }
    let bytes = BASE64
        .decode(value)
        .map_err(|error| format!("{key} is no base64: {error}"))?;
    if bytes.len() > decoded {
        return Err(format!("a {key} over {decoded} bytes decoded"));
    }
    let text = String::from_utf8(bytes).map_err(|_| format!("{key} is not UTF-8"))?;
    if text.chars().any(char::is_control) {
        return Err(format!("{key} holds a control character"));
    }
    Ok(text)
}

/// A prompt's holder: the named shell, a group, or unknown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Holder {
    Shell,
    Group(i32),
    Unknown,
}

/// A terminal's records by id, with time and reporting group.
#[derive(Default)]
pub struct Records {
    list: Vec<(String, Record, u64, Option<i32>)>,
    clock: u64,
    /// A program has reported through OSC 7501 since the last full reset, so OSC 9;4 maps no more.
    reported: bool,
    /// The state a row showed at the last note.
    shows: Option<State>,
    /// Commands open between C and D, for an unknown holder.
    commands: u32,
}

impl Records {
    /// `owner`'s report replaces the record at `id` whole.
    pub fn report(&mut self, id: String, record: Record, owner: Option<i32>) {
        self.reported = true;
        self.put(id, record, owner);
    }

    /// OSC 9;4 as the root record, until a program reports through OSC 7501.
    pub fn progress(&mut self, progress: Progress, owner: Option<i32>) {
        if self.reported {
            log::debug!("status: OSC 9;4 passed by, a program reports through OSC 7501");
            return;
        }
        let root = |state, progress| Record {
            state,
            kind: None,
            progress,
            app: None,
            title: None,
            msg: None,
        };
        match progress {
            Progress::Remove => self.list.retain(|(id, ..)| !id.is_empty()),
            // Paused work is work still, its progress kept.
            Progress::Set(value) | Progress::Pause(value) => {
                self.put(String::new(), root(State::Working, value), owner)
            }
            Progress::Indeterminate => self.put(String::new(), root(State::Working, None), owner),
            Progress::Error(_) => self.put(String::new(), root(State::Error, None), owner),
        }
    }

    /// herdr's agents in place of the last ones; true when they differ.
    pub fn herdr(&mut self, agents: &[(String, Record)]) -> bool {
        let held = self.list.iter().filter(|(id, ..)| id.starts_with(HERDR));
        if held
            .map(|(id, record, ..)| (id, record))
            .eq(agents.iter().map(|(id, record)| (id, record)))
        {
            return false;
        }
        self.list.retain(|(id, ..)| !id.starts_with(HERDR));
        for (id, record) in agents {
            self.put(id.clone(), record.clone(), None);
        }
        true
    }

    /// A full reset: every record goes, and OSC 9;4 maps again.
    pub fn reset(&mut self) {
        self.list.clear();
        self.reported = false;
        self.commands = 0;
    }

    fn put(&mut self, id: String, record: Record, owner: Option<i32>) {
        self.clock += 1;
        if let Some(entry) = self.list.iter_mut().find(|(at, ..)| *at == id) {
            *entry = (id, record, self.clock, owner);
            return;
        }
        if self.list.len() == RECORDS {
            let oldest = (0..self.list.len())
                .min_by_key(|&at| self.list[at].2)
                .expect("records");
            let (gone, ..) = self.list.remove(oldest);
            log::debug!("status: record {gone:?} goes, the one updated longest ago");
        }
        self.list.push((id, record, self.clock, owner));
    }

    /// A program's clear: drops the record at `id` and every one under it; with none, all.
    pub fn clear(&mut self, id: Option<&str>) {
        self.reported = true;
        match id {
            None => self.list.clear(),
            Some(id) => {
                let under = format!("{id}/");
                self.list
                    .retain(|(at, ..)| at != id && !at.starts_with(&under));
            }
        }
    }

    /// Shell prompts drop live records; program prompts keep their own.
    pub fn mark(&mut self, mark: &Mark, holder: Holder) {
        match mark {
            Mark::Output => self.commands = self.commands.saturating_add(1),
            Mark::Done(_) => self.commands = self.commands.saturating_sub(1),
            Mark::Prompt(_) => {
                if holder == Holder::Shell {
                    self.commands = 0;
                }
                let inside = self.commands > 0;
                let before = self.list.len();
                self.list.retain(|(_, record, _, owner)| {
                    matches!(record.state, State::Done | State::Error)
                        || match (holder, owner) {
                            (Holder::Shell, _) => false,
                            (Holder::Group(group), Some(owner)) => group == *owner,
                            // Without groups to compare, C and D count the commands.
                            _ => inside,
                        }
                });
                log::debug!(
                    "status: a prompt held by {holder:?} drops {} records",
                    before - self.list.len()
                );
            }
            Mark::Input => {}
        }
    }

    /// The user typed: done and error records are seen and go, but herdr's, which herdr marks seen; true when any went.
    pub fn seen(&mut self) -> bool {
        let before = self.list.len();
        self.list.retain(|(id, record, ..)| {
            id.starts_with(HERDR) || !matches!(record.state, State::Done | State::Error)
        });
        self.list.len() != before
    }

    /// The record a row shows, with its id: blocked, else error, working, done or idle, the one reported last in its state.
    pub fn shown(&self) -> Option<(&str, &Record)> {
        [
            State::Blocked,
            State::Error,
            State::Working,
            State::Done,
            State::Idle,
        ]
        .into_iter()
        .find_map(|state| {
            self.list
                .iter()
                .filter(|(_, record, ..)| record.state == state)
                .max_by_key(|(_, _, at, _)| *at)
                .map(|(id, record, ..)| (id.as_str(), record))
        })
    }

    /// Notes the state a row shows now; true when it changed since the last note.
    pub fn restate(&mut self) -> bool {
        let now = self.shown().map(|(_, record)| record.state);
        let changed = now != self.shows;
        self.shows = now;
        changed
    }

    /// The state a row showed at the last note.
    pub fn shows(&self) -> Option<State> {
        self.shows
    }

    /// Each record with its id, in the order first reported.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Record)> {
        self.list
            .iter()
            .map(|(id, record, ..)| (id.as_str(), record))
    }

    /// The app of the record at `id`: its own, else its nearest ancestor's, the root's last; herdr's agents have none.
    pub fn app(&self, id: &str) -> Option<&str> {
        let mut at = id;
        loop {
            let app = self
                .list
                .iter()
                .find(|(name, ..)| name == at)
                .and_then(|(_, record, ..)| record.app.as_deref());
            if app.is_some() || at.is_empty() || at.starts_with(HERDR) {
                return app;
            }
            at = at.rfind('/').map_or("", |slash| &at[..slash]);
        }
    }
}

#[cfg(test)]
mod tests;
