use std::{
    ops::Range,
    sync::atomic::{AtomicU64, Ordering},
};

/// Source text and where each line starts, in bytes.
#[derive(Clone, Debug)]
pub struct Buffer {
    text: String,
    starts: Vec<usize>,
    stamp: u64,
}

impl PartialEq for Buffer {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
    }
}

impl Eq for Buffer {}

/// A fresh stamp: no two texts share one.
fn stamp() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

fn starts(text: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(text.match_indices('\n').map(|(at, _)| at + 1))
        .collect()
}

impl Buffer {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let starts = starts(&text);
        Self {
            text,
            starts,
            stamp: stamp(),
        }
    }

    /// Names this text; every edit gives a new one.
    pub(crate) fn stamp(&self) -> u64 {
        self.stamp
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// How many lines it holds; an empty buffer holds one.
    pub fn lines(&self) -> usize {
        self.starts.len()
    }

    /// A line's bytes, without its line break.
    pub fn line_range(&self, line: usize) -> Range<usize> {
        let start = self.starts[line];
        let end = self
            .starts
            .get(line + 1)
            .map_or(self.text.len(), |next| next - 1);
        start..end
    }

    pub fn line(&self, line: usize) -> &str {
        &self.text[self.line_range(line)]
    }

    /// The line holding `offset`.
    pub fn line_of(&self, offset: usize) -> usize {
        self.starts.partition_point(|start| *start <= offset) - 1
    }

    /// An offset's line, and its column in characters.
    pub fn point(&self, offset: usize) -> (usize, usize) {
        let line = self.line_of(offset);
        let column = self.text[self.starts[line]..offset].chars().count();
        (line, column)
    }

    /// The offset at a line and a column in characters, held to the line's end.
    pub fn offset(&self, line: usize, column: usize) -> usize {
        let range = self.line_range(line.min(self.lines() - 1));
        self.text[range.clone()]
            .char_indices()
            .nth(column)
            .map_or(range.end, |(at, _)| range.start + at)
    }

    /// The character after `offset`, or the end.
    pub fn next(&self, offset: usize) -> usize {
        self.text[offset..]
            .chars()
            .next()
            .map_or(offset, |ch| offset + ch.len_utf8())
    }

    /// The character before `offset`, or the start.
    pub fn previous(&self, offset: usize) -> usize {
        self.text[..offset]
            .chars()
            .next_back()
            .map_or(offset, |ch| offset - ch.len_utf8())
    }

    /// How far a line is indented, in characters.
    pub fn indent(&self, line: usize) -> usize {
        self.line(line)
            .chars()
            .take_while(|ch| *ch == ' ' || *ch == '\t')
            .count()
    }

    pub(crate) fn replace(&mut self, range: Range<usize>, text: &str) {
        assert!(
            range.start <= range.end
                && range.end <= self.text.len()
                && self.text.is_char_boundary(range.start)
                && self.text.is_char_boundary(range.end),
            "an edit covers whole characters inside the text: {range:?} of {}",
            self.text.len()
        );
        self.text.replace_range(range, text);
        self.starts = starts(&self.text);
        self.stamp = stamp();
    }
}

/// What kind of character a word motion sees.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Space,
    Word,
    Mark,
}

fn class(ch: char) -> Class {
    if ch.is_alphanumeric() || ch == '_' {
        Class::Word
    } else if ch.is_whitespace() {
        Class::Space
    } else {
        Class::Mark
    }
}

impl Buffer {
    /// The start of the word before `offset`, over any spaces first; a line start stops it.
    pub fn word_start(&self, offset: usize) -> usize {
        let line_start = self.starts[self.line_of(offset)];
        if offset == line_start {
            return self.previous(offset);
        }
        let mut chars = self.text[line_start..offset]
            .char_indices()
            .rev()
            .peekable();
        while chars
            .next_if(|(_, ch)| class(*ch) == Class::Space)
            .is_some()
        {}
        let Some(&(_, first)) = chars.peek() else {
            return line_start;
        };
        let kind = class(first);
        let mut at = offset;
        for (ix, ch) in chars {
            if class(ch) != kind {
                break;
            }
            at = line_start + ix;
        }
        at.min(offset)
    }

    /// The end of the word after `offset`, over any spaces first; a line end stops it.
    pub fn word_end(&self, offset: usize) -> usize {
        let line_end = self.line_range(self.line_of(offset)).end;
        if offset == line_end {
            return self.next(offset);
        }
        let mut chars = self.text[offset..line_end].char_indices().peekable();
        while chars
            .next_if(|(_, ch)| class(*ch) == Class::Space)
            .is_some()
        {}
        let Some(&(_, first)) = chars.peek() else {
            return line_end;
        };
        let kind = class(first);
        let mut at = line_end;
        for (ix, ch) in chars {
            if class(ch) != kind {
                at = offset + ix;
                break;
            }
        }
        at
    }

    /// The word around `offset`, or the run of marks or spaces it sits in.
    pub fn word_at(&self, offset: usize) -> Range<usize> {
        let range = self.line_range(self.line_of(offset));
        let line = &self.text[range.clone()];
        let at = offset - range.start;
        let kind = line[at..]
            .chars()
            .next()
            .or_else(|| line[..at].chars().next_back())
            .map(class);
        let Some(kind) = kind else {
            return offset..offset;
        };
        let start = line[..at]
            .char_indices()
            .rev()
            .take_while(|(_, ch)| class(*ch) == kind)
            .last()
            .map_or(at, |(ix, _)| ix);
        let end = line[at..]
            .char_indices()
            .find(|(_, ch)| class(*ch) != kind)
            .map_or(line.len(), |(ix, _)| at + ix);
        range.start + start..range.start + end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_split_at_breaks_and_points_count_characters() {
        let buffer = Buffer::new("fn main() {\n    let π = 3.14;\n}");
        assert_eq!(buffer.lines(), 3);
        assert_eq!(buffer.line(1), "    let π = 3.14;");
        let pi = buffer.text().find('π').expect("pi");
        assert_eq!(buffer.point(pi), (1, 8));
        assert_eq!(
            buffer.point(pi + 'π'.len_utf8()),
            (1, 9),
            "a column is a character"
        );
        assert_eq!(buffer.offset(1, 9), pi + 'π'.len_utf8());
        assert_eq!(buffer.offset(0, 99), 11, "held to the line's end");
        assert_eq!(Buffer::new("").lines(), 1);
        assert_eq!(buffer.indent(1), 4);
    }

    #[test]
    fn edits_keep_line_starts_in_step() {
        let mut buffer = Buffer::new("a\nb");
        buffer.replace(1..1, "\nc");
        assert_eq!(buffer.text(), "a\nc\nb");
        assert_eq!(buffer.lines(), 3);
        assert_eq!(buffer.line(2), "b");
    }

    #[test]
    fn words_stop_at_their_class_and_at_line_edges() {
        let buffer = Buffer::new("let total = price * 2;\nnext");
        assert_eq!(buffer.word_start(9), 4, "back to the start of total");
        assert_eq!(buffer.word_start(4), 0, "over the space to let");
        assert_eq!(buffer.word_end(4), 9);
        assert_eq!(buffer.word_end(9), 11, "over the space, then the mark");
        assert_eq!(buffer.word_end(22), 23, "a line end steps to the next line");
        assert_eq!(buffer.word_at(6), 4..9);
        assert_eq!(buffer.word_at(10), 10..11, "a mark is its own run");
    }
}
