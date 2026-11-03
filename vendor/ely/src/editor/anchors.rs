use std::ops::Range;

use super::state::{CodeEditor, Marks};

/// Where `offset` lands after `edits`, sorted and apart, each a replaced range and the length written; none when an edit replaced it.
fn moved(offset: usize, edits: &[(Range<usize>, usize)]) -> Option<usize> {
    let mut at = offset as isize;
    for (range, written) in edits {
        if offset <= range.start {
            break;
        }
        if offset < range.end {
            return None;
        }
        at += *written as isize - range.len() as isize;
    }
    Some(at as usize)
}

/// Where the character at `offset` lands; text written at it goes before.
fn carried(offset: usize, edits: &[(Range<usize>, usize)]) -> Option<usize> {
    let mut at = offset as isize;
    for (range, written) in edits {
        if offset < range.start {
            break;
        }
        if offset < range.end {
            return None;
        }
        at += *written as isize - range.len() as isize;
    }
    Some(at as usize)
}

/// A range through `edits`; none when an edit cut into it.
fn moved_range(range: &Range<usize>, edits: &[(Range<usize>, usize)]) -> Option<Range<usize>> {
    Some(moved(range.start, edits)?..moved(range.end, edits)?)
}

/// Carries the owner's offsets through an edit of the text, dropping what it replaced.
pub(crate) fn shift(marks: &mut Marks, edits: &[(Range<usize>, usize)]) {
    marks.diagnostics.retain_mut(|diagnostic| {
        moved_range(&diagnostic.range, edits)
            .map(|range| diagnostic.range = range)
            .is_some()
    });
    marks.backgrounds.retain_mut(|(range, _)| {
        moved_range(range, edits)
            .map(|moved| *range = moved)
            .is_some()
    });
    marks.hints.retain_mut(|hint| {
        moved(hint.offset, edits)
            .map(|at| hint.offset = at)
            .is_some()
    });
    marks.ghost = marks.ghost.take().and_then(|mut ghost| {
        ghost.offset = moved(ghost.offset, edits)?;
        Some(ghost)
    });
    for anchor in &mut marks.anchors {
        *anchor = anchor.and_then(|at| carried(at, edits));
    }
}

/// Drops the owner's offsets when the whole text changes; they named the old one.
pub(crate) fn clear(marks: &mut Marks) {
    marks.diagnostics.clear();
    marks.backgrounds.clear();
    marks.hints.clear();
    marks.ghost = None;
    marks.anchors.fill(None);
}

impl CodeEditor {
    /// Offsets the editor carries through each edit.
    pub fn set_anchors(&mut self, offsets: impl IntoIterator<Item = usize>) {
        let text = self.buffer.text();
        let whole = |at: usize| {
            let fits = text.is_char_boundary(at);
            if !fits {
                log::error!("code editor: anchor {at} splits a character");
            }
            fits.then_some(at)
        };
        self.marks.anchors = offsets.into_iter().map(whole).collect();
    }

    /// Where each anchor is now; none once an edit took it.
    pub fn anchors(&self) -> &[Option<usize>] {
        &self.marks.anchors
    }
}

/// Whether `range` runs forward between whole characters of `text`.
pub(crate) fn fits(text: &str, range: &Range<usize>) -> bool {
    range.start <= range.end
        && text.is_char_boundary(range.start)
        && text.is_char_boundary(range.end)
}

#[cfg(test)]
mod tests {
    use super::{carried, fits, moved};

    #[test]
    fn offsets_ride_their_edits() {
        let edits = [(2..4, 1), (6..6, 3)];
        let cut = 0..2;
        assert_eq!(moved(1, &edits), Some(1), "before every edit");
        assert_eq!(moved(2, &edits), Some(2), "at an edit's start");
        assert_eq!(moved(3, &edits), None, "inside what it replaced");
        assert_eq!(moved(5, &edits), Some(4), "past it, by its change");
        assert_eq!(
            moved(6, &edits),
            Some(5),
            "at an insertion, before what it adds"
        );
        assert_eq!(moved(8, &edits), Some(10));
        assert!(fits("héllo", &(1..3)) && !fits("héllo", &cut) && !fits("ab", &(1..9)));
    }

    #[test]
    fn anchors_go_after_text_written_at_them() {
        let edits = [(2..4, 1), (6..6, 3)];
        assert_eq!(carried(2, &edits), None, "its character replaced");
        assert_eq!(carried(4, &edits), Some(3), "just past a replace");
        assert_eq!(carried(6, &edits), Some(8), "after an insertion at it");
        assert_eq!(carried(1, &edits), Some(1));
    }
}
