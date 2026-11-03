use gpui::{
    AnyElement, Context, Hsla, InteractiveElement, IntoElement, MouseButton, ParentElement,
    SharedString, Styled, div, prelude::*,
};

use super::{
    decor::GitMark,
    state::{CodeEditor, LineNumbers},
};
use crate::{
    primitives::{Disclosure, Icon, Severity},
    theme::{ActiveTheme, IconSize},
};

/// What a gutter cell shows beside a row.
pub(crate) enum Cell {
    Line(usize),
    /// A removed line of a diff.
    Removed,
    Blank,
}

impl CodeEditor {
    /// Columns the gutter spans: a slot for a breakpoint or a problem, the number, a fold chevron and the git bar.
    pub(crate) fn gutter_columns(&self) -> usize {
        match self.options.numbers {
            LineNumbers::Hidden => 5,
            _ => self.digits() + 6,
        }
    }

    /// Columns the numbers take: none when hidden.
    fn digits(&self) -> usize {
        match self.options.numbers {
            LineNumbers::Hidden => 0,
            _ => self.buffer.lines().to_string().len().max(2),
        }
    }

    fn number(&self, line: usize) -> SharedString {
        let current = self.buffer.line_of(self.primary().head);
        match self.options.numbers {
            LineNumbers::Hidden => SharedString::default(),
            LineNumbers::Relative if line != current => line.abs_diff(current).to_string().into(),
            _ => (line + 1).to_string().into(),
        }
    }

    /// The worst problem on a line, if any.
    fn worst(&self, line: usize) -> Option<Severity> {
        let range = self.buffer.line_range(line);
        self.marks
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.range.start <= range.end && range.start <= diagnostic.range.end
            })
            .map(|diagnostic| diagnostic.severity)
            .max_by_key(|severity| match severity {
                Severity::Danger => 3,
                Severity::Warning => 2,
                Severity::Info => 1,
                Severity::Success => 0,
            })
    }

    fn git_color(&self, line: usize, cx: &Context<Self>) -> Option<Hsla> {
        let colors = &cx.theme().colors;
        let added = self
            .marks
            .hunks
            .iter()
            .any(|hunk| (hunk.line..hunk.line + hunk.added).contains(&line));
        if added {
            return Some(colors.success);
        }
        self.marks
            .git
            .iter()
            .find(|(at, _)| *at == line)
            .map(|(_, mark)| match mark {
                GitMark::Added => colors.success,
                GitMark::Modified => colors.info,
                GitMark::Deleted => colors.danger,
            })
    }

    /// The gutter cell for a row.
    pub(crate) fn gutter(&self, cell: Cell, row: usize, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let advance = self.frame.advance;
        let digits = self.digits();
        let frame = || {
            div()
                .flex_none()
                .h_full()
                .flex()
                .items_center()
                .justify_center()
        };
        let base = div()
            .id(("gutter", row))
            .flex_none()
            .w(advance * self.gutter_columns() as f32)
            .h_full()
            .flex()
            .items_center()
            .text_color(colors.fg_subtle);
        let line = match cell {
            Cell::Line(line) => line,
            Cell::Removed => {
                return base
                    .bg(colors.danger.opacity(0.08))
                    .child(frame().w(advance * 2.0))
                    .child(
                        div()
                            .w(advance * digits.max(1) as f32)
                            .text_right()
                            .text_color(colors.danger)
                            .child("−"),
                    )
                    .into_any_element();
            }
            Cell::Blank => return base.into_any_element(),
        };
        let current = self.buffer.line_of(self.primary().head) == line;
        let breakpoint = self.marks.breakpoints.contains(&line);
        let slot = match (breakpoint, self.worst(line)) {
            (true, _) => div()
                .size(advance * 1.1)
                .rounded_full()
                .bg(colors.danger)
                .into_any_element(),
            (false, Some(severity)) => Icon::new(severity.icon())
                .size(IconSize::Xs)
                .color(severity.color(&colors))
                .into_any_element(),
            (false, None) => div().into_any_element(),
        };
        let opens = self.frame.folds.iter().any(|(header, _)| *header == line);
        let folded = self.folded.contains(&line);
        let chevron = opens.then(|| Disclosure::new(("fold", line), !folded).into_any_element());
        let entity = cx.entity();
        let toggle = entity.clone();
        let git = self.git_color(line, cx);
        let number = self.number(line);
        base.child(
            frame()
                .id(("breakpoint", line))
                .w(advance * 2.0)
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    window.prevent_default();
                    entity.update(cx, |editor, cx| editor.toggle_breakpoint(line, cx));
                })
                .child(slot),
        )
        .when(digits > 0, |gutter| {
            gutter.child(
                div()
                    .w(advance * digits as f32)
                    .text_right()
                    // Snapped to device pixels, the box can run short.
                    .whitespace_nowrap()
                    .when(current, |number| number.text_color(colors.fg))
                    .child(number),
            )
        })
        .child(
            frame()
                .id(("chevron", line))
                .w(advance * 2.0)
                .when(opens, |slot| {
                    slot.cursor_pointer()
                        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                            window.prevent_default();
                            toggle.update(cx, |editor, cx| editor.toggle_fold(line, cx));
                        })
                })
                .children(chevron),
        )
        .child(
            div()
                .flex_none()
                .w_0p5()
                .h_full()
                .when_some(git, |bar, color| bar.bg(color)),
        )
        .into_any_element()
    }
}
