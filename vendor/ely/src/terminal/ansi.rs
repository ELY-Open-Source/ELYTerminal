use std::ops::{Index, Range};

use gpui::{
    App, FontStyle, FontWeight, HighlightStyle, IntoElement, ParentElement, RenderOnce,
    SharedString, StrikethroughStyle, Styled, StyledText, UnderlineStyle, Window, div,
};
use vte::ansi::{Attr, Color, Handler, Processor, Rgb};

use super::colors::Ink;
use crate::theme::{ActiveTheme, TextSize};

/// How text prints after the escape codes so far.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Pen {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub inverse: bool,
}

/// Collects the printed text and the pen each stretch of it took.
#[derive(Default)]
struct Printed {
    text: String,
    spans: Vec<(Range<usize>, Pen)>,
    pen: Pen,
}

impl Handler for Printed {
    fn input(&mut self, ch: char) {
        let start = self.text.len();
        self.text.push(ch);
        let end = self.text.len();
        match self.spans.last_mut() {
            Some((range, pen)) if range.end == start && *pen == self.pen => range.end = end,
            _ if self.pen == Pen::default() => {}
            _ => self.spans.push((start..end, self.pen)),
        }
    }

    fn linefeed(&mut self) {
        self.text.push('\n');
    }

    fn put_tab(&mut self, count: u16) {
        self.text.extend(std::iter::repeat_n('\t', count as usize));
    }

    fn terminal_attribute(&mut self, attr: Attr) {
        let pen = &mut self.pen;
        match attr {
            Attr::Reset => *pen = Pen::default(),
            Attr::Bold => pen.bold = true,
            Attr::Dim => pen.dim = true,
            Attr::Italic => pen.italic = true,
            Attr::Underline
            | Attr::DoubleUnderline
            | Attr::Undercurl
            | Attr::DottedUnderline
            | Attr::DashedUnderline => pen.underline = true,
            Attr::Reverse => pen.inverse = true,
            Attr::Strike => pen.strike = true,
            Attr::CancelBold => pen.bold = false,
            Attr::CancelBoldDim => (pen.bold, pen.dim) = (false, false),
            Attr::CancelItalic => pen.italic = false,
            Attr::CancelUnderline => pen.underline = false,
            Attr::CancelReverse => pen.inverse = false,
            Attr::CancelStrike => pen.strike = false,
            Attr::Foreground(color) => pen.fg = Some(color),
            Attr::Background(color) => pen.bg = Some(color),
            Attr::BlinkSlow
            | Attr::BlinkFast
            | Attr::CancelBlink
            | Attr::Hidden
            | Attr::CancelHidden
            | Attr::UnderlineColor(_) => {}
        }
    }
}

/// The text `source` prints, carriage returns and other codes gone, and the pens that are not plain.
pub(crate) fn printed(source: &str) -> (String, Vec<(Range<usize>, Pen)>) {
    let mut printed = Printed::default();
    let mut parser: Processor = Processor::new();
    parser.advance(&mut printed, source.as_bytes());
    (printed.text, printed.spans)
}

/// Plain text sets no colors through OSC 4, 10, 11.
struct Unset;

impl Index<usize> for Unset {
    type Output = Option<Rgb>;

    fn index(&self, _: usize) -> &Option<Rgb> {
        &None
    }
}

impl Pen {
    fn style(&self, ink: &Ink, rule: gpui::Pixels) -> HighlightStyle {
        let set = Unset;
        let paint =
            |color: Option<Color>, plain| color.map_or(plain, |color| ink.resolve(color, &set));
        let (mut fg, mut bg) = (paint(self.fg, ink.fg), paint(self.bg, ink.bg));
        if self.inverse {
            std::mem::swap(&mut fg, &mut bg);
        }
        if self.dim {
            fg = fg.opacity(0.7);
        }
        HighlightStyle {
            color: Some(fg),
            background_color: (self.bg.is_some() || self.inverse).then_some(bg),
            font_weight: self.bold.then_some(FontWeight::BOLD),
            font_style: self.italic.then_some(FontStyle::Italic),
            underline: self.underline.then_some(UnderlineStyle {
                thickness: rule,
                color: Some(fg),
                wavy: false,
            }),
            strikethrough: self.strike.then_some(StrikethroughStyle {
                thickness: rule,
                color: Some(fg),
            }),
            ..HighlightStyle::default()
        }
    }
}

/// Text colored by the escape codes in it, as a terminal would print it; other codes drop away.
#[derive(IntoElement)]
pub struct AnsiText {
    source: SharedString,
}

impl AnsiText {
    pub fn new(source: impl Into<SharedString>) -> Self {
        Self {
            source: source.into(),
        }
    }
}

impl RenderOnce for AnsiText {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let ink = Ink::new(&theme.colors);
        let (text, spans) = printed(&self.source);
        let rule = theme.underline_thickness();
        let styles: Vec<(Range<usize>, HighlightStyle)> = spans
            .into_iter()
            .map(|(range, pen)| (range, pen.style(&ink, rule)))
            .collect();
        div()
            .font_family(theme.mono_family.clone())
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(ink.fg)
            .child(StyledText::new(text).with_highlights(styles))
    }
}

#[cfg(test)]
mod tests {
    use vte::ansi::NamedColor;

    use super::*;

    #[test]
    fn codes_become_pens_and_drop_out_of_the_text() {
        let (text, spans) = printed("\x1b[1;31merror\x1b[0m: bad\r\n\x1b[4mnext\x1b[24m ok");
        assert_eq!(text, "error: bad\nnext ok");
        let red = Pen {
            fg: Some(Color::Named(NamedColor::Red)),
            bold: true,
            ..Pen::default()
        };
        let under = Pen {
            underline: true,
            ..Pen::default()
        };
        assert_eq!(spans, [(0..5, red), (11..15, under)]);
    }
}
