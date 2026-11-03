use std::ops::Range;

use gpui::{
    Bounds, ClipboardItem, Context, EntityInputHandler, Pixels, Point, UTF16Selection, Window,
};

use super::{
    TextInput,
    edit::{from_utf16, to_utf16},
};

impl TextInput {
    fn range_from_utf16(&self, range: &Range<usize>) -> Range<usize> {
        from_utf16(&self.text, range.start)..from_utf16(&self.text, range.end)
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        to_utf16(&self.text, range.start)..to_utf16(&self.text, range.end)
    }
}

impl EntityInputHandler for TextInput {
    fn paste(&mut self, item: ClipboardItem, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = item.text() {
            self.replace(self.selection.clone(), &text, false, cx);
        }
    }

    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.text[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selection),
            reversed: self.reversed,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked.as_ref().map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let Some(marked) = self.marked.clone() else {
            return;
        };
        let composed = self.text[marked.clone()].to_string();
        self.replace(marked, &composed, true, cx);
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        log::debug!(
            "text input {:?}: replace {range_utf16:?} marked {:?} selection {:?} in {} bytes",
            cx.entity_id(),
            self.marked,
            self.selection,
            self.text.len()
        );
        let range = range_utf16
            .map(|range| self.range_from_utf16(&range))
            .or(self.marked.clone())
            .unwrap_or(self.selection.clone());
        self.replace(range, text, true, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        selected_utf16: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        log::debug!(
            "text input {:?}: compose {range_utf16:?} selecting {selected_utf16:?} marked {:?} selection {:?}",
            cx.entity_id(),
            self.marked,
            self.selection
        );
        let range = range_utf16
            .map(|range| self.range_from_utf16(&range))
            .or(self.marked.clone())
            .unwrap_or(self.selection.clone());
        if self.marked.is_none() {
            self.composing = Some(self.snapshot());
        }
        self.edits += 1;
        self.text.replace_range(range.clone(), text);
        self.marked = (!text.is_empty()).then(|| range.start..range.start + text.len());
        if self.marked.is_none() {
            self.composing = None;
        }
        self.selection = match selected_utf16 {
            Some(inner) => {
                let inner = from_utf16(text, inner.start)..from_utf16(text, inner.end);
                range.start + inner.start..range.start + inner.end
            }
            None => range.start + text.len()..range.start + text.len(),
        };
        self.reversed = false;
        self.restart_blink(cx);
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.range_from_utf16(&range_utf16);
        let (start, end) = (self.bounds_for(range.start)?, self.bounds_for(range.end)?);
        Some(Bounds::from_corners(start.origin, end.bottom_left()))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        self.layout.as_ref()?;
        Some(to_utf16(&self.text, self.offset_for_point(point)))
    }
}
