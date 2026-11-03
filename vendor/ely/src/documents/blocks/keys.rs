use gpui::{Context, Window};

use super::{BlockData, BlockEditor, BlockKind, table::shown_cell};
use crate::documents::modes::at_edge;

/// A new empty paragraph.
fn paragraph() -> BlockData {
    BlockData::new(BlockKind::Paragraph, [""])
}

impl BlockEditor {
    /// Where `key` sits, logged and none when the block has left.
    pub(super) fn found(&self, key: u64, what: &str) -> Option<usize> {
        let at = self.blocks.iter().position(|block| block.key == key);
        if at.is_none() {
            log::error!("block editor: no block {key} to {what}");
        }
        at
    }

    /// Where `key` sits now; a listener painted for a block since gone finds none.
    pub(crate) fn index(&self, key: u64) -> Option<usize> {
        self.found(key, "act on")
    }

    /// What `key` is now; a box painted for it acts only while it still is that.
    pub(crate) fn kind_of(&self, key: u64) -> Option<&BlockKind> {
        Some(&self.blocks[self.index(key)?].kind)
    }

    /// The block holding `key`'s field `field` now, when it still has that field.
    pub(crate) fn place(&self, key: u64, field: usize) -> Option<usize> {
        let ix = self.index(key)?;
        let held = field < self.blocks[ix].fields.len();
        if !held {
            log::error!("block editor: block {key} has no field {field} now");
        }
        held.then_some(ix)
    }

    /// The fields in reading order, as `(block, field)`: a closed toggle hides its body.
    pub(super) fn reading_order(&self) -> Vec<(u64, usize)> {
        self.blocks
            .iter()
            .flat_map(|block| {
                let shown = match block.kind {
                    BlockKind::Toggle(false) => 1,
                    _ => block.fields.len(),
                };
                let hidden: Vec<usize> = match &block.kind {
                    BlockKind::Table { columns, merged } => merged
                        .iter()
                        .map(|(row, column)| row * columns + column)
                        .collect(),
                    _ => Vec::new(),
                };
                (0..shown)
                    .filter(move |field| !hidden.contains(field))
                    .map(move |field| (block.key, field))
            })
            .collect()
    }

    /// Enter in a field: prose splits, a toggle's title opens its body, a caption starts text below, a cell steps down a row; elsewhere Enter breaks the line.
    pub(crate) fn on_enter(
        &mut self,
        key: u64,
        field: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(ix) = self.place(key, field) else {
            return true;
        };
        let kind = self.blocks[ix].kind.clone();
        match kind {
            kind if kind.is_prose() => self.split(key, window, cx),
            BlockKind::Toggle(open) if field == 0 => {
                if !open {
                    self.set_kind(key, BlockKind::Toggle(true), cx);
                }
                self.focus_field(key, 1, None, window, cx);
            }
            BlockKind::Image(_) | BlockKind::Video(_) | BlockKind::Embed(_) => {
                self.insert(Some(key), paragraph(), window, cx);
            }
            BlockKind::Table { columns, merged } => {
                let below = field + columns;
                if below >= self.blocks[ix].fields.len() {
                    self.add_row(key, window, cx);
                }
                let shown = shown_cell(&merged, columns, below);
                self.focus_field(key, shown, None, window, cx);
            }
            _ => return false,
        }
        true
    }

    /// Backspace at a field's start: prose joins up or turns back into text; an empty code, math or diagram block turns into text.
    pub(crate) fn on_backspace(
        &mut self,
        key: u64,
        field: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(ix) = self.place(key, field) else {
            return true;
        };
        let block = &self.blocks[ix];
        let input = block.fields[field].read(cx);
        if input.cursor() != 0 || !input.selection().is_empty() {
            return false;
        }
        let empty = input.is_empty();
        match &block.kind {
            kind if kind.is_prose() => self.join(key, window, cx),
            kind if kind.is_source() && empty => {
                self.turn_into(key, BlockKind::Paragraph, window, cx)
            }
            BlockKind::Toggle(_) if field == 0 && block.fields[1].read(cx).is_empty() => {
                self.turn_into(key, BlockKind::Paragraph, window, cx)
            }
            _ => return false,
        }
        true
    }

    /// Up or Down on a field's first or last line moves to the field above or below, a table cell by a row.
    pub(crate) fn on_vertical(
        &mut self,
        key: u64,
        field: usize,
        down: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(ix) = self.place(key, field) else {
            return true;
        };
        let block = &self.blocks[ix];
        if !at_edge(block.fields[field].read(cx), down) {
            return false;
        }
        if let BlockKind::Table { columns, merged } = &block.kind {
            let row = if down {
                Some(field + columns)
            } else {
                field.checked_sub(*columns)
            };
            if let Some(row) = row.filter(|row| *row < block.fields.len()) {
                let shown = shown_cell(merged, *columns, row);
                self.focus_field(key, shown, None, window, cx);
                return true;
            }
        }
        let order = self.reading_order();
        let Some(at) = order.iter().position(|place| *place == (key, field)) else {
            log::error!("block editor: field {field} of block {key} is not shown now");
            return true;
        };
        let target = if down {
            order[at + 1..].iter().find(|(other, _)| *other != key)
        } else {
            order[..at].iter().rev().find(|(other, _)| *other != key)
        };
        let Some(&(target, target_field)) = target else {
            if down && !self.blocks.last().is_some_and(|last| last.kind.is_prose()) {
                self.insert(Some(key), paragraph(), window, cx);
                return true;
            }
            return false;
        };
        let Some(found) = self.index(target) else {
            return true;
        };
        let end = self.blocks[found].fields[target_field]
            .read(cx)
            .text()
            .len();
        self.focus_field(
            target,
            target_field,
            Some(if down { 0 } else { end }),
            window,
            cx,
        );
        true
    }

    /// The submit key in any field leaves the block for a new paragraph below.
    pub(crate) fn on_submit(&mut self, key: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.insert(Some(key), paragraph(), window, cx);
    }
}
