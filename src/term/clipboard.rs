//! kitty's clipboard protocol, OSC 5522: its packets and answers.

use base64_simd::STANDARD;
use gpui::ClipboardItem;

/// The longest body kept: a 4096-byte chunk and its keys.
pub(super) const BODY: usize = 8192;

/// One write's most bytes, the least kitty's spec allows.
#[cfg(not(test))]
pub(super) const LIMIT: usize = 64 * 1024 * 1024;
#[cfg(test)]
pub(super) const LIMIT: usize = 256;

/// The most bytes one answer carries, before base64.
const CHUNK: usize = 4096;

/// The type that lists what the clipboard holds.
pub(super) const LIST: &str = ".";

/// Which clipboard a packet names, `loc`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Clipboard,
    Primary,
}

/// A program's password and name, `pw` and `name`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pass {
    pub word: String,
    pub name: String,
}

/// A read: its types, place, id and pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Read {
    pub place: Place,
    pub mimes: Vec<String>,
    pub id: String,
    pub pass: Option<Pass>,
}

/// One OSC 5522 packet, as the tap reads it.
#[derive(Debug, PartialEq, Eq)]
pub enum Packet {
    Read(Read),
    Write {
        place: Place,
        id: String,
    },
    /// A type's chunk of base64; no type ends the write.
    Data {
        mime: Option<String>,
        payload: Vec<u8>,
    },
    Alias {
        mime: String,
        aliases: Vec<String>,
    },
    /// A write's packet breaking a rule; it fails the write.
    Broken,
}

/// The packet a body past `5522;` holds, if any.
pub(super) fn read(body: &[u8]) -> Option<Packet> {
    let (meta, payload) = match body.iter().position(|&byte| byte == b';') {
        Some(at) => (&body[..at], &body[at + 1..]),
        None => (body, &b""[..]),
    };
    let kind = kind(meta);
    let writes = matches!(kind.as_deref(), Some("wdata" | "walias"));
    let keys = match keys(meta) {
        Ok(keys) => keys,
        Err(reason) => {
            log::warn!("pty: OSC 5522 {reason}");
            return writes.then_some(Packet::Broken);
        }
    };
    let decoded = |key| value(&keys, key).map(text).transpose();
    let packet = (|| -> Result<Option<Packet>, String> {
        let place = match value(&keys, "loc") {
            Some("primary") => Place::Primary,
            _ => Place::Clipboard,
        };
        let id = value(&keys, "id").map_or_else(String::new, sanitized);
        Ok(Some(match kind.as_deref() {
            Some("read") => {
                let pass = match (decoded("pw")?, decoded("name")?) {
                    (Some(word), Some(name)) if !word.is_empty() && !name.is_empty() => {
                        Some(Pass { word, name })
                    }
                    _ => None,
                };
                let mimes = words(&text(payload)?);
                Packet::Read(Read {
                    place,
                    mimes,
                    id,
                    pass,
                })
            }
            Some("write") => Packet::Write { place, id },
            Some("wdata") => Packet::Data {
                mime: decoded("mime")?.filter(|mime| !mime.is_empty()),
                payload: payload.to_vec(),
            },
            Some("walias") => {
                let mime = decoded("mime")?.filter(|mime| !mime.is_empty());
                let mime = mime.ok_or("an alias names no type")?;
                let aliases = words(&text(payload)?);
                Packet::Alias { mime, aliases }
            }
            other => {
                log::debug!("pty: OSC 5522 of type {other:?} passed by");
                return Ok(None);
            }
        }))
    })();
    packet.unwrap_or_else(|reason| {
        log::warn!("pty: OSC 5522 {reason}");
        writes.then_some(Packet::Broken)
    })
}

/// A packet past the bound fails a write, unless reading.
pub(super) fn long(body: &[u8]) -> Option<Packet> {
    let meta = body.split(|&byte| byte == b';').next().unwrap_or_default();
    (kind(meta).as_deref() != Some("read")).then_some(Packet::Broken)
}

/// Its type, read though another pair is malformed.
fn kind(meta: &[u8]) -> Option<String> {
    let pairs = meta.split(|&byte| byte == b':');
    let kind = pairs
        .filter_map(|pair| pair.strip_prefix(b"type="))
        .next_back()?;
    Some(String::from_utf8_lossy(kind).into_owned())
}

/// Its `key=value` pairs, joined by `:`.
fn keys(meta: &[u8]) -> Result<Vec<(String, String)>, String> {
    let meta = std::str::from_utf8(meta).map_err(|_| "keys are not UTF-8".to_string())?;
    meta.split(':')
        .map(|pair| match pair.split_once('=') {
            Some((key, value)) => Ok((key.to_string(), value.to_string())),
            None => Err(format!("{pair:?} is no key=value")),
        })
        .collect()
}

fn value<'a>(keys: &'a [(String, String)], key: &str) -> Option<&'a str> {
    keys.iter()
        .rev()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.as_str())
}

/// Padded base64 of UTF-8 text, as the spec asks.
fn text(encoded: impl AsRef<[u8]>) -> Result<String, String> {
    let bytes = STANDARD
        .decode_to_vec(encoded.as_ref())
        .map_err(|error| format!("holds no padded base64: {error}"))?;
    String::from_utf8(bytes).map_err(|_| "holds no UTF-8".to_string())
}

fn words(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_string).collect()
}

/// An id keeps only kitty's characters.
fn sanitized(id: &str) -> String {
    let kept = |c: &char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '+' | '.');
    id.chars().filter(kept).collect()
}

/// One answer: type, status, keys, then `payload` in base64.
pub(super) fn answer(kind: &str, status: &str, keys: &[(&str, &str)], payload: &[u8]) -> Vec<u8> {
    let mut text = format!("\x1b]5522;type={kind}:status={status}");
    for (key, value) in keys.iter().filter(|(_, value)| !value.is_empty()) {
        text.push_str(&format!(":{key}={value}"));
    }
    if !payload.is_empty() {
        text.push(';');
        text.push_str(&STANDARD.encode_to_string(payload));
    }
    text.push_str("\x1b\\");
    text.into_bytes()
}

/// A read's answers: OK, each type's data in chunks, DONE.
pub(super) fn answers(read: &Read, found: &[(String, Vec<u8>)], pass: &str) -> Vec<u8> {
    let pass = match pass {
        "" => String::new(),
        pass => STANDARD.encode_to_string(pass),
    };
    let place = match read.place {
        Place::Primary => "primary",
        Place::Clipboard => "",
    };
    let id = read.id.as_str();
    let mut out = answer(
        "read",
        "OK",
        &[("loc", place), ("id", id), ("pw", &pass)],
        b"",
    );
    for (mime, data) in found {
        let mime = STANDARD.encode_to_string(mime);
        let keys = [("id", id), ("mime", mime.as_str()), ("pw", &pass)];
        // An empty type, an empty list too, still answers.
        for chunk in data.chunks(CHUNK).chain(data.is_empty().then_some(&[][..])) {
            out.extend(answer("read", "DATA", &keys, chunk));
        }
    }
    out.extend(answer("read", "DONE", &[("id", id), ("pw", &pass)], b""));
    out
}

/// What `item` holds of each type; `.` lists its types.
pub(super) fn found(item: Option<&ClipboardItem>, mimes: &[String]) -> Vec<(String, Vec<u8>)> {
    // Each kind answers once, so repeats cost nothing.
    let mut answered = Vec::new();
    let mut each = |mime: &String| {
        let answers = if mime == LIST {
            Some(LIST)
        } else {
            reads(mime)
        };
        if answers.is_some_and(|it| answered.contains(&it)) {
            log::debug!("clipboard: {mime} read already in this read");
            return None;
        }
        if mime == LIST {
            answered.push(LIST);
            let types = item.map(types).unwrap_or_default();
            let list = if types.is_empty() {
                String::new()
            } else {
                types.join(" ") + "\n"
            };
            return Some((mime.clone(), list.into_bytes()));
        }
        let held = item.and_then(|item| data(item, mime));
        match held {
            Some(_) => answered.extend(answers),
            None => log::debug!("clipboard: no {mime} to read"),
        }
        held.map(|data| (mime.clone(), data))
    };
    mimes.iter().filter_map(&mut each).collect()
}

mod mime;
mod prompt;
mod serve;
mod session;
mod write;

use mime::{data, holds, item, reads, types};
pub(super) use session::{Asking, Session, Step};
pub(super) use write::{Fail, Write};

#[cfg(test)]
mod tests;
