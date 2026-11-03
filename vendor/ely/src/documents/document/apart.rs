//! Edits read off the UI thread: formats and block commands.

use std::ops::Range;

use gpui::{AppContext as _, Context};

use super::{
    DocumentEvent, LARGEST_READ, MarkdownDocument,
    block::{self, BlockCommand, settled},
    cells,
    format::{Change, EditRefusal, InlineFormat, drop_image, toggle},
    tables::{self, TableEdit},
};
use crate::documents::{MarkdownMode, modes::joint};

/// The text read, the open block's offset, the selection.
type Snapshot = (String, usize, Range<usize>);

impl MarkdownDocument {
    /// `format` at the selection, read apart; refusals by event.
    pub fn format(
        &mut self,
        format: InlineFormat,
        cx: &mut Context<Self>,
    ) -> Result<(), EditRefusal> {
        self.apart(
            format!("format {format:?}"),
            move |full, selection| toggle(full, selection, format),
            cx,
        )
    }

    /// Removes the picture at source byte `at`, read apart.
    pub fn delete_image(&mut self, at: usize, cx: &mut Context<Self>) -> Result<(), EditRefusal> {
        let drop = move |full: &str, _| drop_image(full, at);
        self.apart("delete image".to_string(), drop, cx)
    }

    /// `command` at the caret, read apart; refusals by event.
    pub fn block(
        &mut self,
        command: BlockCommand,
        cx: &mut Context<Self>,
    ) -> Result<(), EditRefusal> {
        self.apart(
            format!("block {command:?}"),
            move |full, selection| block::run(full, selection, command),
            cx,
        )
    }

    /// `edit` on the table at the caret, read apart.
    pub fn edit_table(
        &mut self,
        edit: TableEdit,
        cx: &mut Context<Self>,
    ) -> Result<(), EditRefusal> {
        self.apart(
            format!("table edit {edit:?}"),
            move |full, selection| match edit {
                TableEdit::Delete => tables::delete(full, selection.start),
                _ => tables::edit_table(full, selection.start, edit).map_err(EditRefusal::Cells),
            },
            cx,
        )
    }

    /// Reads `read` off the UI thread, then writes its change.
    fn apart(
        &mut self,
        name: String,
        read: impl FnOnce(&str, Range<usize>) -> Result<Change, EditRefusal> + Send + 'static,
        cx: &mut Context<Self>,
    ) -> Result<(), EditRefusal> {
        if self.mode == MarkdownMode::Preview {
            return Err(EditRefusal::Preview);
        }
        let (full, offset, selection) = self.snapshot(cx);
        if full.len() > LARGEST_READ {
            log::info!("markdown document: {name} refused: {} bytes", full.len());
            return Err(EditRefusal::TooLong);
        }
        if self.editing {
            log::info!("markdown document: {name} refused: one runs");
            return Err(EditRefusal::Busy);
        }
        log::info!("markdown document: {name} reads {} bytes apart", full.len());
        let taken = (full.clone(), offset, selection.clone());
        self.editing = true;
        cx.spawn(async move |document, cx| {
            let change = cx
                .background_spawn(async move { read(&full, selection) })
                .await;
            document
                .update(cx, |document, cx| {
                    document.editing = false;
                    let done = if document.mode == MarkdownMode::Preview {
                        Err(EditRefusal::Preview)
                    } else if document.snapshot(cx) != taken {
                        Err(EditRefusal::Changed)
                    } else {
                        change
                            .and_then(|change| document.apply(&name, change, &taken.0, taken.1, cx))
                    };
                    if let Err(why) = done {
                        log::info!("markdown document: {name} refused: {why:?}");
                        cx.emit(DocumentEvent::EditRefused(why));
                    }
                })
                .ok();
        })
        .detach();
        Ok(())
    }

    /// The text an edit reads: the open block's in place.
    pub(super) fn snapshot(&self, cx: &Context<Self>) -> Snapshot {
        let text = self.editor.read(cx).text();
        let Some(open) = &self.open else {
            let picked = self.editor.read(cx).primary().range();
            return settle((text.to_string(), 0, picked));
        };
        let input = open.input.read(cx);
        let (before, after) = match open.append {
            true => (format!("{text}{}", joint(text)), ""),
            false => (
                text[..open.range.start].to_string(),
                &text[open.range.end..],
            ),
        };
        let (typed, mut picked) = (input.text(), input.selection());
        // A cell reads as it writes back.
        let typed = match open.cell {
            Some(_) => {
                let edges = cells::edges(text, open.range.clone());
                let shift = |at: usize| cells::kept_at(typed, at, edges);
                picked = shift(picked.start)..shift(picked.end);
                cells::kept(typed, edges)
            }
            None => typed.to_string(),
        };
        let offset = before.len();
        let full = format!("{before}{typed}{after}");
        settle((full, offset, offset + picked.start..offset + picked.end))
    }

    /// Writes `change`: into the open block while it stays inside.
    fn apply(
        &mut self,
        name: &str,
        (edits, selected): Change,
        full: &str,
        offset: usize,
        cx: &mut Context<Self>,
    ) -> Result<(), EditRefusal> {
        log::info!("markdown document: {name}");
        if let Some(open) = &self.open {
            let typed = open.input.read(cx).text();
            // Offsets read the escaped cell; its field holds it raw.
            let edges = cells::edges(self.editor.read(cx).text(), open.range.clone());
            let raw = open.cell.is_some() && cells::kept(typed, edges) != typed;
            let end = offset + typed.len();
            let grown = (edits.iter().map(|(_, with)| with.len()).sum::<usize>() + end)
                .checked_sub(edits.iter().map(|(range, _)| range.len()).sum::<usize>());
            let inside = !raw
                && edits
                    .iter()
                    .all(|(range, _)| offset <= range.start && range.end <= end)
                && grown.is_some_and(|grown| offset <= selected.start && selected.end <= grown);
            if inside {
                let (Some(from), Some(to)) = (
                    edits.iter().map(|(range, _)| range.start).min(),
                    edits.iter().map(|(range, _)| range.end).max(),
                ) else {
                    return Ok(());
                };
                let mut written = String::new();
                let mut at = from;
                for (range, with) in &edits {
                    written.push_str(&full[at..range.start]);
                    written.push_str(with);
                    at = range.end;
                }
                written.push_str(&full[at..to]);
                open.input.update(cx, |input, cx| {
                    input.select(from - offset..to - offset, cx);
                    input.insert(&written, cx);
                    input.select(selected.start - offset..selected.end - offset, cx);
                });
                return Ok(());
            }
            log::info!("markdown document: {name} reaches past the open block");
            self.commit(cx);
            if self.editor.read(cx).text() != full {
                log::error!("markdown document: {name}: the write-back is not the text read");
                return Err(EditRefusal::Changed);
            }
        }
        self.editor.update(cx, |editor, cx| {
            editor.edit(edits, cx);
            editor.select(std::iter::once(selected), cx);
        });
        Ok(())
    }
}

/// The editor splits lines on LF only; reads start off a CRLF.
fn settle((full, offset, picked): Snapshot) -> Snapshot {
    let picked = settled(&full, picked.start)..settled(&full, picked.end);
    (full, offset, picked)
}
