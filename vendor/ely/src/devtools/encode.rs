use gpui::{
    App, AppContext as _, ClipboardItem, ElementId, Entity, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div,
};

use super::request::percent;
use crate::{
    buttons::{IconButton, SegmentedControl},
    feedback::InlineMessage,
    forms::{Input, TextInput},
    primitives::{IconName, Severity},
    theme::{ActiveTheme, Radius, TextSize},
    typography::literal,
};

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Bytes in Base64, padded with `=`.
pub fn base64(bytes: &[u8]) -> String {
    bytes
        .chunks(3)
        .flat_map(|chunk| {
            let word = chunk.iter().enumerate().fold(0u32, |word, (ix, byte)| {
                word | (*byte as u32) << (16 - 8 * ix)
            });
            (0..4).map(move |ix| match ix <= chunk.len() {
                true => ALPHABET[(word >> (18 - 6 * ix) & 63) as usize] as char,
                false => '=',
            })
        })
        .collect()
}

/// Base64 back to bytes; spaces are skipped, and any other stray character or a bad length is an error.
pub fn unbase64(text: &str) -> Result<Vec<u8>, String> {
    let clean: Vec<u8> = text
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    if !clean.len().is_multiple_of(4) {
        return Err(format!(
            "Base64 comes in fours; this is {} long",
            clean.len()
        ));
    }
    let (mut out, quads) = (Vec::new(), clean.len() / 4);
    for (at, quad) in clean.chunks(4).enumerate() {
        let pads = quad.iter().rev().take_while(|byte| **byte == b'=').count();
        if pads > 2 || (pads > 0 && at + 1 < quads) {
            return Err("= pads only the last four, twice at most".into());
        }
        let mut word = 0u32;
        for (ix, byte) in quad.iter().enumerate() {
            let value = match byte {
                b'=' if ix >= 4 - pads => 0,
                _ => ALPHABET
                    .iter()
                    .position(|each| each == byte)
                    .ok_or_else(|| format!("{:?} is not Base64", *byte as char))?
                    as u32,
            };
            word = word << 6 | value;
        }
        out.extend(word.to_be_bytes()[1..4 - pads].iter());
    }
    Ok(out)
}

/// Bytes as two hex digits each.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Hex back to bytes, two digits a byte; spaces are skipped.
pub fn unhex(text: &str) -> Result<Vec<u8>, String> {
    let clean: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    if !clean.len().is_multiple_of(2) {
        return Err("Hex comes in pairs of digits".to_string());
    }
    clean
        .chunks(2)
        .map(|pair| {
            let digits: String = pair.iter().collect();
            byte_of(&digits).ok_or_else(|| format!("{digits:?} is not hex"))
        })
        .collect()
}

/// The byte two hex digits spell; a sign or anything else is none.
fn byte_of(digits: &str) -> Option<u8> {
    (digits.len() == 2 && digits.chars().all(|ch| ch.is_ascii_hexdigit()))
        .then(|| u8::from_str_radix(digits, 16).expect("two hex digits read"))
}

/// Percent-encoding undone: `%XX` becomes its byte and `+` a space.
pub fn unpercent(text: &str) -> Result<Vec<u8>, String> {
    let bytes = text.as_bytes();
    let (mut out, mut ix) = (Vec::new(), 0);
    while ix < bytes.len() {
        match bytes[ix] {
            b'%' => {
                let digits = text
                    .get(ix + 1..ix + 3)
                    .ok_or("a % needs two hex digits after it")?;
                out.push(byte_of(digits).ok_or_else(|| format!("%{digits} is not an escape"))?);
                ix += 3;
            }
            b'+' => {
                out.push(b' ');
                ix += 1;
            }
            byte => {
                out.push(byte);
                ix += 1;
            }
        }
    }
    Ok(out)
}

/// What an encoder turns text into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code {
    Base64,
    Url,
    Hex,
}

impl Code {
    const ALL: [Code; 3] = [Code::Base64, Code::Url, Code::Hex];

    fn name(self) -> &'static str {
        match self {
            Code::Base64 => "Base64",
            Code::Url => "URL",
            Code::Hex => "Hex",
        }
    }

    /// `text` encoded, or decoded back to UTF-8 text.
    pub fn run(self, text: &str, encode: bool) -> Result<String, String> {
        if encode {
            return Ok(match self {
                Code::Base64 => base64(text.as_bytes()),
                Code::Url => percent(text),
                Code::Hex => hex(text.as_bytes()),
            });
        }
        let bytes = match self {
            Code::Base64 => unbase64(text)?,
            Code::Url => unpercent(text)?,
            Code::Hex => unhex(text)?,
        };
        String::from_utf8(bytes).map_err(|_| "The bytes are not UTF-8 text".to_string())
    }
}

/// The encoder's own field, code and way.
struct Desk {
    input: Entity<TextInput>,
    code: Code,
    encode: bool,
}

/// Text in, encoded or decoded out, as Base64, for a URL, or as hex; what fails to decode says why, and the result copies.
#[derive(IntoElement)]
pub struct Encoder {
    id: ElementId,
    text: SharedString,
}

impl Encoder {
    /// The text it starts with.
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
        }
    }
}

impl RenderOnce for Encoder {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let text = self.text.to_string();
        let desk = window.use_keyed_state((self.id.clone(), "desk"), cx, move |window, cx| Desk {
            input: cx.new(|cx| {
                let mut input = TextInput::new(window, cx).multi_line(3, 8);
                input.set_text(text, cx);
                input
            }),
            code: Code::Base64,
            encode: true,
        });
        let (input, code, encode) = {
            let desk = desk.read(cx);
            (desk.input.clone(), desk.code, desk.encode)
        };
        let out = code.run(input.read(cx).text(), encode);
        let theme = cx.theme();
        let (coded, way) = (desk.clone(), desk.clone());
        let result = match out {
            Err(why) => InlineMessage::new(Severity::Danger, why).into_any_element(),
            Ok(text) => {
                let copied = text.clone();
                div()
                    .flex()
                    .items_start()
                    .gap_2()
                    .p_3()
                    .rounded(theme.radius(Radius::Md))
                    .bg(theme.colors.sunken)
                    .child(
                        literal(div())
                            .flex_1()
                            .min_w_0()
                            .font_family(theme.mono_family.clone())
                            .text_size(theme.text_size(TextSize::Sm))
                            .child(text),
                    )
                    .child(
                        IconButton::new((self.id.clone(), "copy"), IconName::Copy)
                            .tooltip("Copy the result")
                            .on_click(move |_, _, cx| {
                                log::info!("encoder: copied");
                                cx.write_to_clipboard(ClipboardItem::new_string(copied.clone()))
                            }),
                    )
                    .into_any_element()
            }
        };
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        Code::ALL
                            .iter()
                            .fold(
                                SegmentedControl::new((self.id.clone(), "code"), code.name()),
                                |control, code| control.segment(code.name(), code.name(), None),
                            )
                            .on_change(move |name, _, cx| {
                                coded.update(cx, |desk, cx| {
                                    desk.code = *Code::ALL
                                        .iter()
                                        .find(|code| code.name() == name.as_ref())
                                        .expect("a code the control offers");
                                    cx.notify();
                                })
                            }),
                    )
                    .child(
                        SegmentedControl::new(
                            (self.id, "way"),
                            if encode { "encode" } else { "decode" },
                        )
                        .segment("encode", "Encode", None)
                        .segment("decode", "Decode", None)
                        .on_change(move |value, _, cx| {
                            way.update(cx, |desk, cx| {
                                desk.encode = match value.as_ref() {
                                    "encode" => true,
                                    "decode" => false,
                                    other => panic!("encoder: no way named {other}"),
                                };
                                cx.notify();
                            })
                        }),
                    ),
            )
            .child(Input::new(&input))
            .child(result)
    }
}

#[cfg(test)]
mod tests {
    use super::{Code, base64, hex, unbase64, unhex, unpercent};

    #[test]
    fn base64_round_trips_with_padding() {
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"M"), "TQ==");
        assert_eq!(unbase64("TWE=").expect("base64"), b"Ma");
        assert_eq!(unbase64("TQ ==").expect("spaces skip"), b"M");
        assert!(unbase64("TW!u").is_err());
        assert!(unbase64("TWE").is_err());
        assert!(unbase64("T===").is_err(), "three pads say nothing");
        assert!(unbase64("TQ==TWFu").is_err(), "pads end the text");
    }

    #[test]
    fn a_sign_is_no_hex_digit() {
        assert!(unhex("+f").is_err());
        assert!(unpercent("%+f").is_err());
        assert_eq!(unhex("0f").expect("hex"), [15]);
    }

    #[test]
    fn hex_and_url_round_trip_and_name_their_faults() {
        assert_eq!(hex(b"hi"), "6869");
        assert_eq!(unhex("68 69").expect("hex"), b"hi");
        assert!(unhex("6").is_err());
        assert_eq!(unpercent("a%20b+c").expect("url"), b"a b c");
        assert!(unpercent("%2").is_err());
        assert_eq!(
            Code::Url.run("a b/é", true).expect("encodes"),
            "a%20b%2F%C3%A9"
        );
        assert_eq!(
            Code::Base64.run("//8=", false),
            Err("The bytes are not UTF-8 text".to_string())
        );
    }
}
