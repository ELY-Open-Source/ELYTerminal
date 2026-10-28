//! X input method compound text, decoded through libX11's own tables.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::ops::RangeInclusive;

/// What failed to decode, with its bytes.
#[derive(Debug)]
pub enum DecodeError {
    InvalidEncoding(Vec<u8>),
    UnsupportedEncoding(Vec<u8>),
}

/// The 96-sets' upper halves in `upper.bin`, by final byte.
const FINALS: &[u8] = b"ABCDFGHLMTVY_bf";
static UPPER: &[u8] = include_bytes!("../tables/upper.bin");
const GB2312: Table = Table(include_bytes!("../tables/gb2312.bin"), grid);
const JISX0208: Table = Table(include_bytes!("../tables/jisx0208.bin"), grid);
const JISX0212: Table = Table(include_bytes!("../tables/jisx0212.bin"), grid);
const KSC5601: Table = Table(include_bytes!("../tables/ksc5601.bin"), grid);
const BIG5: Table = Table(include_bytes!("../tables/big5.bin"), big5);
const BIG5HKSCS: Table = Table(include_bytes!("../tables/big5hkscs.bin"), big5);
const GBK: Table = Table(include_bytes!("../tables/gbk.bin"), gbk);

/// A two-byte set: its table and each pair's place.
#[derive(Clone, Copy)]
struct Table(&'static [u8], fn(u8, u8) -> Option<usize>);

impl Table {
    fn decode(self, bytes: &[u8], out: &mut String) -> Option<()> {
        let pairs = bytes.chunks_exact(2);
        if !pairs.remainder().is_empty() {
            return None;
        }
        for pair in pairs {
            out.push(lookup(self.0, (self.1)(pair[0], pair[1])?)?);
        }
        Some(())
    }
}

/// A set G0 or G1 holds.
#[derive(Clone, Copy)]
enum Set {
    Ascii,
    Roman,
    Katakana,
    Upper(usize),
    Double(Table),
}

impl Set {
    /// Decodes the leading `range` bytes and returns the rest.
    fn decode<'b>(
        self,
        bytes: &'b [u8],
        range: RangeInclusive<u8>,
        out: &mut String,
    ) -> Result<&'b [u8], DecodeError> {
        let end = bytes
            .iter()
            .position(|b| !range.contains(b))
            .unwrap_or(bytes.len());
        let (run, rest) = bytes.split_at(end);
        let invalid = || DecodeError::InvalidEncoding(run.to_vec());
        match self {
            Set::Ascii => out.extend(run.iter().map(|&b| char::from(b))),
            Set::Roman => out.extend(run.iter().map(|&b| match b {
                b'\\' => '¥',
                b'~' => '‾',
                _ => char::from(b),
            })),
            Set::Katakana => {
                for &b in run {
                    match b {
                        0xa1..=0xdf => out.extend(char::from_u32(u32::from(b) + 0xfec0)),
                        _ => return Err(invalid()),
                    }
                }
            }
            Set::Upper(set) => {
                for &b in run {
                    out.push(lookup(UPPER, set * 96 + usize::from(b - 0xa0)).ok_or_else(invalid)?);
                }
            }
            Set::Double(table) => table.decode(run, out).ok_or_else(invalid)?,
        }
        Ok(rest)
    }
}

/// G0 and G1 hold across escapes and segments, like libX11.
pub fn compound_text_to_utf8(bytes: &[u8]) -> Result<String, DecodeError> {
    let mut out = String::with_capacity(bytes.len());
    let mut g0 = Set::Ascii;
    // ISO 8859-1's, the first of FINALS.
    let mut g1 = Set::Upper(0);
    let mut rest = bytes;
    while let [byte, tail @ ..] = rest {
        rest = match byte {
            0x1b => escape(tail, &mut g0, &mut g1, &mut out)?,
            0x21..=0x7e => g0.decode(rest, 0x21..=0x7e, &mut out)?,
            0xa0..=0xff => g1.decode(rest, 0xa0..=0xff, &mut out)?,
            _ => {
                out.push(char::from(*byte));
                tail
            }
        };
    }
    Ok(out)
}

/// Reads the sequence after an ESC; returns what follows.
fn escape<'b>(
    bytes: &'b [u8],
    g0: &mut Set,
    g1: &mut Set,
    out: &mut String,
) -> Result<&'b [u8], DecodeError> {
    let end = bytes
        .iter()
        .position(|b| !(0x20..=0x2f).contains(b))
        .map_or(bytes.len(), |end| end + 1);
    let (sequence, rest) = bytes.split_at(end);
    let escaped = || [&[0x1b][..], sequence].concat();
    if !sequence.last().is_some_and(|b| (0x30..=0x7e).contains(b)) {
        return Err(DecodeError::InvalidEncoding(escaped()));
    }
    let unsupported = || DecodeError::UnsupportedEncoding(escaped());
    match sequence {
        b"(B" => *g0 = Set::Ascii,
        b"(J" => *g0 = Set::Roman,
        b")I" => *g1 = Set::Katakana,
        b"%@" => {}
        b"%G" => return utf8(rest, out),
        b"%/2" => return segment(rest, out),
        [b'-', last] => {
            *g1 = Set::Upper(
                FINALS
                    .iter()
                    .position(|f| f == last)
                    .ok_or_else(unsupported)?,
            )
        }
        [b'$', b'(', last] => *g0 = double(*last).ok_or_else(unsupported)?,
        _ => return Err(unsupported()),
    }
    Ok(rest)
}

/// Reads UTF-8 to `ESC % @`, or to the end.
fn utf8<'b>(bytes: &'b [u8], out: &mut String) -> Result<&'b [u8], DecodeError> {
    let end = bytes
        .windows(3)
        .position(|w| w == b"\x1b%@")
        .unwrap_or(bytes.len());
    let text = &bytes[..end];
    let text =
        core::str::from_utf8(text).map_err(|_| DecodeError::InvalidEncoding(text.to_vec()))?;
    out.push_str(text);
    Ok(bytes.get(end + 3..).unwrap_or_default())
}

/// Reads an extended segment: length, encoding name, STX, text.
fn segment<'b>(bytes: &'b [u8], out: &mut String) -> Result<&'b [u8], DecodeError> {
    let invalid = || DecodeError::InvalidEncoding(bytes.to_vec());
    let [m @ 0x80..=0xff, l @ 0x80..=0xff, rest @ ..] = bytes else {
        return Err(invalid());
    };
    let length = usize::from(m - 0x80) * 128 + usize::from(l - 0x80);
    let (segment, rest) = rest.split_at_checked(length).ok_or_else(invalid)?;
    let stx = segment
        .iter()
        .position(|&b| b == 0x02)
        .ok_or_else(invalid)?;
    let (name, text) = (&segment[..stx], &segment[stx + 1..]);
    let table = match name {
        b"big5-0" => BIG5,
        b"big5hkscs-0" => BIG5HKSCS,
        b"gbk-0" => GBK,
        _ => return Err(DecodeError::UnsupportedEncoding(name.to_vec())),
    };
    table
        .decode(text, out)
        .ok_or_else(|| DecodeError::InvalidEncoding(text.to_vec()))?;
    Ok(rest)
}

/// A 94 by 94 set, by its final byte.
fn double(last: u8) -> Option<Set> {
    Some(Set::Double(match last {
        b'A' => GB2312,
        b'B' => JISX0208,
        b'C' => KSC5601,
        b'D' => JISX0212,
        _ => return None,
    }))
}

/// A pair's place in a 94 by 94 set.
fn grid(row: u8, cell: u8) -> Option<usize> {
    let place = |b: u8| (0x21..=0x7e).contains(&b).then(|| usize::from(b - 0x21));
    Some(place(row)? * 94 + place(cell)?)
}

/// A Big5 pair's place, leads from 0x81, as libX11 counts.
fn big5(lead: u8, trail: u8) -> Option<usize> {
    let trail = match trail {
        0x40..=0x7e => trail - 0x40,
        0xa1..=0xfe => trail - 0x62,
        _ => return None,
    };
    Some(usize::from(lead.checked_sub(0x81)?) * 157 + usize::from(trail))
}

/// A GBK pair's place, leads from 0x81.
fn gbk(lead: u8, trail: u8) -> Option<usize> {
    let trail = match trail {
        0x40..=0x7e => trail - 0x40,
        0x80..=0xfe => trail - 0x41,
        _ => return None,
    };
    Some(usize::from(lead.checked_sub(0x81)?) * 190 + usize::from(trail))
}

/// A table unit's character, none for 0.
fn lookup(table: &[u8], index: usize) -> Option<char> {
    let unit = table.get(2 * index..2 * index + 2)?;
    let unit = u16::from_le_bytes([unit[0], unit[1]]);
    char::from_u32(u32::from(unit)).filter(|_| unit != 0)
}
