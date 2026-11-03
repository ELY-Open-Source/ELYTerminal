use std::ops::Range;

use pulldown_cmark::{Event, Tag};

use super::{Reader, Toggle};
use crate::documents::blocks::{BlockData, BlockKind};

/// An HTML tag: where it starts and ends, its name, whether it closes, and its attributes' names.
struct Mark {
    at: usize,
    end: usize,
    closes: bool,
    name: String,
    attributes: Vec<String>,
}

/// The tags in `html`, past comments; a quoted value is read as a value.
fn marks(html: &str) -> Vec<Mark> {
    let bytes = html.as_bytes();
    let (mut marks, mut at) = (Vec::new(), 0);
    while let Some(found) = html[at..].find('<').map(|found| at + found) {
        if html[found..].starts_with("<!--") {
            at = html[found..]
                .find("-->")
                .map_or(html.len(), |end| found + end + 3);
            continue;
        }
        let closes = bytes.get(found + 1) == Some(&b'/');
        let name_at = found + 1 + usize::from(closes);
        let name_end = html[name_at..]
            .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '-'))
            .map_or(html.len(), |end| name_at + end);
        if !bytes.get(name_at).is_some_and(u8::is_ascii_alphabetic) {
            at = found + 1;
            continue;
        }
        let (end, attributes) = attributes(html, name_end);
        marks.push(Mark {
            at: found,
            end,
            closes,
            name: html[name_at..name_end].to_ascii_lowercase(),
            attributes,
        });
        at = end;
    }
    marks
}

/// Reads a tag's attributes from `from` to its `>`: where the tag ends, and the names.
fn attributes(html: &str, from: usize) -> (usize, Vec<String>) {
    let bytes = html.as_bytes();
    let (mut names, mut word, mut value, mut quote) = (Vec::new(), None, false, None);
    for (at, byte) in bytes.iter().copied().enumerate().skip(from) {
        if let Some(open) = quote {
            if byte == open {
                (quote, value) = (None, false);
            }
            continue;
        }
        let ends =
            byte.is_ascii_whitespace() || byte == b'>' || (!value && matches!(byte, b'=' | b'/'));
        if !ends {
            match byte {
                b'"' | b'\'' => quote = Some(byte),
                _ => word = word.or(Some(at)),
            }
            continue;
        }
        if let Some(start) = word.take() {
            if !value {
                names.push(html[start..at].to_ascii_lowercase());
            }
            value = false;
        }
        match byte {
            b'=' => value = true,
            b'>' => return (at + 1, names),
            _ => {}
        }
    }
    if let Some(start) = word.filter(|_| !value) {
        names.push(html[start..].to_ascii_lowercase());
    }
    (html.len(), names)
}

fn named<'a>(marks: &'a [Mark], name: &str, closes: bool, from: usize) -> Option<&'a Mark> {
    marks
        .iter()
        .find(|mark| mark.name == name && mark.closes == closes && mark.at >= from)
}

impl Reader<'_> {
    pub(super) fn html_block(&mut self, range: Range<usize>) {
        let source = self.source;
        let block = &source[range.clone()];
        let marks = marks(block);
        let first = marks
            .first()
            .filter(|mark| block[..mark.at].trim().is_empty() && mark.name == "details");
        let Some(first) = first.filter(|mark| !mark.closes) else {
            self.html = true;
            return self.put(BlockKind::Paragraph, block.trim().to_string());
        };
        let summary = named(&marks, "summary", false, first.end);
        let shut = summary.and_then(|open| named(&marks, "summary", true, open.end));
        let (title, head) = match (summary, shut) {
            (Some(open), Some(shut)) => (block[open.end..shut.at].trim(), shut.end),
            _ => ("", first.end),
        };
        let head = range.start + head;
        self.toggle = Some(Toggle {
            open: first.attributes.iter().any(|name| name == "open"),
            title: title.to_string(),
            body: head,
            depth: 0,
        });
        self.scan_toggle(head, range.end);
    }

    /// Skips a toggle's body, Markdown kept as written, to the `</details>` that closes it.
    pub(super) fn in_toggle(&mut self, event: Event, range: Range<usize>) {
        if let Event::Start(Tag::HtmlBlock) = event {
            self.scan_toggle(range.start, range.end);
        }
    }

    /// Follows `<details>` tags in `from..to`; the close that matches the toggle's open ends it.
    fn scan_toggle(&mut self, from: usize, to: usize) {
        let source = self.source;
        let Some(mut depth) = self.toggle.as_ref().map(|toggle| toggle.depth) else {
            return;
        };
        let mut closed = None;
        let marks = marks(&source[from..to]);
        for mark in marks.into_iter().filter(|mark| mark.name == "details") {
            match (mark.closes, depth) {
                (false, _) => depth += 1,
                (true, 0) => {
                    closed = Some(mark);
                    break;
                }
                (true, _) => depth -= 1,
            }
        }
        let Some(close) = closed else {
            if let Some(toggle) = &mut self.toggle {
                toggle.depth = depth;
            }
            return;
        };
        self.end_toggle(from + close.at);
        self.html = true;
        let rest = source[from + close.end..to].trim();
        if !rest.is_empty() {
            self.put(BlockKind::Paragraph, rest.to_string());
        }
    }

    fn end_toggle(&mut self, end: usize) {
        let Some(toggle) = self.toggle.take() else {
            return;
        };
        let body = self.source[toggle.body..end].trim().to_string();
        self.bodies.push(toggle.body..end);
        if let Some((_, _, parts)) = &mut self.quote {
            parts.push(body);
            return;
        }
        self.emit_item();
        let kind = BlockKind::Toggle(toggle.open);
        self.out.push(BlockData::new(kind, [toggle.title, body]));
    }

    /// A toggle the source never closes keeps the rest as its body.
    pub(super) fn unclosed(&mut self) {
        if let Some(toggle) = &self.toggle {
            log::error!(
                "block editor: the <details> at byte {} never closes; its body runs to the end",
                toggle.body
            );
            self.end_toggle(self.source.len());
        }
    }
}
