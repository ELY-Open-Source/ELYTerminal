use std::{ops::Range, time::Duration};

use unicode_segmentation::UnicodeSegmentation;
use web_time::Instant;

pub(crate) fn prev_grapheme(text: &str, offset: usize) -> usize {
    text.grapheme_indices(true)
        .rev()
        .find_map(|(ix, _)| (ix < offset).then_some(ix))
        .unwrap_or(0)
}

pub(crate) fn next_grapheme(text: &str, offset: usize) -> usize {
    text.grapheme_indices(true)
        .find_map(|(ix, _)| (ix > offset).then_some(ix))
        .unwrap_or(text.len())
}

fn words(text: &str) -> impl DoubleEndedIterator<Item = (usize, &str)> {
    text.split_word_bound_indices()
        .filter(|(_, word)| !word.trim().is_empty())
}

/// Start of the word before `offset`.
pub(crate) fn prev_word(text: &str, offset: usize) -> usize {
    words(text)
        .rev()
        .find_map(|(ix, _)| (ix < offset).then_some(ix))
        .unwrap_or(0)
}

/// End of the word after `offset`.
pub(crate) fn next_word(text: &str, offset: usize) -> usize {
    words(text)
        .find_map(|(ix, word)| (ix + word.len() > offset).then_some(ix + word.len()))
        .unwrap_or(text.len())
}

/// The word around `offset`, for double-click selection.
pub(crate) fn word_at(text: &str, offset: usize) -> Range<usize> {
    text.split_word_bound_indices()
        .find(|(ix, word)| *ix <= offset && offset < ix + word.len())
        .map(|(ix, word)| ix..ix + word.len())
        .unwrap_or(offset..offset)
}

pub(crate) fn line_start(text: &str, offset: usize) -> usize {
    text[..offset].rfind('\n').map_or(0, |ix| ix + 1)
}

pub(crate) fn line_end(text: &str, offset: usize) -> usize {
    text[offset..]
        .find('\n')
        .map_or(text.len(), |ix| offset + ix)
}

pub(crate) fn to_utf16(text: &str, offset: usize) -> usize {
    text[..offset].chars().map(char::len_utf16).sum()
}

pub(crate) fn from_utf16(text: &str, offset: usize) -> usize {
    let mut units = 0;
    for (ix, ch) in text.char_indices() {
        if units >= offset {
            return ix;
        }
        units += ch.len_utf16();
    }
    text.len()
}

/// Display offset of a content offset when each grapheme shows as `bullet`; an offset the text lost clamps into it.
pub(crate) fn masked_offset(text: &str, offset: usize, bullet: char) -> usize {
    text[..text.floor_char_boundary(offset)]
        .graphemes(true)
        .count()
        * bullet.len_utf8()
}

/// Content offset of a display offset in masked text.
pub(crate) fn unmasked_offset(text: &str, display: usize, bullet: char) -> usize {
    let count = display / bullet.len_utf8();
    text.grapheme_indices(true)
        .nth(count)
        .map_or(text.len(), |(ix, _)| ix)
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Snapshot {
    pub text: String,
    pub selection: Range<usize>,
}

/// Typing within this pause joins one undo step.
const JOIN: Duration = Duration::from_millis(900);

/// Undo and redo as whole snapshots of what an editor holds.
pub struct History<S> {
    undo: Vec<S>,
    redo: Vec<S>,
    typing_since: Option<Instant>,
}

impl<S> Default for History<S> {
    fn default() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            typing_since: None,
        }
    }
}

impl<S> History<S> {
    /// Records the state before an edit. Typing in one burst makes one step.
    pub fn record(&mut self, before: S, typing: bool) {
        let joins = typing && self.typing_since.is_some_and(|at| at.elapsed() < JOIN);
        if !joins {
            self.undo.push(before);
        }
        self.typing_since = typing.then(Instant::now);
        self.redo.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self, current: S) -> Option<S> {
        let previous = self.undo.pop()?;
        self.redo.push(current);
        self.typing_since = None;
        Some(previous)
    }

    pub fn redo(&mut self, current: S) -> Option<S> {
        let next = self.redo.pop()?;
        self.undo.push(current);
        self.typing_since = None;
        Some(next)
    }

    /// The size all steps hold, as `size` measures one.
    pub fn held(&self, size: impl Fn(&S) -> usize) -> usize {
        self.undo.iter().chain(&self.redo).map(size).sum()
    }

    /// Drops oldest undo, then farthest redo, past `budget`; how many.
    pub fn trim(&mut self, budget: usize, size: impl Fn(&S) -> usize) -> usize {
        let mut held = self.held(&size);
        let (mut undone, mut redone) = (0, 0);
        while held > budget && undone < self.undo.len() {
            held -= size(&self.undo[undone]);
            undone += 1;
        }
        while held > budget && redone < self.redo.len() {
            held -= size(&self.redo[redone]);
            redone += 1;
        }
        self.undo.drain(..undone);
        self.redo.drain(..redone);
        undone + redone
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trimming_drops_the_oldest_steps_past_the_budget() {
        let mut history = History::default();
        for step in ["aaaa", "bbbb", "cccc"] {
            history.record(step.to_string(), false);
        }
        assert_eq!(history.trim(9, String::len), 1, "12 bytes over 9");
        assert_eq!(history.undo("now".into()), Some("cccc".to_string()));
        assert_eq!(history.undo("cccc".into()), Some("bbbb".to_string()));
        assert_eq!(history.undo("bbbb".into()), None, "the oldest went");
        assert_eq!(history.trim(100, String::len), 0);
    }

    #[test]
    fn trimming_drops_the_farthest_redo_when_undo_is_spent() {
        let mut history = History::default();
        history.record("small".to_string(), false);
        // Undo leaves the large text to redo.
        assert_eq!(history.undo("x".repeat(20)), Some("small".to_string()));
        history.redo.insert(0, "y".repeat(20));
        assert_eq!(history.trim(25, String::len), 1);
        assert_eq!(
            history.redo("small".into()),
            Some("x".repeat(20)),
            "the nearest stays"
        );
        assert_eq!(history.redo("x".repeat(20)), None);
    }

    #[test]
    fn graphemes_keep_clusters_whole() {
        let text = "ae\u{301}🇯🇵z";
        assert_eq!(next_grapheme(text, 0), 1);
        assert_eq!(next_grapheme(text, 1), 4);
        assert_eq!(next_grapheme(text, 4), 12);
        assert_eq!(prev_grapheme(text, 12), 4);
        assert_eq!(prev_grapheme(text, 0), 0);
    }

    #[test]
    fn words_skip_spaces_and_punctuation_stops() {
        let text = "hello  brave, new";
        assert_eq!(next_word(text, 0), 5);
        assert_eq!(next_word(text, 5), 12);
        assert_eq!(prev_word(text, 17), 14);
        assert_eq!(prev_word(text, 14), 12);
        assert_eq!(word_at(text, 8), 7..12);
    }

    #[test]
    fn lines_and_utf16_round_trip() {
        let text = "one\ntwo😀\nthree";
        assert_eq!(line_start(text, 6), 4);
        assert_eq!(line_end(text, 6), 11);
        let emoji_end = 11;
        assert_eq!(to_utf16(text, emoji_end), 9);
        assert_eq!(from_utf16(text, 9), emoji_end);
    }

    #[test]
    fn masking_maps_graphemes_to_bullets() {
        let text = "pa😀s";
        assert_eq!(masked_offset(text, 6, '•'), 9);
        assert_eq!(unmasked_offset(text, 9, '•'), 6);
        assert_eq!(unmasked_offset(text, 12, '•'), text.len());
        assert_eq!(
            masked_offset("a", 2, '•'),
            3,
            "an offset past the text clamps"
        );
        assert_eq!(
            masked_offset(text, 4, '•'),
            6,
            "inside a character, before it"
        );
    }

    #[test]
    fn typing_joins_one_step_and_redo_clears_on_edit() {
        let snap = |text: &str| Snapshot {
            text: text.into(),
            selection: text.len()..text.len(),
        };
        let mut history = History::<Snapshot>::default();
        history.record(snap(""), true);
        history.record(snap("a"), true);
        history.record(snap("ab"), false);
        assert_eq!(history.undo(snap("")).unwrap(), snap("ab"));
        assert_eq!(history.undo(snap("ab")).unwrap(), snap(""));
        assert!(history.undo(snap("")).is_none());
        assert_eq!(history.redo(snap("")).unwrap(), snap("ab"));
        history.record(snap("ab"), false);
        assert!(history.redo(snap("abc")).is_none());
    }
}
