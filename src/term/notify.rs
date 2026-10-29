use base64::Engine;

use super::status::BASE64;

/// A notification a program asks for: OSC 9, 777 or 99.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    /// OSC 99's identifier, which updates, closes and answers name; none for OSC 9, 777 and OSC 99 without one.
    pub id: Option<String>,
    pub title: String,
    pub body: String,
    /// A press shows the window and session (`a=focus`, the default), and tells the program (`a=report`).
    pub focus: bool,
    pub report: bool,
    /// The program hears of its close: `c=1`.
    pub close: bool,
    /// Milliseconds it lasts: `w` over 0.
    pub expire: Option<u32>,
}

/// Where a program's work stands, OSC 9;4.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    Remove,
    Set(Option<u8>),
    Error(Option<u8>),
    Indeterminate,
    Pause(Option<u8>),
}

/// What a program asks of notifications and progress.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ask {
    Show(Notice),
    /// Closes the notification this id names: OSC 99's `p=close`.
    Close(String),
    /// Which notifications are alive; the answer names this id: `p=alive`.
    Alive(String),
    /// What the terminal does with OSC 99; the answer names this id: `p=?`.
    Query(String),
    Progress(Progress),
}

/// The ask in an OSC 9 body past `9;`: ConEmu's commands, 9;4 among them, else iTerm2's notification, as Ghostty reads them.
pub fn nine(body: &[u8]) -> Option<Ask> {
    let semi = |at: usize| body.get(at) == Some(&b';');
    let conemu = match body {
        [] => false,
        [b'1', b';', ..] | [b'1', b'0'] => true,
        [b'1', b'0', b';', b'0'..=b'3', ..] | [b'1', b'1', b';', ..] | [b'1', b'2', ..] => true,
        [b'4', b';', state @ b'0'..=b'4', ..] => {
            return Some(Ask::Progress(progress(*state, &body[3..])));
        }
        [b'5', ..] => true,
        [b'2' | b'3' | b'6' | b'7' | b'8' | b'9', ..] => semi(1),
        _ => false,
    };
    if conemu {
        log::debug!(
            "pty: OSC 9;{} is ConEmu's, no notification",
            String::from_utf8_lossy(body)
        );
        return None;
    }
    let text = safe(body, "OSC 9")?;
    (!text.is_empty()).then(|| Ask::Show(plain(text, String::new())))
}

/// OSC 9;4's state and the progress after it, clamped to 100.
fn progress(state: u8, rest: &[u8]) -> Progress {
    let value = rest
        .strip_prefix(b";")
        .and_then(|digits| std::str::from_utf8(digits).ok())
        .filter(|digits| !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()))
        .and_then(|digits| digits.parse::<u64>().ok())
        .map(|value| value.min(100) as u8);
    match state {
        b'0' => Progress::Remove,
        // Set alone means 0, as in Ghostty; a value that does not read means none.
        b'1' if rest.is_empty() => Progress::Set(Some(0)),
        b'1' => Progress::Set(value),
        b'2' => Progress::Error(value),
        b'3' => Progress::Indeterminate,
        _ => Progress::Pause(value),
    }
}

/// The notification in an OSC 777 body past `777;`: rxvt's `notify;title;body`.
pub fn rxvt(body: &[u8]) -> Option<Ask> {
    let Some(rest) = body.strip_prefix(b"notify;") else {
        log::warn!(
            "pty: OSC 777;{} is no rxvt notify",
            String::from_utf8_lossy(body)
        );
        return None;
    };
    let Some(at) = rest.iter().position(|&byte| byte == b';') else {
        log::warn!("pty: OSC 777 notify without a title");
        return None;
    };
    let (title, body) = (
        safe(&rest[..at], "OSC 777")?,
        safe(&rest[at + 1..], "OSC 777")?,
    );
    (!title.is_empty() || !body.is_empty()).then(|| Ask::Show(plain(title, body)))
}

/// A notification with no options, the body standing in for a missing title.
fn plain(title: String, body: String) -> Notice {
    Notice {
        id: None,
        title,
        body,
        focus: true,
        report: false,
        close: false,
        expire: None,
    }
}

/// Text with no control character, else none, logged with `what` sent it.
fn safe(bytes: &[u8], what: &str) -> Option<String> {
    let text = String::from_utf8(bytes.to_vec())
        .inspect_err(|_| log::warn!("pty: {what} text is not UTF-8"))
        .ok()?;
    if text.chars().any(char::is_control) {
        log::warn!("pty: {what} text holds a control character");
        return None;
    }
    Some(text)
}

/// gpui closes no notification on Linux, nor replaces one, so close and expiry hold elsewhere alone.
pub(super) const CLOSES: bool = !cfg!(target_os = "linux");

/// The answer to OSC 99's query, naming `id`: a press focuses and reports, a notification posts while unseen, its payloads, and close and expiry where gpui closes one.
pub fn answer(id: &str) -> String {
    let (close, expire) = if CLOSES { ("close,", ":w=1") } else { ("", "") };
    format!(
        "\x1b]99;i={id}:p=?;a=focus,report:o=invisible:p=title,body,{close}?,alive{expire}\x1b\\"
    )
}

/// The answer to OSC 99's `p=alive`, naming `id`: the ids of notifications still alive.
pub fn alive(id: &str, alive: &[&str]) -> String {
    format!("\x1b]99;i={id}:p=alive;{}\x1b\\", alive.join(","))
}

/// What `c=1` hears of notification `id`: its close goes untracked, as gpui hears of none.
pub fn untracked(id: &str) -> String {
    format!("\x1b]99;i={id}:p=close;untracked\x1b\\")
}

/// What `a=report` hears when notification `id` is pressed.
pub fn activated(id: &str) -> String {
    format!("\x1b]99;i={id};\x1b\\")
}

/// An OSC 99 payload's most bytes: plain, and base64; and its metadata's.
const PLAIN: usize = 2048;
const ENCODED: usize = 4096;
const METADATA: usize = 1024;

/// The longest OSC 99 body the tap's scanner keeps: `99;`, metadata, `;` and a payload.
pub(super) const BODY: usize = 3 + METADATA + 1 + ENCODED;

/// The most text one notification gathers from its chunks, and how many gather at once.
const GATHERED: usize = 16384;
const PENDING: usize = 32;

/// A payload gathering its chunks: its bytes so far, and base64 short of a whole group.
#[derive(Default)]
struct Text {
    bytes: Vec<u8>,
    rest: Vec<u8>,
}

impl Text {
    /// Adds a chunk: plain text as it is, base64 by whole groups, so a program may chunk before encoding or after.
    fn add(&mut self, payload: &[u8], encoded: bool) -> Result<(), String> {
        // Plain text is UTF-8 without a control character; base64 holds any UTF-8, checked once whole.
        if !encoded {
            let text = std::str::from_utf8(payload).map_err(|_| "payload is not UTF-8")?;
            if text.chars().any(char::is_control) {
                return Err("payload holds a control character".into());
            }
            self.finish()?;
            self.bytes.extend_from_slice(payload);
            return Ok(());
        }
        self.rest.extend_from_slice(payload);
        let whole = self.rest.len() / 4 * 4;
        for group in self.rest[..whole].chunks(4) {
            self.bytes.extend(decode(group)?);
        }
        self.rest.drain(..whole);
        Ok(())
    }

    /// Decodes the base64 left, padded or not, as the last chunk may leave it.
    fn finish(&mut self) -> Result<(), String> {
        if !self.rest.is_empty() {
            self.bytes.extend(decode(&self.rest)?);
            self.rest.clear();
        }
        Ok(())
    }

    fn len(&self) -> usize {
        self.bytes.len() + self.rest.len()
    }
}

fn decode(base64: &[u8]) -> Result<Vec<u8>, String> {
    BASE64
        .decode(base64)
        .map_err(|error| format!("payload is no base64: {error}"))
}

/// A notification gathering its chunks.
struct Gathering {
    notice: Notice,
    title: Text,
    body: Text,
}

/// OSC 99 notifications gathered from their chunks, by id.
#[derive(Default)]
pub(crate) struct Kitty {
    pending: Vec<(String, Gathering)>,
}

impl Kitty {
    /// The ask an OSC 99 body past `99;` completes, logged with why when there is none.
    pub fn read(&mut self, body: &[u8]) -> Option<Ask> {
        self.parse(body)
            .inspect_err(|why| log::warn!("pty: OSC 99 dropped: {why}"))
            .ok()
            .flatten()
    }

    fn parse(&mut self, body: &[u8]) -> Result<Option<Ask>, String> {
        let at = body
            .iter()
            .position(|&byte| byte == b';')
            .ok_or("no payload")?;
        let (metadata, payload) = (&body[..at], &body[at + 1..]);
        if metadata.len() > METADATA {
            return Err(format!("metadata over {METADATA} bytes"));
        }
        let mut options: Vec<(u8, &[u8])> = Vec::new();
        for pair in metadata
            .split(|&byte| byte == b':')
            .filter(|pair| !pair.is_empty())
        {
            match pair {
                [key, b'=', value @ ..] if key.is_ascii_alphabetic() => options.push((*key, value)),
                _ => log::debug!("pty: OSC 99 passes {} by", String::from_utf8_lossy(pair)),
            }
        }
        let get = |key: u8| {
            options
                .iter()
                .rev()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| *value)
        };
        let id = match get(b'i') {
            Some(id)
                if id
                    .iter()
                    .all(|&byte| byte.is_ascii_alphanumeric() || b"_-+.".contains(&byte)) =>
            {
                Some(String::from_utf8(id.to_vec()).expect("ASCII"))
            }
            Some(id) => {
                return Err(format!(
                    "id {} holds a byte ids may not",
                    String::from_utf8_lossy(id)
                ));
            }
            None => None,
        };
        let encoded = get(b'e') == Some(b"1");
        let limit = if encoded { ENCODED } else { PLAIN };
        if payload.len() > limit {
            return Err(format!("a payload over {limit} bytes"));
        }
        // An answer names the asker's id, or 0 without one, as kitty's spec says.
        let named = || id.clone().unwrap_or_else(|| "0".into());
        match get(b'p').unwrap_or(b"title") {
            b"?" => return Ok(Some(Ask::Query(named()))),
            b"alive" => return Ok(Some(Ask::Alive(named()))),
            b"close" => return Ok(id.map(Ask::Close)),
            b"title" | b"body" => {}
            other => {
                log::debug!(
                    "pty: OSC 99 payload {} passed by",
                    String::from_utf8_lossy(other)
                );
                return Ok(None);
            }
        }
        let key = id.clone().unwrap_or_default();
        let at = match self.pending.iter().position(|(name, _)| *name == key) {
            Some(at) => at,
            None if self.pending.len() == PENDING => {
                return Err(format!("{PENDING} notifications gathering already"));
            }
            None => {
                let notice = Notice {
                    id: id.clone(),
                    ..plain(String::new(), String::new())
                };
                let (title, body) = (Text::default(), Text::default());
                self.pending.push((
                    key,
                    Gathering {
                        notice,
                        title,
                        body,
                    },
                ));
                self.pending.len() - 1
            }
        };
        let gathering = &mut self.pending[at].1;
        let text = if get(b'p') == Some(b"body") {
            &mut gathering.body
        } else {
            &mut gathering.title
        };
        // A chunk that breaks the rules drops what its notification gathered.
        if let Err(why) = text.add(payload, encoded) {
            self.pending.remove(at);
            return Err(why);
        }
        if gathering.title.len() + gathering.body.len() > GATHERED {
            self.pending.remove(at);
            return Err(format!("a notification over {GATHERED} bytes"));
        }
        let notice = &mut gathering.notice;
        if let Some(actions) = get(b'a') {
            for action in actions.split(|&byte| byte == b',') {
                let (on, name) = match action.strip_prefix(b"-") {
                    Some(name) => (false, name),
                    None => (true, action),
                };
                match name {
                    b"focus" => notice.focus = on,
                    b"report" => notice.report = on,
                    _ => {}
                }
            }
        }
        if let Some(close) = get(b'c') {
            notice.close = close == b"1";
        }
        if let Some(expire) = get(b'w') {
            notice.expire = std::str::from_utf8(expire)
                .ok()
                .and_then(|ms| ms.parse::<u32>().ok())
                .filter(|&ms| ms > 0);
        }
        if get(b'd') == Some(b"0") {
            return Ok(None);
        }
        let (_, gathering) = self.pending.remove(at);
        let Gathering {
            mut notice,
            mut title,
            mut body,
        } = gathering;
        title.finish()?;
        body.finish()?;
        notice.title = String::from_utf8(title.bytes).map_err(|_| "title is not UTF-8")?;
        notice.body = String::from_utf8(body.bytes).map_err(|_| "body is not UTF-8")?;
        if notice.title.is_empty() && notice.body.is_empty() {
            return Ok(None);
        }
        if notice.title.is_empty() {
            notice.title = std::mem::take(&mut notice.body);
        }
        Ok(Some(Ask::Show(notice)))
    }
}

/// A notification or progress an OSC 99, 777 or 9 body asks for.
pub(super) fn ask(body: &[u8], kitty: &mut Kitty) -> Option<Ask> {
    if let Some(rest) = body.strip_prefix(b"99;") {
        kitty.read(rest)
    } else if let Some(rest) = body.strip_prefix(b"777;") {
        rxvt(rest)
    } else {
        body.strip_prefix(b"9;").and_then(nine)
    }
}

#[cfg(test)]
mod tests;
