use std::ops::Range;

use gpui::SharedString;
use similar::{ChangeTag, TextDiff, udiff::UnifiedHunkHeader};

/// Whether a diff line stayed, came in, or went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    Same,
    Added,
    Removed,
}

/// A diff line: kind, numbers from one, text, changed words.
#[derive(Clone, Debug, PartialEq)]
pub struct DiffLine {
    pub kind: LineKind,
    pub old: Option<usize>,
    pub new: Option<usize>,
    pub text: SharedString,
    pub words: Vec<Range<usize>>,
}

/// A hunk of changes in context, or folded unchanged lines.
#[derive(Clone, Debug, PartialEq)]
pub enum Stretch {
    Hunk {
        /// `@@ -old +new @@`, from one.
        header: SharedString,
        lines: Vec<DiffLine>,
    },
    Folded(Vec<DiffLine>),
}

fn same(text: &str, old: usize, new: usize) -> DiffLine {
    DiffLine {
        kind: LineKind::Same,
        old: Some(old + 1),
        new: Some(new + 1),
        text: text.trim_end_matches(['\n', '\r']).to_string().into(),
        words: Vec::new(),
    }
}

/// `old` against `new`, `context` lines per hunk, the rest folded.
pub fn diff(old: &str, new: &str, context: usize) -> Vec<Stretch> {
    let diff = TextDiff::from_lines(old, new);
    let (olds, news): (Vec<&str>, Vec<&str>) = (
        diff.iter_old_slices().collect(),
        diff.iter_new_slices().collect(),
    );
    let mut out = Vec::new();
    let mut at = (0, 0);
    for group in diff.grouped_ops(context) {
        let (first, last) = (
            group.first().expect("a group holds ops"),
            group.last().expect("a group holds ops"),
        );
        let (old_start, new_start) = (first.old_range().start, first.new_range().start);
        if old_start > at.0 {
            out.push(Stretch::Folded(
                (at.0..old_start)
                    .zip(at.1..new_start)
                    .map(|(old, new)| same(olds[old], old, new))
                    .collect(),
            ));
        }
        let (old_end, new_end) = (last.old_range().end, last.new_range().end);
        let mut lines = Vec::new();
        for op in &group {
            for change in diff.iter_inline_changes(op) {
                let (mut text, mut words) = (String::new(), Vec::new());
                for (emphasized, part) in change.iter_strings_lossy() {
                    let part = part.trim_end_matches(['\n', '\r']);
                    if emphasized && !part.is_empty() {
                        words.push(text.len()..text.len() + part.len());
                    }
                    text.push_str(part);
                }
                let kind = match change.tag() {
                    ChangeTag::Equal => LineKind::Same,
                    ChangeTag::Insert => LineKind::Added,
                    ChangeTag::Delete => LineKind::Removed,
                };
                lines.push(DiffLine {
                    kind,
                    old: change.old_index().map(|ix| ix + 1),
                    new: change.new_index().map(|ix| ix + 1),
                    text: text.into(),
                    words: if kind == LineKind::Same {
                        Vec::new()
                    } else {
                        words
                    },
                });
            }
        }
        let header = UnifiedHunkHeader::new(&group).to_string();
        out.push(Stretch::Hunk {
            header: header.into(),
            lines,
        });
        at = (old_end, new_end);
    }
    if at.0 < olds.len() {
        out.push(Stretch::Folded(
            (at.0..olds.len())
                .zip(at.1..news.len())
                .map(|(old, new)| same(olds[old], old, new))
                .collect(),
        ));
    }
    out
}

/// Lines added and removed from `old` to `new`.
pub fn stat(old: &str, new: &str) -> (usize, usize) {
    let diff = TextDiff::from_lines(old, new);
    diff.iter_all_changes()
        .fold((0, 0), |(added, removed), change| match change.tag() {
            ChangeTag::Insert => (added + 1, removed),
            ChangeTag::Delete => (added, removed + 1),
            ChangeTag::Equal => (added, removed),
        })
}

/// A hunk's rows side by side, as places in `lines`.
pub(crate) fn pair_places(lines: &[DiffLine]) -> Vec<(Option<usize>, Option<usize>)> {
    let mut out = Vec::new();
    let mut ix = 0;
    while ix < lines.len() {
        if lines[ix].kind == LineKind::Same {
            out.push((Some(ix), Some(ix)));
            ix += 1;
            continue;
        }
        let run = |from: usize, kind| {
            from..from
                + lines[from..]
                    .iter()
                    .take_while(|line| line.kind == kind)
                    .count()
        };
        let removed = run(ix, LineKind::Removed);
        let added = run(removed.end, LineKind::Added);
        ix = added.end;
        for row in 0..removed.len().max(added.len()) {
            let at = |range: &Range<usize>| (row < range.len()).then(|| range.start + row);
            out.push((at(&removed), at(&added)));
        }
    }
    out
}

/// A hunk's lines side by side: removals beside following additions.
pub fn pairs(lines: &[DiffLine]) -> Vec<(Option<&DiffLine>, Option<&DiffLine>)> {
    pair_places(lines)
        .into_iter()
        .map(|(left, right)| (left.map(|ix| &lines[ix]), right.map(|ix| &lines[ix])))
        .collect()
}

/// Marks words changed between each removal and its addition.
pub fn mark_words(lines: &mut [DiffLine]) {
    for (left, right) in pair_places(lines) {
        let (Some(left), Some(right)) = (left, right) else {
            continue;
        };
        if left == right {
            continue;
        }
        let (old, new) = (lines[left].text.clone(), lines[right].text.clone());
        let diff = TextDiff::from_unicode_words(old.as_ref(), new.as_ref());
        let (mut at, mut words, mut kept) = ((0, 0), (Vec::new(), Vec::new()), false);
        for change in diff.iter_all_changes() {
            let (value, len) = (change.value(), change.value().len());
            let real = !value.trim().is_empty();
            match change.tag() {
                ChangeTag::Equal => {
                    kept |= real;
                    at = (at.0 + len, at.1 + len);
                }
                ChangeTag::Delete => {
                    words.0.extend(real.then(|| at.0..at.0 + len));
                    at.0 += len;
                }
                ChangeTag::Insert => {
                    words.1.extend(real.then(|| at.1..at.1 + len));
                    at.1 += len;
                }
            }
        }
        // Lines with nothing in common mark no words.
        if kept {
            (lines[left].words, lines[right].words) = words;
        }
    }
}

/// A side-by-side row's numbers: old left, new right.
pub(crate) fn side_numbers(
    left: Option<&DiffLine>,
    right: Option<&DiffLine>,
) -> (Option<usize>, Option<usize>) {
    (
        left.and_then(|line| line.old),
        right.and_then(|line| line.new),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_change_shows_its_words_and_numbers() {
        let stretches = diff("a\nlet x = 1;\nc\n", "a\nlet x = 2;\nc\nd\n", 1);
        let [Stretch::Hunk { header, lines }] = stretches.as_slice() else {
            panic!("one hunk, not {stretches:?}");
        };
        assert_eq!(header.as_ref(), "@@ -1,3 +1,4 @@");
        let kinds: Vec<LineKind> = lines.iter().map(|line| line.kind).collect();
        use LineKind::*;
        assert_eq!(kinds, [Same, Removed, Added, Same, Added]);
        assert_eq!(&lines[1].text[lines[1].words[0].clone()], "1");
        assert_eq!((lines[2].old, lines[2].new), (None, Some(2)));
        assert_eq!((lines[4].old, lines[4].new), (None, Some(4)));
    }

    #[test]
    fn unchanged_lines_between_hunks_fold() {
        let old: String = (1..=20).map(|n| format!("{n}\n")).collect();
        let new = old
            .replacen("2\n", "two\n", 1)
            .replace("19\n", "nineteen\n");
        let stretches = diff(&old, &new, 1);
        let shape: Vec<&str> = stretches
            .iter()
            .map(|stretch| match stretch {
                Stretch::Hunk { .. } => "hunk",
                Stretch::Folded(_) => "folded",
            })
            .collect();
        assert_eq!(shape, ["hunk", "folded", "hunk"]);
        let Stretch::Folded(hidden) = &stretches[1] else {
            unreachable!()
        };
        assert_eq!((hidden.len(), hidden[0].old), (14, Some(4)));
        assert_eq!(stat(&old, &new), (2, 2));
    }

    #[test]
    fn removals_sit_beside_the_additions_after_them() {
        let stretches = diff("a\nb\nc\n", "a\nB\nC\nD\n", 0);
        let lines = stretches
            .iter()
            .find_map(|stretch| match stretch {
                Stretch::Hunk { lines, .. } => Some(lines),
                Stretch::Folded(_) => None,
            })
            .expect("a hunk");
        let sides: Vec<(bool, bool)> = pairs(lines)
            .iter()
            .map(|(left, right)| (left.is_some(), right.is_some()))
            .collect();
        assert_eq!(sides, [(true, true), (true, true), (false, true)]);
    }

    #[test]
    fn words_are_marked_on_lines_built_elsewhere() {
        let line = |kind, text: &str| DiffLine {
            kind,
            old: None,
            new: None,
            text: text.to_string().into(),
            words: Vec::new(),
        };
        let mut lines = vec![
            line(LineKind::Removed, "let x = 1;"),
            line(LineKind::Removed, "abc"),
            line(LineKind::Added, "let x = 2;"),
            line(LineKind::Added, "xyz"),
        ];
        mark_words(&mut lines);
        assert_eq!(&lines[0].text[lines[0].words[0].clone()], "1");
        assert_eq!(&lines[2].text[lines[2].words[0].clone()], "2");
        assert!(
            lines[1].words.is_empty() && lines[3].words.is_empty(),
            "nothing shared"
        );
    }

    #[test]
    fn each_side_numbers_its_own_file() {
        let stretches = diff("a\nb\n", "x\na\nb\n", 3);
        let [Stretch::Hunk { lines, .. }] = stretches.as_slice() else {
            panic!("one hunk, not {stretches:?}");
        };
        let numbers: Vec<_> = pairs(lines)
            .into_iter()
            .map(|(left, right)| side_numbers(left, right))
            .collect();
        assert_eq!(
            numbers,
            [(None, Some(1)), (Some(1), Some(2)), (Some(2), Some(3))]
        );
    }

    #[test]
    fn an_empty_side_starts_before_its_range() {
        let stretches = diff("", "a\n", 3);
        let [Stretch::Hunk { header, .. }] = stretches.as_slice() else {
            panic!("one hunk, not {stretches:?}");
        };
        assert_eq!(header.as_ref(), "@@ -0,0 +1 @@");
    }
}
