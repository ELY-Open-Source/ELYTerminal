use std::collections::HashMap;

use gpui::{App, Context, Focusable, Window};

use super::{Block, BlockData, BlockEditor, BlockEvent, BlockKind, Snapshot};

impl BlockEditor {
    /// Splits a prose block at the caret: the text after it moves to a new block of the kind that follows.
    pub(crate) fn split(&mut self, key: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.index(key) else {
            return;
        };
        let kind = self.blocks[ix].kind.clone();
        let field = self.blocks[ix].fields[0].clone();
        if field.read(cx).is_empty()
            && kind != BlockKind::Paragraph
            && kind.next() != BlockKind::Paragraph
        {
            self.turn_into(key, BlockKind::Paragraph, window, cx);
            return;
        }
        let selection = field.read(cx).selection();
        let tail = field.read(cx).text()[selection.end..].to_string();
        log::info!("block editor: block {key} splits over {selection:?}");
        self.before_change(cx);
        field.update(cx, |field, cx| {
            let end = field.text().len();
            field.select(selection.start..end, cx);
            field.insert("", cx);
        });
        let key = self.fresh_key();
        let next = self.build(
            BlockData {
                kind: kind.next(),
                texts: vec![tail],
            },
            key,
            window,
            cx,
        );
        self.blocks.insert(ix + 1, next);
        self.after_change(cx);
        self.focus_field(key, 0, Some(0), window, cx);
    }

    /// Backspace at a prose block's start: another kind turns back into text; text joins the prose above, or an empty block goes.
    pub(crate) fn join(&mut self, key: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.index(key) else {
            return;
        };
        if self.blocks[ix].kind != BlockKind::Paragraph {
            self.turn_into(key, BlockKind::Paragraph, window, cx);
            return;
        }
        let text = self.blocks[ix].fields[0].read(cx).text().to_string();
        let Some(above) = ix.checked_sub(1).map(|ix| &self.blocks[ix]) else {
            return;
        };
        if !above.kind.is_prose() {
            if text.is_empty() {
                self.remove(key, window, cx);
            }
            return;
        }
        let (above_key, above_field) = (above.key, above.fields[0].clone());
        log::info!("block editor: block {key} joins block {above_key}");
        self.before_change(cx);
        let joint = above_field.read(cx).text().len();
        above_field.update(cx, |field, cx| {
            field.select(joint..joint, cx);
            field.insert(&text, cx);
        });
        self.blocks.remove(ix);
        self.after_change(cx);
        self.focus_field(above_key, 0, Some(joint), window, cx);
    }

    pub(crate) fn undo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.clear_span(window, cx);
        self.settle(true, cx);
        if let Some(previous) = self.history.undo(self.now(cx)) {
            log::info!("block editor: undo");
            self.restore(previous, window, cx);
        }
    }

    pub(crate) fn redo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.clear_span(window, cx);
        if let Some(next) = self.history.redo(self.now(cx)) {
            log::info!("block editor: redo");
            self.restore(next, window, cx);
        }
    }

    /// The current document with the caret where it is now.
    fn now(&self, cx: &Context<Self>) -> Snapshot {
        let mut now = self.current.clone();
        now.caret = self.snapshot(cx).caret;
        now
    }

    /// Puts the document back as `snapshot` holds it, keeping fields whose blocks stay, and the caret where it was.
    fn restore(&mut self, snapshot: Snapshot, window: &mut Window, cx: &mut Context<Self>) {
        let mut kept: HashMap<u64, Block> = self
            .blocks
            .drain(..)
            .map(|block| (block.key, block))
            .collect();
        for (key, data) in &snapshot.blocks {
            let block = match kept.remove(key) {
                Some(mut block)
                    if block.fields.len() == data.texts.len()
                        && block.kind.same_fields(&data.kind) =>
                {
                    block.kind = data.kind.clone();
                    for (field, text) in block.fields.iter().zip(&data.texts) {
                        if field.read(cx).text() != text {
                            field.update(cx, |field, cx| field.set_text(text.clone(), cx));
                        }
                    }
                    block
                }
                _ => self.build(data.clone(), *key, window, cx),
            };
            self.blocks.push(block);
        }
        if let Some((key, field, selection)) = snapshot.caret.clone()
            && let Some(block) = self.blocks.iter().find(|block| block.key == key)
            && let Some(field) = block.fields.get(field)
        {
            field.update(cx, |field, cx| field.select(selection, cx));
            window.focus(&field.focus_handle(cx), cx);
        }
        self.current = snapshot;
        cx.emit(BlockEvent::Changed);
        cx.notify();
    }

    pub(super) fn snapshot(&self, cx: &App) -> Snapshot {
        let caret = self.caret.and_then(|(key, field)| {
            let block = self.blocks.iter().find(|block| block.key == key)?;
            let selection = block.fields.get(field)?.read(cx).selection();
            Some((key, field, selection))
        });
        Snapshot {
            caret,
            blocks: self
                .blocks
                .iter()
                .map(|block| {
                    let texts = block
                        .fields
                        .iter()
                        .map(|field| field.read(cx).text().to_string())
                        .collect();
                    (
                        block.key,
                        BlockData {
                            kind: block.kind.clone(),
                            texts,
                        },
                    )
                })
                .collect(),
        }
    }

    /// Records what changed as one undo step, a burst of typing as one, and tells the owner.
    pub(super) fn settle(&mut self, typing: bool, cx: &mut Context<Self>) {
        let now = self.snapshot(cx);
        let typed_from = self.typed_from.take();
        if now.blocks != self.current.blocks {
            let mut before = std::mem::replace(&mut self.current, now);
            if typed_from.is_some() {
                before.caret = typed_from;
            }
            self.history.record(before, typing);
            cx.emit(BlockEvent::Changed);
            cx.notify();
        }
    }

    /// Records the document before a change to its blocks.
    pub(crate) fn before_change(&mut self, cx: &mut Context<Self>) {
        self.settle(true, cx);
        let mut before = self.current.clone();
        before.caret = self.snapshot(cx).caret;
        self.history.record(before, false);
    }

    /// After a change to the blocks: the new state is current.
    pub(crate) fn after_change(&mut self, cx: &mut Context<Self>) {
        self.current = self.snapshot(cx);
        cx.emit(BlockEvent::Changed);
        cx.notify();
    }
}
