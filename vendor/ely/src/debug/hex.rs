use std::{ops::Range, rc::Rc};

use gpui::{
    App, ElementId, HoverListenerMode, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, StatefulInteractiveElement, Styled, Window, div, prelude::*, uniform_list,
};

use super::OnIndex;
use crate::theme::{ActiveTheme, TextSize};

/// Bytes a row shows.
const WIDTH: usize = 16;

/// How a byte reads as text: itself when it prints, a dot when not.
fn glyph(byte: u8) -> char {
    if byte.is_ascii_graphic() || byte == b' ' {
        byte as char
    } else {
        '·'
    }
}

/// What the bytes from `at` read as: the byte in hex, decimal, signed and binary, and the little-endian words that fit.
pub(crate) fn inspect(bytes: &[u8], at: usize) -> String {
    let byte = bytes[at];
    let mut parts = vec![
        format!("0x{byte:02X}"),
        byte.to_string(),
        format!("i8 {}", byte as i8),
        format!("0b{byte:08b}"),
    ];
    if let Some(word) = bytes.get(at..at + 2) {
        parts.push(format!("u16 {}", u16::from_le_bytes([word[0], word[1]])));
    }
    if let Some(word) = bytes.get(at..at + 4) {
        parts.push(format!(
            "u32 {}",
            u32::from_le_bytes([word[0], word[1], word[2], word[3]])
        ));
    }
    parts.join(" · ")
}

/// Memory as a hex dump: each row its address, sixteen bytes in fours, and the same bytes as text. A press selects a byte; the one under the pointer lights on both sides, and the selection reads below as numbers. Long dumps draw only the rows in view; it fills its box.
#[derive(IntoElement)]
pub struct HexViewer {
    id: ElementId,
    bytes: Rc<Vec<u8>>,
    base: u64,
    selected: Option<Range<usize>>,
    on_select: Option<OnIndex>,
}

impl HexViewer {
    pub fn new(id: impl Into<ElementId>, bytes: impl Into<Rc<Vec<u8>>>) -> Self {
        Self {
            id: id.into(),
            bytes: bytes.into(),
            base: 0,
            selected: None,
            on_select: None,
        }
    }

    /// The address of the first byte; the last must fit an address too.
    pub fn base(mut self, base: u64) -> Self {
        let last = self.bytes.len().saturating_sub(1) as u64;
        assert!(
            base.checked_add(last).is_some(),
            "a dump at {base:#X} runs past the last address"
        );
        self.base = base;
        self
    }

    pub fn selected(mut self, bytes: Range<usize>) -> Self {
        let inside = bytes.start < bytes.end && bytes.end <= self.bytes.len();
        if !inside {
            log::error!(
                "hex viewer: no bytes {bytes:?} in {}; none selected",
                self.bytes.len()
            );
        }
        self.selected = inside.then_some(bytes);
        self
    }

    pub fn on_select(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for HexViewer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let hovered =
            window.use_keyed_state((self.id.clone(), "hovered"), cx, |_, _| None::<usize>);
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let rows = self.bytes.len().div_ceil(WIDTH);
        let digits = format!(
            "{:X}",
            self.base + self.bytes.len().saturating_sub(1) as u64
        )
        .len()
        .max(8);
        let reading = self
            .selected
            .as_ref()
            .map(|range| inspect(&self.bytes, range.start));
        let (bytes, base, selected, on_select, id) = (
            self.bytes.clone(),
            self.base,
            self.selected.clone(),
            self.on_select.clone(),
            self.id.clone(),
        );
        let list = uniform_list((self.id.clone(), "rows"), rows, move |range, _, cx| {
            let colors = cx.theme().colors.clone();
            let lit = *hovered.read(cx);
            range
                .map(|row| {
                    let start = row * WIDTH;
                    let end = (start + WIDTH).min(bytes.len());
                    let wash = |at: usize| {
                        if selected.as_ref().is_some_and(|range| range.contains(&at)) {
                            Some(colors.selection)
                        } else if lit == Some(at) {
                            Some(colors.hover)
                        } else {
                            None
                        }
                    };
                    let cell = |at: usize, text: String, key: &str| {
                        let (pick, hover) = (on_select.clone(), hovered.clone());
                        div()
                            .id((id.clone(), format!("{key}-{at}")))
                            .px_0p5()
                            .when_some(wash(at), |cell, wash| cell.bg(wash))
                            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
                            .on_hover(move |on, _, cx| {
                                hover.update(cx, |hovered, cx| {
                                    let next = if *on { Some(at) } else { None };
                                    if *hovered != next && (*on || *hovered == Some(at)) {
                                        *hovered = next;
                                        cx.notify();
                                    }
                                })
                            })
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.prevent_default()
                            })
                            .when_some(pick, |cell, pick| {
                                cell.on_click(move |_, window, cx| pick(at, window, cx))
                            })
                            .child(text)
                    };
                    div()
                        .id(row)
                        .w_full()
                        .flex()
                        .gap_4()
                        .whitespace_nowrap()
                        .child(
                            div()
                                .text_color(colors.fg_subtle)
                                .child(format!("{:0digits$X}", base + start as u64)),
                        )
                        .child(div().flex().gap_2().children(
                            (start..end).collect::<Vec<_>>().chunks(4).map(|four| {
                                div().flex().children(
                                    four.iter()
                                        .map(|at| cell(*at, format!("{:02X}", bytes[*at]), "hex")),
                                )
                            }),
                        ))
                        .child(div().flex().text_color(colors.fg_muted).children(
                            (start..end).map(|at| cell(at, glyph(bytes[at]).to_string(), "text")),
                        ))
                })
                .collect()
        })
        .size_full();
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .font_family(theme.mono_family.clone())
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.fg)
                    .child(list),
            )
            .children(reading.map(|reading| {
                div()
                    .font_family(theme.mono_family.clone())
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.fg_muted)
                    .child(reading)
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_selection_off_the_bytes_selects_none() {
        let viewer = || HexViewer::new("hex", vec![1, 2, 3]);
        let (inside, at_end, past) = (0..2, 3..3, 2..9);
        assert_eq!(viewer().selected(inside.clone()).selected, Some(inside));
        assert_eq!(viewer().selected(at_end).selected, None);
        assert_eq!(viewer().selected(past).selected, None);
    }

    #[test]
    fn bytes_read_as_numbers_and_text() {
        let bytes = [0x2A, 0x01, 0x00, 0x00, 0xFF];
        assert_eq!(
            inspect(&bytes, 0),
            "0x2A · 42 · i8 42 · 0b00101010 · u16 298 · u32 298"
        );
        assert_eq!(
            inspect(&bytes, 4),
            "0xFF · 255 · i8 -1 · 0b11111111",
            "no word past the end"
        );
        assert_eq!((glyph(b'A'), glyph(0), glyph(b' ')), ('A', '·', ' '));
    }
}
