use std::ops::Range;

use gpui::{
    AnyElement, Context, FontWeight, HighlightStyle, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, ParentElement, Stateful, Styled, StyledText, UnderlineStyle,
    Window, canvas, div, prelude::*,
};

use super::{
    cursor::Selection,
    gutter::Cell,
    layout::{Shown, stack},
    measure::Measured,
    state::{CodeEditor, CursorShape},
};
use crate::{
    charts::tint,
    theme::{ActiveTheme, Radius},
};

impl CodeEditor {
    /// A line's own styles: syntax colors, rainbow brackets, problems, washes, matches and the matched pair.
    pub(super) fn styles(
        &self,
        line: usize,
        cx: &Context<Self>,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        let colors = &cx.theme().colors;
        let range = self.row_range(line);
        let local = |span: &Range<usize>| {
            span.start.max(range.start) - range.start..span.end.min(range.end) - range.start
        };
        let touches = |span: &Range<usize>| {
            span.start < range.end.max(range.start + 1) && range.start < span.end
        };
        let mut spans = self.line_colors(self.row_text(line), cx);
        // Offsets found in an older text are left out.
        let fresh = self.frame.stamp == self.buffer.stamp();
        let brackets = self
            .frame
            .brackets
            .iter()
            .filter(|bracket| fresh && range.contains(&bracket.offset));
        for bracket in brackets {
            let at = bracket.offset - range.start;
            let color = if self.options.rainbow {
                tint(colors, bracket.depth % 3)
            } else {
                colors.syntax.punctuation
            };
            spans.retain(|(span, _)| *span != (at..at + 1));
            spans.push((at..at + 1, color.into()));
        }
        for span in self
            .frame
            .occurrences
            .iter()
            .filter(|span| fresh && touches(span))
        {
            spans.push((
                local(span),
                HighlightStyle {
                    background_color: Some(colors.selection.opacity(0.45)),
                    ..HighlightStyle::default()
                },
            ));
        }
        for (span, color) in self
            .marks
            .backgrounds
            .iter()
            .filter(|(span, _)| touches(span))
        {
            spans.push((
                local(span),
                HighlightStyle {
                    background_color: Some(*color),
                    ..HighlightStyle::default()
                },
            ));
        }
        for diagnostic in self
            .marks
            .diagnostics
            .iter()
            .filter(|diagnostic| touches(&diagnostic.range))
        {
            spans.push((
                local(&diagnostic.range),
                HighlightStyle {
                    underline: Some(UnderlineStyle {
                        thickness: cx.theme().underline_thickness(),
                        color: Some(diagnostic.severity.color(colors)),
                        wavy: true,
                    }),
                    ..HighlightStyle::default()
                },
            ));
        }
        if let Some((open, close)) = self.frame.matched.filter(|_| fresh) {
            for at in [open, close].into_iter().filter(|at| range.contains(at)) {
                let at = at - range.start;
                spans.push((
                    at..at + 1,
                    HighlightStyle {
                        background_color: Some(colors.active),
                        font_weight: Some(FontWeight::BOLD),
                        ..HighlightStyle::default()
                    },
                ));
            }
        }
        stack(spans)
    }

    /// The columns guides stand at on a line: every tab stop inside its indent, or the next line's when it is blank.
    fn guides(&self, line: usize) -> Vec<usize> {
        let indent = (line..self.buffer.lines())
            .find(|at| !self.buffer.line(*at).trim().is_empty())
            .map_or(0, |at| self.buffer.indent(at));
        (0..indent).step_by(self.options.tab).collect()
    }

    /// Dots for spaces, arrows for tabs; other characters hidden.
    fn whitespace(&self, line: usize, cx: &Context<Self>) -> StyledText {
        let text = self.row_text(line);
        let notes = self.notes(line, cx);
        let hidden = HighlightStyle {
            color: Some(gpui::transparent_black()),
            ..HighlightStyle::default()
        };
        let (mut out, mut hide) = (String::new(), Vec::new());
        let mut note = notes.iter().peekable();
        for (at, ch) in text.char_indices() {
            let start = out.len();
            while let Some((_, words, _)) = note.next_if(|(place, ..)| *place <= at) {
                out.push_str(words);
            }
            match ch {
                ' ' | '\t' => {
                    hide.push((start..out.len(), hidden));
                    out.push(if ch == ' ' { '·' } else { '→' });
                }
                _ => {
                    out.push(ch);
                    hide.push((start..out.len(), hidden));
                }
            }
        }
        hide.retain(|(range, _)| !range.is_empty());
        StyledText::new(out).with_highlights(hide)
    }

    pub(crate) fn line_row(
        &self,
        base: Stateful<gpui::Div>,
        row: usize,
        line: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let (advance, height) = (self.frame.advance, self.frame.line);
        let text = self.row_text(line);
        let range = self.row_range(line);
        let shown = Shown::new(text, self.notes(line, cx), self.styles(line, cx));
        let measured = Measured::new(
            &shown,
            theme.mono_family.clone(),
            self.metrics.size,
            advance,
            window,
        );
        // Text where laid out; rulers and guides on grid.
        let x = |column: usize| advance + measured.x(column);
        let grid = |column: usize| advance * (column + 1) as f32;
        let focused = self.focus.is_focused(window);
        let primary = self.primary();
        let current = primary.is_empty() && self.buffer.line_of(primary.head) == line;
        let column_of =
            |offset: usize| shown.column(text, self.buffer.point(offset).1.min(text.len()));
        let selections = self.selections.iter().filter_map(|selection| {
            let span = selection.range();
            if span.is_empty() || span.end < range.start || span.start > range.end {
                return None;
            }
            let start = if span.start <= range.start {
                0
            } else {
                column_of(span.start)
            };
            let past = span.end > range.end;
            let end = if past {
                column_of(range.end) + 1
            } else {
                column_of(span.end)
            };
            (end > start).then(|| {
                div()
                    .absolute()
                    .top_0()
                    .left(x(start))
                    .w(x(end) - x(start))
                    .h(height)
                    .bg(if focused {
                        colors.selection
                    } else {
                        colors.selection.opacity(0.5)
                    })
            })
        });
        let active = self
            .frame
            .folds
            .iter()
            .filter(|(header, end)| {
                let at = self.buffer.line_of(primary.head);
                *header < at && at <= *end
            })
            .map(|(header, _)| self.buffer.indent(*header))
            .next_back();
        let guides = if self.options.guides {
            self.guides(line)
        } else {
            Default::default()
        };
        let rulers = self.options.rulers.iter().map(|column| {
            div()
                .absolute()
                .top_0()
                .left(grid(*column))
                .h(height)
                .w_0()
                .border_l_1()
                .border_color(colors.border.opacity(0.6))
        });
        let guides = guides.into_iter().map(|column| {
            let strong = active == Some(column);
            div()
                .absolute()
                .top_0()
                .left(grid(column))
                .h(height)
                .w_0()
                .border_l_1()
                .border_color(if strong {
                    colors.border_strong
                } else {
                    colors.border.opacity(0.6)
                })
        });
        let carets = self
            .selections
            .iter()
            .filter(|selection| self.buffer.line_of(selection.head) == line)
            .filter(|_| focused && self.caret_on)
            .map(|selection| {
                let column = column_of(selection.head);
                let (left, wide) = (x(column), x(column + 1) - x(column));
                let caret = theme.caret_width().to_pixels(window.rem_size());
                let mark = div().absolute().left(left);
                match self.options.cursor {
                    CursorShape::Line => mark.top_0().w(caret).h(height).bg(colors.focus),
                    CursorShape::Block => mark
                        .top_0()
                        .w(wide)
                        .h(height)
                        .bg(colors.focus.opacity(0.35)),
                    CursorShape::Underline => mark
                        .top(height - caret * 1.5)
                        .w(wide)
                        .h(caret * 1.5)
                        .bg(colors.focus),
                }
            });
        let dots = self.options.whitespace.then(|| {
            div()
                .absolute()
                .top_0()
                .left(x(0))
                .h(height)
                .line_height(height)
                .whitespace_nowrap()
                .text_color(colors.fg_subtle.opacity(0.6))
                .child(self.whitespace(line, cx))
        });
        let folded = self.folded.contains(&line).then(|| {
            let entity = cx.entity();
            div()
                .id(("unfold", line))
                .absolute()
                .top_0()
                .left(x(shown.text.chars().count() + 1))
                .h(height)
                .flex()
                .items_center()
                .child(
                    div()
                        .px_1()
                        .rounded(theme.radius(Radius::Sm))
                        .bg(colors.hover)
                        .text_color(colors.fg_muted)
                        .line_height(height * 0.8)
                        .child("⋯"),
                )
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    window.prevent_default();
                    entity.update(cx, |editor, cx| editor.toggle_fold(line, cx));
                })
        });
        let added = self
            .marks
            .hunks
            .iter()
            .any(|hunk| (hunk.line..hunk.line + hunk.added).contains(&line));
        let entity = cx.entity();
        let left = x(0);
        let code = div()
            .id(("code", row))
            .relative()
            .flex_1()
            .h_full()
            .overflow_hidden()
            .when(current && focused, |code| code.bg(colors.hover))
            .when(added, |code| code.bg(colors.success.opacity(0.08)))
            .children(selections)
            .children(guides)
            .children(rulers)
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left(left)
                    .h(height)
                    .line_height(height)
                    .whitespace_nowrap()
                    .text_color(colors.syntax.variable)
                    .child(
                        StyledText::new(shown.text.clone()).with_highlights(shown.styles.clone()),
                    ),
            )
            .children(dots)
            .children(folded)
            .children(carets)
            .child(
                canvas(
                    move |bounds, _, cx| {
                        let origin = bounds.origin.x + left;
                        entity.update(cx, |editor, _| {
                            if editor.metrics.left != origin {
                                editor.metrics.left = origin;
                            }
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |editor, event: &MouseDownEvent, window, cx| {
                    editor.press(line, event, window, cx)
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |editor, event: &MouseDownEvent, window, cx| {
                    editor.secondary_press(line, event, window, cx)
                }),
            )
            .on_mouse_move(
                cx.listener(move |editor, event: &MouseMoveEvent, window, cx| {
                    editor.drag(line, event, window, cx)
                }),
            );
        base.child(self.gutter(Cell::Line(line), row, cx))
            .child(code)
            .into_any_element()
    }

    /// A press on a line: one caret, a word on a double press, the line on a triple; Shift extends, Alt adds a caret.
    pub(crate) fn press(
        &mut self,
        line: usize,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus, cx);
        let Some(at) = self.offset_in(line, event.position.x, window, cx) else {
            return;
        };
        let chosen = match event.click_count {
            2 => {
                let word = self.buffer.word_at(at);
                vec![Selection {
                    anchor: word.start,
                    head: word.end,
                    goal: None,
                }]
            }
            3.. => {
                let range = self.buffer.line_range(line);
                vec![Selection {
                    anchor: range.start,
                    head: self.buffer.next(range.end),
                    goal: None,
                }]
            }
            _ if event.modifiers.shift => {
                let mut all = self.selections.clone();
                let primary = all.last_mut().expect("an editor keeps a cursor");
                primary.head = at;
                primary.goal = None;
                all
            }
            _ if event.modifiers.alt => {
                let mut all = self.selections.clone();
                all.push(Selection::caret(at));
                all
            }
            _ => {
                self.dragging = true;
                vec![Selection::caret(at)]
            }
        };
        self.set_selections(chosen, cx);
    }

    /// A right press keeps a selection it lands in.
    pub(crate) fn secondary_press(
        &mut self,
        line: usize,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus, cx);
        let Some(at) = self.offset_in(line, event.position.x, window, cx) else {
            return;
        };
        let held = (self.selections.iter()).any(|selection| {
            let range = selection.range();
            !range.is_empty() && (range.start..=range.end).contains(&at)
        });
        if !held {
            self.set_selections(vec![Selection::caret(at)], cx);
        }
    }

    /// A drag over a line stretches the primary selection to where the pointer is.
    pub(crate) fn drag(
        &mut self,
        line: usize,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.dragging || !event.dragging() {
            return;
        }
        let Some(at) = self.offset_in(line, event.position.x, window, cx) else {
            return;
        };
        let mut all = self.selections.clone();
        let primary = all.last_mut().expect("an editor keeps a cursor");
        if primary.head == at {
            return;
        }
        primary.head = at;
        primary.goal = None;
        self.set_selections(all, cx);
    }
}
