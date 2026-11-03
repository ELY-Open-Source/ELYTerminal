use std::ops::Range;

use gpui::{App, Entity};

use super::markdown::{Mark, edge, marks};
use crate::forms::TextInput;

/// A format the toolbars apply to markdown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Bold,
    Italic,
    Strike,
    Code,
    Link,
    Heading(u8),
    Quote,
    Bullet,
    Numbered,
    Task,
}

impl Format {
    /// The marker an inline format wraps its text in.
    fn marker(self) -> Option<&'static str> {
        match self {
            Self::Bold => Some("**"),
            Self::Italic => Some("*"),
            Self::Strike => Some("~~"),
            Self::Code => Some("`"),
            _ => None,
        }
    }

    /// The mark an inline format writes.
    fn mark(self) -> Option<Mark> {
        match self {
            Self::Bold => Some(Mark::Strong),
            Self::Italic => Some(Mark::Emphasis),
            Self::Strike => Some(Mark::Strike),
            Self::Code => Some(Mark::Code),
            _ => None,
        }
    }

    /// The prefix a line format starts its lines with; numbered lines count up from one.
    fn prefix(self, nth: usize) -> String {
        match self {
            Self::Heading(level) => format!("{} ", "#".repeat(level.clamp(1, 6) as usize)),
            Self::Quote => "> ".into(),
            Self::Bullet => "- ".into(),
            Self::Numbered => format!("{}. ", nth + 1),
            Self::Task => "- [ ] ".into(),
            _ => unreachable!("an inline format has no prefix"),
        }
    }
}

/// A numbered line's prefix, such as `12. `, in bytes; none for other lines.
fn number_prefix(line: &str) -> usize {
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 && line[digits..].starts_with(". ") {
        digits + 2
    } else {
        0
    }
}

/// Whether `line` already starts as `format` writes it.
fn has(format: Format, line: &str) -> bool {
    match format {
        Format::Task => line.starts_with("- [ ] ") || line.starts_with("- [x] "),
        Format::Bullet => {
            (line.starts_with("- ") || line.starts_with("* ")) && !has(Format::Task, line)
        }
        Format::Numbered => number_prefix(line) > 0,
        _ => line.starts_with(&format.prefix(0)),
    }
}

/// A line's own prefix: heading hashes, quote, bullet, number or task box, with its space.
fn line_prefix(line: &str) -> usize {
    let hashes = line.chars().take_while(|ch| *ch == '#').count();
    if (1..=6).contains(&hashes) && line[hashes..].starts_with(' ') {
        return hashes + 1;
    }
    for mark in ["- [ ] ", "- [x] ", "> ", "- ", "* "] {
        if line.starts_with(mark) {
            return mark.len();
        }
    }
    number_prefix(line)
}

/// The span of `format` that `selection` fills, but for other styles' markers: where it is written, and where its text sits.
fn own(
    text: &str,
    selection: &Range<usize>,
    format: Format,
) -> Option<(Range<usize>, Range<usize>)> {
    let wanted = format.mark().expect("an inline format has a mark");
    let only_markers = |part: &str| part.chars().all(|ch| matches!(ch, '*' | '_' | '~' | '`'));
    marks(text).into_iter().find_map(|(range, mark)| {
        if mark != wanted || selection.start < range.start || range.end < selection.end {
            return None;
        }
        let edge = edge(&text[range.clone()], mark);
        let inner = range.start + edge..range.end - edge;
        let fills = inner.start <= selection.start
            && selection.end <= inner.end
            && only_markers(&text[inner.start..selection.start])
            && only_markers(&text[selection.end..inner.end]);
        fills.then_some((range, inner))
    })
}

/// `text` with `format` applied over `selection`, and where the selection lands. An inline format wraps the selection, or unwraps it when already wrapped; an empty one leaves the caret between its markers. A line format prefixes each touched line, or takes the prefix off when every line has it.
pub fn apply(text: &str, selection: Range<usize>, format: Format) -> (String, Range<usize>) {
    assert!(
        selection.start <= selection.end && selection.end <= text.len(),
        "the selection {selection:?} lies outside {} bytes",
        text.len()
    );
    if format == Format::Link {
        let label = &text[selection.clone()];
        let linked = format!("{}[{label}]()", &text[..selection.start]);
        let caret = linked.len() - 1;
        return (format!("{linked}{}", &text[selection.end..]), caret..caret);
    }
    if let Some(marker) = format.marker() {
        let (before, after) = (&text[..selection.start], &text[selection.end..]);
        let inner = &text[selection.clone()];
        if selection.is_empty() && before.ends_with(marker) && after.starts_with(marker) {
            let start = selection.start - marker.len();
            let unwrapped = format!("{}{inner}{}", &before[..start], &after[marker.len()..]);
            return (unwrapped, start..selection.end - marker.len());
        }
        if let Some((range, text_range)) = own(text, &selection, format) {
            let shift = text_range.start - range.start;
            let unwrapped = format!(
                "{}{}{}",
                &text[..range.start],
                &text[text_range],
                &text[range.end..]
            );
            return (unwrapped, selection.start - shift..selection.end - shift);
        }
        let shift = marker.len();
        let wrapped = format!("{before}{marker}{inner}{marker}{after}");
        return (wrapped, selection.start + shift..selection.end + shift);
    }
    let first = text[..selection.start].rfind('\n').map_or(0, |at| at + 1);
    let last = text[selection.end..]
        .find('\n')
        .map_or(text.len(), |at| selection.end + at);
    let lines: Vec<&str> = text[first..last].split('\n').collect();
    let every = lines.iter().all(|line| has(format, line));
    let changed: Vec<String> = lines
        .iter()
        .enumerate()
        .map(|(nth, line)| {
            let bare = &line[line_prefix(line)..];
            if every {
                bare.to_string()
            } else {
                format!("{}{bare}", format.prefix(nth))
            }
        })
        .collect();
    let block = changed.join("\n");
    let out = format!("{}{block}{}", &text[..first], &text[last..]);
    (out, first..first + block.len())
}

/// The one stretch where `new` differs from `old`: the range it replaces and what replaces it.
fn changed<'a>(old: &str, new: &'a str) -> (Range<usize>, &'a str) {
    let head: usize = old
        .chars()
        .zip(new.chars())
        .take_while(|(was, now)| was == now)
        .map(|(was, _)| was.len_utf8())
        .sum();
    let tail: usize = old[head..]
        .chars()
        .rev()
        .zip(new[head..].chars().rev())
        .take_while(|(was, now)| was == now)
        .map(|(was, _)| was.len_utf8())
        .sum();
    (head..old.len() - tail, &new[head..new.len() - tail])
}

/// Applies `format` to the field's selection as one undo step, then selects what it formatted.
pub fn format(field: &Entity<TextInput>, format: Format, cx: &mut App) {
    field.update(cx, |input, cx| {
        let text = input.text().to_string();
        let (next, selection) = apply(&text, input.selection(), format);
        let (range, inserted) = changed(&text, &next);
        log::info!("format {format:?}: {range:?} becomes {inserted:?}");
        input.select(range, cx);
        input.insert(inserted, cx);
        input.select(selection, cx);
    });
}

/// The formats that hold all of `selection`, for a toolbar to show pressed.
pub fn active(text: &str, selection: Range<usize>) -> Vec<Format> {
    let mut on: Vec<Format> = marks(text)
        .into_iter()
        .filter(|(range, _)| range.start <= selection.start && selection.end <= range.end)
        .filter_map(|(_, mark)| match mark {
            Mark::Strong => Some(Format::Bold),
            Mark::Emphasis => Some(Format::Italic),
            Mark::Strike => Some(Format::Strike),
            Mark::Code => Some(Format::Code),
            Mark::Link => Some(Format::Link),
            Mark::Heading(level) => Some(Format::Heading(level)),
            Mark::Quote => Some(Format::Quote),
            Mark::Syntax => None,
        })
        .collect();
    let line = &text[text[..selection.start].rfind('\n').map_or(0, |at| at + 1)..];
    on.extend(
        [Format::Task, Format::Bullet, Format::Numbered]
            .into_iter()
            .find(|list| has(*list, line)),
    );
    on
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_formats_wrap_and_unwrap() {
        let (bold, selection) = apply("a lift b", 2..6, Format::Bold);
        assert_eq!((bold.as_str(), selection.clone()), ("a **lift** b", 4..8));
        assert_eq!(
            apply(&bold, selection, Format::Bold),
            ("a lift b".into(), 2..6)
        );
        assert_eq!(apply("ab", 1..1, Format::Code), ("a``b".into(), 2..2));
        assert_eq!(
            apply("see docs", 4..8, Format::Link),
            ("see [docs]()".into(), 11..11)
        );
    }

    #[test]
    fn a_format_unwraps_only_its_own_markers() {
        assert_eq!(
            apply("**bold**", 2..6, Format::Italic),
            ("***bold***".into(), 3..7)
        );
        assert_eq!(
            apply("a *it* b", 3..5, Format::Italic),
            ("a it b".into(), 2..4)
        );
        assert_eq!(apply("**bold**", 2..6, Format::Bold), ("bold".into(), 0..4));
    }

    #[test]
    fn line_formats_prefix_every_touched_line_or_take_it_off() {
        let text = "one\ntwo\nthree";
        let (listed, selection) = apply(text, 1..6, Format::Numbered);
        assert_eq!(listed, "1. one\n2. two\nthree");
        assert_eq!(apply(&listed, selection, Format::Numbered).0, text);
        assert_eq!(apply("## Old", 3..3, Format::Heading(1)).0, "# Old");
        assert_eq!(apply("- item", 2..2, Format::Task).0, "- [ ] item");
        assert_eq!(apply("- [x] done", 2..2, Format::Task).0, "done");
        assert_eq!(apply("- [ ] task", 2..2, Format::Bullet).0, "- task");
    }

    #[test]
    fn a_change_is_the_stretch_between_what_stayed() {
        assert_eq!(changed("a lift b", "a **lift** b"), (2..6, "**lift**"));
        assert_eq!(changed("same", "same"), (4..4, ""));
        assert_eq!(changed("héllo", "hello"), (1..3, "e"));
    }

    #[test]
    fn pressed_formats_hold_the_selection() {
        let text = "- [ ] a **bold** word";
        assert_eq!(active(text, 10..12), [Format::Bold, Format::Task]);
        assert_eq!(active("## Lift", 4..4), [Format::Heading(2)]);
        assert_eq!(active("12. twelve", 5..5), [Format::Numbered]);
    }
}
