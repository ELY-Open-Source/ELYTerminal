//! A write under way, its base64 decoded as it comes.

use std::rc::Rc;

use base64_simd::STANDARD;

use super::{LIMIT, Place};

/// The most types, aliases among them, one write holds.
const TYPES: usize = 64;

/// Why a write fails, as its status names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fail {
    Invalid,
    TooBig,
}

impl Fail {
    pub fn status(self) -> &'static str {
        match self {
            Self::Invalid => "EINVAL",
            Self::TooBig => "EFBIG",
        }
    }
}

/// Each type and its data, which aliases share.
pub type Parts = Vec<(String, Rc<Vec<u8>>)>;

/// A write's types and data so far.
#[derive(Debug)]
pub struct Write {
    pub place: Place,
    pub id: String,
    parts: Vec<(String, Vec<u8>)>,
    /// Base64 past the last whole group of four.
    carry: Vec<u8>,
    /// The type's padding came; no more may follow.
    ended: bool,
    aliases: Vec<(String, String)>,
    size: usize,
}

impl Write {
    pub fn new(place: Place, id: String) -> Self {
        Self {
            place,
            id,
            parts: Vec::new(),
            carry: Vec::new(),
            ended: false,
            aliases: Vec::new(),
            size: 0,
        }
    }

    /// A chunk of `mime`'s base64; a type's chunks come together.
    pub fn add(&mut self, mime: &str, payload: &[u8]) -> Result<(), Fail> {
        if self.parts.last().is_none_or(|(last, _)| last != mime) {
            self.close()?;
            if self.parts.iter().any(|(sent, _)| sent == mime) {
                log::warn!("clipboard: {mime} came again after another type");
                return Err(Fail::Invalid);
            }
            if self.parts.len() + self.aliases.len() >= TYPES {
                log::warn!("clipboard: a write passes {TYPES} types");
                return Err(Fail::TooBig);
            }
            self.parts.push((mime.to_string(), Vec::new()));
            self.grow(mime.len())?;
        }
        if payload.is_empty() {
            return Ok(());
        }
        if self.ended {
            log::warn!("clipboard: {mime} goes on past its padding");
            return Err(Fail::Invalid);
        }
        self.carry.extend_from_slice(payload);
        let whole = self.carry.len() / 4 * 4;
        let data = &mut self.parts.last_mut().expect("a part was pushed").1;
        let before = data.len();
        STANDARD
            .decode_append(&self.carry[..whole], data)
            .map_err(|error| {
                log::warn!("clipboard: {mime} is no padded base64: {error}");
                Fail::Invalid
            })?;
        let grown = data.len() - before;
        self.ended = self.carry[..whole].ends_with(b"=");
        self.carry.drain(..whole);
        self.grow(grown)
    }

    /// Counts `bytes` more against the limit.
    fn grow(&mut self, bytes: usize) -> Result<(), Fail> {
        self.size += bytes;
        if self.size > LIMIT {
            log::warn!("clipboard: a write passes {LIMIT} bytes");
            return Err(Fail::TooBig);
        }
        Ok(())
    }

    /// `aliases` hold what `mime` holds; their names count too.
    pub fn alias(&mut self, mime: String, aliases: Vec<String>) -> Result<(), Fail> {
        if self.parts.len() + self.aliases.len() + aliases.len() > TYPES {
            log::warn!("clipboard: a write's aliases pass {TYPES} types");
            return Err(Fail::TooBig);
        }
        self.grow(aliases.iter().map(|alias| alias.len() + mime.len()).sum())?;
        self.aliases
            .extend(aliases.into_iter().map(|alias| (alias, mime.clone())));
        Ok(())
    }

    /// Each type and its data, aliases after, sharing it.
    pub fn finish(mut self) -> Result<Parts, Fail> {
        self.close()?;
        let mut parts: Parts = self
            .parts
            .into_iter()
            .map(|(mime, data)| (mime, Rc::new(data)))
            .collect();
        for (alias, mime) in self.aliases {
            let held = parts.iter().find(|(sent, _)| *sent == mime);
            match held.map(|(_, data)| Rc::clone(data)) {
                Some(data) if !parts.iter().any(|(sent, _)| *sent == alias) => {
                    parts.push((alias, data));
                }
                Some(_) => log::debug!("clipboard: alias {alias} was sent itself"),
                None => log::debug!("clipboard: alias {alias} names {mime}, never sent"),
            }
        }
        Ok(parts)
    }

    /// Ends the type under way, on a whole group.
    fn close(&mut self) -> Result<(), Fail> {
        if !self.carry.is_empty() {
            log::warn!("clipboard: a type's base64 lacks its padding");
            return Err(Fail::Invalid);
        }
        self.ended = false;
        Ok(())
    }
}
