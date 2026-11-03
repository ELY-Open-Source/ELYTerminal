//! Matches shown in any mode; replacements in one step.

use std::ops::Range;

use gpui::{Context, Window};

use super::{
    MarkdownDocument,
    cells::{self, Aim},
    edit::Landing,
};
use crate::documents::MarkdownMode;

impl MarkdownDocument {
    /// `range` in view: selected, or in its opened block.
    pub fn show_match(&mut self, range: Range<usize>, window: &mut Window, cx: &mut Context<Self>) {
        let line = |document: &Self, cx: &Context<Self>| {
            document.editor.read(cx).text()[..range.start]
                .matches('\n')
                .count()
        };
        // A newer request takes the place of one waiting.
        self.showing = None;
        match self.mode {
            MarkdownMode::Source | MarkdownMode::Split => {
                self.editor
                    .update(cx, |editor, cx| editor.select(std::iter::once(range), cx));
            }
            MarkdownMode::Preview => self.reveal(line(self, cx), window, cx),
            MarkdownMode::Visual => {
                if self.commit(cx).is_some() || !self.read_now(cx) {
                    self.reparse(true, cx);
                }
                if !self.read_now(cx) {
                    // Shown once read, if nothing else changed it.
                    let expected = self.edits + self.writes as u64;
                    self.showing = Some((range, expected));
                    return cx.notify();
                }
                let ix = self.block_at(range.start);
                let held = self.parsed.blocks.get(ix).map(|block| block.range.clone());
                let Some(block) =
                    held.filter(|block| block.start <= range.start && range.end <= block.end)
                else {
                    // Outside one block: its line.
                    return self.reveal(line(self, cx), window, cx);
                };
                // A match in a table opens its cell.
                let text = self.editor.read(cx).text();
                let landing = match cells::find(text, block.clone(), Aim::Byte(range.start)) {
                    Some(cell)
                        if cell.range.start <= range.start && range.end <= cell.range.end =>
                    {
                        Landing::Cell(Aim::At(cell.at))
                    }
                    Some(_) => return self.reveal(line(self, cx), window, cx),
                    None => Landing::End,
                };
                self.open(Some(ix), landing, window, cx);
                self.list.scroll_to_reveal_item(ix);
                if let Some(open) = &self.open {
                    let input = open.input.clone();
                    let inside = range.start - open.range.start..range.end - open.range.start;
                    input.update(cx, |input, cx| input.select(inside, cx));
                }
            }
        }
    }

    /// A match asked before its text was read, now shown.
    pub(super) fn show_waiting(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.showing.is_none() || !self.read_now(cx) {
            return;
        }
        let (range, expected) = self.showing.take().expect("checked above");
        if self.edits != expected {
            return log::info!("markdown document: a waiting match lapsed with its text");
        }
        self.show_match(range, window, cx);
    }

    /// `edits` as one undo step; false in preview.
    pub fn replace(&mut self, edits: Vec<(Range<usize>, String)>, cx: &mut Context<Self>) -> bool {
        if self.mode == MarkdownMode::Preview {
            return false;
        }
        self.commit(cx);
        let text = self.editor.read(cx).text();
        if edits
            .iter()
            .all(|(range, with)| text[range.clone()] == *with)
        {
            return true;
        }
        log::info!("markdown document: {} replaced", edits.len());
        // Its own change: an open block outlives it.
        self.writes += 1;
        self.editor.update(cx, |editor, cx| editor.edit(edits, cx));
        true
    }
}
