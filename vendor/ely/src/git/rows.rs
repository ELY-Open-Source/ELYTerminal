//! The rows a diff viewer draws, and their colors.

use std::ops::Range;

use gpui::{App, HighlightStyle, Hsla, SharedString, StyledText};

use super::{
    diff::{DiffLine, LineKind, Stretch, pair_places},
    viewer::DiffLayout,
};
use crate::{
    editor::{code_colors_with, stack},
    forms::Syntax,
    theme::Palette,
};

/// A line's place: its stretch, then its line there.
pub type Place = (usize, usize);

/// A row as the viewer draws it.
#[derive(Clone)]
pub(super) enum Shown {
    /// A hunk's header and its place among the stretches.
    Header(usize, SharedString),
    Line(Place, DiffLine),
    Pair(Option<(Place, DiffLine)>, Option<(Place, DiffLine)>),
    /// A folded stretch: its place and how many lines.
    Fold(usize, usize),
}

pub(super) fn shown(stretches: &[Stretch], layout: DiffLayout, open: &[usize]) -> Vec<Shown> {
    let mut rows = Vec::new();
    for (ix, stretch) in stretches.iter().enumerate() {
        match stretch {
            Stretch::Hunk { header, lines } => {
                rows.push(Shown::Header(ix, header.clone()));
                let at = |line: usize| ((ix, line), lines[line].clone());
                match layout {
                    DiffLayout::Unified => rows.extend((0..lines.len()).map(|line| {
                        let (place, line) = at(line);
                        Shown::Line(place, line)
                    })),
                    DiffLayout::Split => rows.extend(
                        pair_places(lines)
                            .into_iter()
                            .map(|(left, right)| Shown::Pair(left.map(at), right.map(at))),
                    ),
                }
            }
            Stretch::Folded(lines) if open.contains(&ix) => {
                rows.extend(lines.iter().enumerate().map(|(at, line)| {
                    let place = (ix, at);
                    match layout {
                        DiffLayout::Unified => Shown::Line(place, line.clone()),
                        DiffLayout::Split => {
                            Shown::Pair(Some((place, line.clone())), Some((place, line.clone())))
                        }
                    }
                }))
            }
            Stretch::Folded(lines) => rows.push(Shown::Fold(ix, lines.len())),
        }
    }
    rows
}

/// The wash behind a line, and behind its changed words.
pub(super) fn washes(kind: LineKind, colors: &Palette) -> (Option<Hsla>, Hsla) {
    match kind {
        LineKind::Same => (None, colors.hover),
        LineKind::Added => (
            Some(colors.success.opacity(0.08)),
            colors.success.opacity(0.25),
        ),
        LineKind::Removed => (
            Some(colors.danger.opacity(0.08)),
            colors.danger.opacity(0.25),
        ),
    }
}

/// A line's syntax colors; none for prose.
pub(super) fn colors_of(
    text: &str,
    syntax: Option<&Syntax>,
    cx: &App,
) -> Vec<(Range<usize>, HighlightStyle)> {
    syntax.map_or_else(Vec::new, |syntax| code_colors_with(text, syntax, cx))
}

/// A line colored as its file reads, changed words washed.
pub(super) fn code(
    line: &DiffLine,
    colors: &Palette,
    syntax: Option<&Syntax>,
    cx: &App,
) -> StyledText {
    let (_, word) = washes(line.kind, colors);
    let mut styles = colors_of(&line.text, syntax, cx);
    styles.extend(line.words.iter().map(|range| {
        (
            range.clone(),
            HighlightStyle {
                background_color: Some(word),
                ..HighlightStyle::default()
            },
        )
    }));
    styles.sort_by_key(|(range, _)| range.start);
    StyledText::new(line.text.clone()).with_highlights(stack(styles))
}

/// A line as assistive tech reads it: number, change, text.
pub(super) fn spoken(line: &DiffLine) -> SharedString {
    let (number, sign) = match line.kind {
        LineKind::Same => (line.new, ""),
        LineKind::Added => (line.new, "+ "),
        LineKind::Removed => (line.old, "− "),
    };
    let number = number.map_or_else(String::new, |n| format!("{n} "));
    format!("{number}{sign}{}", line.text).into()
}

#[cfg(test)]
mod tests {
    #[gpui::test]
    fn each_file_colors_as_its_kind_reads(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            crate::init_for_tests(cx);
            let syntax = crate::theme::ActiveTheme::theme(cx).colors.syntax.clone();
            let color = |text: &str, path: &str, cx: &gpui::App| {
                let syntax = crate::forms::syntax_of(path);
                super::colors_of(text, syntax, cx)
                    .into_iter()
                    .map(|(range, style)| (range, style.color))
                    .collect::<Vec<_>>()
            };
            for path in [
                "docs/README.md",
                "notes.MARKDOWN",
                "LICENSE",
                "a/b.txt",
                "data.csv",
            ] {
                assert!(
                    color("# return 42", path, cx).is_empty(),
                    "{path} reads plain"
                );
            }
            for path in [
                "script.py",
                "config.toml",
                "run.sh",
                ".gitignore",
                "Makefile",
                "CMakeLists.txt",
                "build.cmake",
            ] {
                assert_eq!(
                    color("# return 42", path, cx),
                    [(0..11, Some(syntax.comment))],
                    "{path}"
                );
            }
            assert_eq!(
                color("-- return 42", "q.sql", cx),
                [(0..12, Some(syntax.comment))]
            );
            let rust = color("return 42 // done", "src/main.rs", cx);
            assert_eq!(rust[0], (0..6, Some(syntax.keyword)), "{rust:?}");
            assert_eq!(rust.last(), Some(&(10..17, Some(syntax.comment))));
            // Each language's own words.
            let python = color("assert ready and True", "a.py", cx);
            assert_eq!(
                [python[0].1, python[2].1, python[3].1],
                [
                    Some(syntax.keyword),
                    Some(syntax.keyword),
                    Some(syntax.constant)
                ]
            );
            assert_ne!(
                color("return = 42", "config.toml", cx)[0].1,
                Some(syntax.keyword),
                "a TOML key"
            );
            // Words belong to their own language.
            assert_ne!(
                color("let from = 42;", "main.rs", cx)[1].1,
                Some(syntax.keyword),
                "Rust's from"
            );
            assert_eq!(
                color("def hello", "a.rb", cx)[0].1,
                Some(syntax.keyword),
                "Ruby's def"
            );
            assert_eq!(
                color("-- return 42", "Main.hs", cx),
                [(0..12, Some(syntax.comment))]
            );
            assert_eq!(
                color("% return 42", "main.erl", cx),
                [(0..11, Some(syntax.comment))]
            );
            // A comment may follow code at once outside Shell.
            assert_eq!(
                color("answer = 42# return 99", "config.toml", cx).last(),
                Some(&(11..22, Some(syntax.comment)))
            );
            assert_eq!(
                color("set(x 1)# return 42", "CMakeLists.txt", cx).last(),
                Some(&(8..19, Some(syntax.comment)))
            );
            // Shell comments start words only.
            let shell = color("echo foo#bar 42", "run.sh", cx);
            assert!(
                shell.iter().all(|(_, c)| *c != Some(syntax.comment)),
                "{shell:?}"
            );
            assert_eq!(
                color("echo x # note", "run.sh", cx).last(),
                Some(&(7..13, Some(syntax.comment)))
            );
        });
    }

    #[test]
    fn a_viewer_reads_its_file_once_built() {
        let viewer = |path: &str| {
            super::super::viewer::DiffViewer::new("d", path.to_string(), "a\n", "b\n").syntax
        };
        assert_eq!(viewer("notes.md"), None);
        assert_eq!(viewer("a.py"), crate::forms::syntax_of("a.py"));
        assert_eq!(
            viewer("CMakeLists.txt"),
            crate::forms::syntax_of("Makefile")
        );
    }

    #[test]
    fn a_line_speaks_its_number_change_and_text() {
        let stretches = super::super::diff::diff("a\nb\n", "a\nc\n", 1);
        let super::Stretch::Hunk { lines, .. } = &stretches[0] else {
            panic!("a hunk")
        };
        let said: Vec<String> = lines
            .iter()
            .map(|line| super::spoken(line).to_string())
            .collect();
        assert_eq!(said, ["1 a", "2 − b", "2 + c"]);
    }
}
