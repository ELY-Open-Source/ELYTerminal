use std::ops::Range;

use gpui::{
    App, Bounds, ClipboardItem, Context, EntityInputHandler, FontStyle, HighlightStyle, Pixels,
    Point, UTF16Selection, Window, point,
};

use super::{
    cursor::Selection,
    layout::{Row, Shown},
    measure::Measured,
    state::CodeEditor,
};
use crate::{
    forms::{from_utf16, to_utf16},
    theme::ActiveTheme,
};

impl CodeEditor {
    /// Notes set into a line: inlay hints, and the ghost text's first line.
    pub(crate) fn notes(&self, line: usize, cx: &App) -> Vec<(usize, String, HighlightStyle)> {
        let colors = &cx.theme().colors;
        let range = self.row_range(line);
        let hint = HighlightStyle {
            color: Some(colors.fg_subtle),
            background_color: Some(colors.hover),
            ..HighlightStyle::default()
        };
        let mut notes: Vec<(usize, String, HighlightStyle)> = self
            .marks
            .hints
            .iter()
            .filter(|hint| range.contains(&hint.offset) || hint.offset == range.end)
            .map(|note| (note.offset - range.start, note.text.to_string(), hint))
            .collect();
        if let Some(ghost) = &self.marks.ghost
            && self.buffer.line_of(ghost.offset) == line
        {
            let first = ghost.text.split('\n').next().unwrap_or_default();
            let style = HighlightStyle {
                color: Some(colors.fg_subtle),
                font_style: Some(FontStyle::Italic),
                ..HighlightStyle::default()
            };
            notes.push((ghost.offset - range.start, first.to_string(), style));
        }
        notes
    }

    /// A line's bytes as its row shows them: a multi-line ghost cuts the rest to follow its last line.
    pub(crate) fn row_range(&self, line: usize) -> Range<usize> {
        let range = self.buffer.line_range(line);
        match &self.marks.ghost {
            Some(ghost) if ghost.text.contains('\n') && range.contains(&ghost.offset) => {
                range.start..ghost.offset
            }
            _ => range,
        }
    }

    pub(crate) fn row_text(&self, line: usize) -> &str {
        &self.buffer.text()[self.row_range(line)]
    }

    /// Ends a composition: all of it, from its first mark, is one undo step.
    fn end_composition(&mut self) {
        if let Some(start) = self.composing.take()
            && start.text != self.buffer.text()
        {
            self.history.record(start, false);
            self.cap_history();
        }
    }

    /// The offset under a window point, when a row of code is there.
    pub(crate) fn offset_at(
        &self,
        position: Point<Pixels>,
        window: &Window,
        cx: &Context<Self>,
    ) -> Option<usize> {
        let metrics = self.metrics;
        if metrics.line <= Pixels::ZERO || metrics.advance <= Pixels::ZERO {
            return None;
        }
        let scrolled = -self.scroll.0.borrow().base_handle.offset().y;
        let row = ((position.y - metrics.top + scrolled) / metrics.line).floor();
        let row = self.frame.rows.get(row.max(0.0) as usize)?;
        let Row::Line(line) = *row else {
            return None;
        };
        self.offset_in(line, position.x, window, cx)
    }

    /// The offset nearest window x on a line; none for a line the text has since lost.
    pub(crate) fn offset_in(
        &self,
        line: usize,
        x: Pixels,
        window: &Window,
        cx: &Context<Self>,
    ) -> Option<usize> {
        if line >= self.buffer.lines() {
            return None;
        }
        let (shown, measured) = self.measured(line, window, cx);
        let column = measured.column(x - self.metrics.left);
        Some(
            self.buffer
                .offset(line, shown.back(self.row_text(line), column)),
        )
    }

    /// A line as drawn and laid out.
    fn measured(&self, line: usize, window: &Window, cx: &Context<Self>) -> (Shown, Measured) {
        let text = self.row_text(line);
        let shown = Shown::new(text, self.notes(line, cx), self.styles(line, cx));
        let family = cx.theme().mono_family.clone();
        let measured = Measured::new(
            &shown,
            family,
            self.metrics.size,
            self.metrics.advance,
            window,
        );
        (shown, measured)
    }

    fn utf16_range(&self, range: &Range<usize>) -> Range<usize> {
        let text = self.buffer.text();
        to_utf16(text, range.start)..to_utf16(text, range.end)
    }

    pub(crate) fn byte_range(&self, range: &Range<usize>) -> Range<usize> {
        let text = self.buffer.text();
        from_utf16(text, range.start)..from_utf16(text, range.end)
    }
}

impl EntityInputHandler for CodeEditor {
    fn paste(&mut self, item: ClipboardItem, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = item.text() {
            self.paste(&text, cx);
        }
    }

    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.byte_range(&range_utf16);
        actual_range.replace(self.utf16_range(&range));
        Some(self.buffer.text()[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let primary = self.primary();
        Some(UTF16Selection {
            range: self.utf16_range(&primary.range()),
            reversed: primary.head < primary.anchor,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked.as_ref().map(|range| self.utf16_range(range))
    }

    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.marked = None;
        self.end_composition();
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .map(|range| self.byte_range(&range))
            .or(self.marked.clone());
        match range {
            Some(range) => {
                self.marked = None;
                self.apply(vec![(range, text.to_string())], true, cx);
            }
            None => self.type_text(text, cx),
        }
        self.end_composition();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        selected_utf16: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.editable() {
            return;
        }
        let range = range_utf16
            .map(|range| self.byte_range(&range))
            .or(self.marked.clone())
            .unwrap_or(self.primary().range());
        if self.composing.is_none() {
            self.composing = Some(self.snapshot());
        }
        self.apply(vec![(range.clone(), text.to_string())], true, cx);
        self.marked = (!text.is_empty()).then(|| range.start..range.start + text.len());
        if self.marked.is_none() {
            self.end_composition();
        }
        let caret = match selected_utf16 {
            Some(inner) => range.start + from_utf16(text, inner.end),
            None => range.start + text.len(),
        };
        self.set_selections(vec![Selection::caret(caret)], cx);
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        _: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.byte_range(&range_utf16);
        let line = self.buffer.line_of(range.start);
        let row = self
            .frame
            .rows
            .iter()
            .position(|row| *row == Row::Line(line))?;
        let text = self.row_text(line);
        let (shown, measured) = self.measured(line, window, cx);
        let column = shown.column(text, self.buffer.point(range.start).1.min(text.len()));
        let metrics = self.metrics;
        let scrolled = -self.scroll.0.borrow().base_handle.offset().y;
        let origin = point(
            metrics.left + measured.x(column),
            metrics.top + metrics.line * row as f32 - scrolled,
        );
        let wide = measured.x(column + 1) - measured.x(column);
        Some(Bounds::new(origin, gpui::size(wide, metrics.line)))
    }

    fn character_index_for_point(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<usize> {
        let offset = self.offset_at(position, window, cx)?;
        Some(to_utf16(self.buffer.text(), offset))
    }
}
