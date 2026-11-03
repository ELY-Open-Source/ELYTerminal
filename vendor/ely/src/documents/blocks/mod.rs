mod content;
mod edit;
mod export;
mod import;
mod keys;
mod kind;
mod media;
mod span;
mod table;
#[cfg(all(test, feature = "test-support"))]
mod tests;
mod view;

use std::{collections::HashMap, rc::Rc};

use gpui::{
    App, AppContext as _, Context, Entity, EntityId, EventEmitter, Focusable, Global, ScrollHandle,
    SharedString, Subscription, Window,
};

pub use kind::{Align, BlockKind, Media};
pub(crate) use kind::{matching, shortcut};
pub(crate) use media::source;

use super::markdown::markdown_highlights;
use crate::forms::{History, InputEvent, TextInput, code_highlights};

/// A block as the editor gives it out and takes it in: its kind and the text of each of its fields.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockData {
    pub kind: BlockKind,
    pub texts: Vec<String>,
}

impl BlockData {
    pub fn new(kind: BlockKind, texts: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let texts: Vec<String> = texts.into_iter().map(Into::into).collect();
        assert!(
            kind.accepts(texts.len()),
            "a {} block cannot hold {} fields",
            kind.label(),
            texts.len()
        );
        Self { kind, texts }
    }
}

/// What a block editor tells its owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockEvent {
    Changed,
}

/// Renders a diagram's source to SVG, or says what is wrong with it.
pub(crate) type RenderDiagram = Rc<dyn Fn(&str) -> Result<String, String>>;

/// Text shared by synced blocks, by name, across every editor in the app.
#[derive(Default)]
struct SyncedTexts(HashMap<SharedString, String>);

impl Global for SyncedTexts {}

pub(crate) struct Block {
    pub key: u64,
    pub kind: BlockKind,
    pub fields: Vec<Entity<TextInput>>,
    _changes: Vec<Subscription>,
}

/// The whole document as undo keeps it, and where the caret was: block, field and selection.
#[derive(Clone, Debug)]
struct Snapshot {
    blocks: Vec<(u64, BlockData)>,
    caret: Option<(u64, usize, std::ops::Range<usize>)>,
}

/// A document of blocks, Notion-like, that fills its box and scrolls, keeping the caret in view: prose, lists and to-dos, quotes and callouts, toggles, code, math, diagrams, dividers, tables, media, columns and synced blocks. Markdown at a paragraph's start changes its kind; `/` offers every kind; `@` and `:` suggest people and emoji; blocks drag by their handle. Undo covers text and structure alike.
pub struct BlockEditor {
    pub(crate) blocks: Vec<Block>,
    next: u64,
    history: History<Snapshot>,
    current: Snapshot,
    /// The field that last held focus, as block and field.
    caret: Option<(u64, usize)>,
    /// The selection the last edit in a field replaced.
    typed_from: Option<(u64, usize, std::ops::Range<usize>)>,
    pub(crate) people: Vec<SharedString>,
    pub(crate) diagram: Option<RenderDiagram>,
    scroll: ScrollHandle,
    across: span::Across,
    /// The caret last brought into view: field, selection and length.
    revealed: Option<(EntityId, std::ops::Range<usize>, usize)>,
    _synced: Subscription,
}

impl EventEmitter<BlockEvent> for BlockEditor {}

impl BlockEditor {
    pub fn new(blocks: Vec<BlockData>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let synced = cx.observe_global_in::<SyncedTexts>(window, |editor, _, cx| editor.pull(cx));
        let mut editor = Self {
            blocks: Vec::new(),
            next: 0,
            history: History::default(),
            current: Snapshot {
                blocks: Vec::new(),
                caret: None,
            },
            caret: None,
            typed_from: None,
            people: Vec::new(),
            diagram: None,
            scroll: ScrollHandle::new(),
            across: span::Across::default(),
            revealed: None,
            _synced: synced,
        };
        for data in blocks {
            let key = editor.fresh_key();
            let block = editor.build(data, key, window, cx);
            editor.blocks.push(block);
        }
        editor.current = editor.snapshot(cx);
        editor
    }

    /// Handles `@` suggests.
    pub fn people(mut self, handles: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.people = handles.into_iter().map(Into::into).collect();
        self
    }

    /// Draws diagram blocks: their source to SVG, or an error to show in its place.
    pub fn diagrams(mut self, render: impl Fn(&str) -> Result<String, String> + 'static) -> Self {
        self.diagram = Some(Rc::new(render));
        self
    }

    /// The document as it stands.
    pub fn blocks(&self, cx: &App) -> Vec<BlockData> {
        self.snapshot(cx)
            .blocks
            .into_iter()
            .map(|(_, data)| data)
            .collect()
    }

    /// A field for `kind`: inline markdown styled as typed, or code colored as code.
    fn field(
        kind: &BlockKind,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<TextInput> {
        let code = *kind == BlockKind::Code;
        let field = cx.new(|cx| {
            let hint = match kind {
                BlockKind::Paragraph | BlockKind::Table { .. } => "",
                kind => kind.placeholder(),
            };
            let field = TextInput::new(window, cx)
                .multi_line(1, usize::MAX)
                .placeholder(hint);
            match (code, kind.is_source()) {
                (true, _) => field.highlighter(code_highlights),
                (false, true) => field,
                (false, false) => field.highlighter(markdown_highlights),
            }
        });
        field.update(cx, |field, cx| field.set_text(text, cx));
        field
    }

    fn fresh_key(&mut self) -> u64 {
        self.next += 1;
        self.next
    }

    fn build(
        &mut self,
        data: BlockData,
        key: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Block {
        assert!(
            data.kind.accepts(data.texts.len()),
            "a {} block cannot hold {} fields",
            data.kind.label(),
            data.texts.len()
        );
        let mut texts = data.texts;
        if let BlockKind::Synced(name) = &data.kind {
            let shared = cx
                .try_global::<SyncedTexts>()
                .and_then(|synced| synced.0.get(name).cloned());
            match shared {
                Some(shared) => texts[0] = shared,
                None => {
                    cx.default_global::<SyncedTexts>()
                        .0
                        .insert(name.clone(), texts[0].clone());
                }
            }
        }
        let fields: Vec<Entity<TextInput>> = texts
            .iter()
            .map(|text| Self::field(&data.kind, text, window, cx))
            .collect();
        let changes = fields
            .iter()
            .map(|field| Self::watch(key, field, window, cx))
            .collect();
        Block {
            key,
            kind: data.kind,
            fields,
            _changes: changes,
        }
    }

    fn watch(
        key: u64,
        field: &Entity<TextInput>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Subscription {
        cx.subscribe_in(field, window, move |editor, field, event, window, cx| {
            editor.on_field(key, field, *event, window, cx)
        })
    }

    /// Adds an empty field to a block, at `at` or last.
    pub(crate) fn add_field(
        &mut self,
        key: u64,
        at: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ix) = self.index(key) else {
            return;
        };
        let field = Self::field(&self.blocks[ix].kind, "", window, cx);
        let watched = Self::watch(key, &field, window, cx);
        let block = &mut self.blocks[ix];
        let at = at.unwrap_or(block.fields.len());
        block.fields.insert(at, field);
        block._changes.insert(at, watched);
    }

    pub(crate) fn remove_field(&mut self, key: u64, field: usize) {
        let Some(ix) = self.index(key) else {
            return;
        };
        let block = &mut self.blocks[ix];
        block.fields.remove(field);
        drop(block._changes.remove(field));
    }

    fn on_field(
        &mut self,
        key: u64,
        field: &Entity<TextInput>,
        event: InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ix) = self.index(key) else {
            return;
        };
        match event {
            InputEvent::Changed => {
                let at = self.blocks[ix].fields.iter().position(|each| each == field);
                if at.is_some_and(|at| self.span_takes(key, at, window, cx)) {
                    return;
                }
                let typed = field.focus_handle(cx).is_focused(window);
                if typed
                    && self.blocks[ix].kind == BlockKind::Paragraph
                    && let Some((kind, length)) = shortcut(field.read(cx).text())
                    && field.read(cx).cursor() == length
                {
                    log::info!(
                        "block editor: markdown turns block {key} into {}",
                        kind.label()
                    );
                    field.update(cx, |field, cx| {
                        field.select(0..length, cx);
                        field.insert("", cx);
                    });
                    self.turn_into(key, kind, window, cx);
                    return;
                }
                if let BlockKind::Synced(name) = &self.blocks[ix].kind {
                    let text = field.read(cx).text().to_string();
                    let shared = cx
                        .try_global::<SyncedTexts>()
                        .and_then(|synced| synced.0.get(name));
                    if shared != Some(&text) {
                        log::info!("block editor: synced {name} written");
                        cx.default_global::<SyncedTexts>()
                            .0
                            .insert(name.clone(), text);
                    }
                }
                let at = self.blocks[ix].fields.iter().position(|each| each == field);
                self.typed_from = at.map(|at| (key, at, field.read(cx).edited_from()));
                self.settle(true, cx);
            }
            InputEvent::Focus => {
                self.caret = self.blocks[ix]
                    .fields
                    .iter()
                    .position(|each| each == field)
                    .map(|at| (key, at));
                if field.read(cx).is_empty() {
                    let hint = self.blocks[ix].kind.placeholder();
                    field.update(cx, |field, cx| field.set_placeholder(hint, cx));
                }
            }
            InputEvent::Blur if self.blocks[ix].kind == BlockKind::Paragraph => {
                field.update(cx, |field, cx| field.set_placeholder("", cx));
            }
            _ => {}
        }
    }

    /// Takes in synced text written elsewhere.
    fn pull(&mut self, cx: &mut Context<Self>) {
        let shared: Vec<(Entity<TextInput>, String)> = self
            .blocks
            .iter()
            .filter_map(|block| match &block.kind {
                BlockKind::Synced(name) => cx
                    .try_global::<SyncedTexts>()
                    .and_then(|synced| synced.0.get(name))
                    .map(|text| (block.fields[0].clone(), text.clone())),
                _ => None,
            })
            .collect();
        for (field, text) in shared {
            if field.read(cx).text() != text {
                log::debug!("block editor: synced text arrived");
                field.update(cx, |field, cx| field.set_text(text, cx));
            }
        }
    }

    pub(crate) fn focus_field(
        &self,
        key: u64,
        field: usize,
        at: Option<usize>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(ix) = self.index(key) else {
            return;
        };
        let Some(field) = self.blocks[ix].fields.get(field) else {
            return;
        };
        if let Some(at) = at {
            field.update(cx, |field, cx| field.select(at..at, cx));
        }
        window.focus(&field.focus_handle(cx), cx);
    }

    /// Adds a block after `after`, or first, and puts the caret in it.
    pub fn insert(
        &mut self,
        after: Option<u64>,
        data: BlockData,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> u64 {
        let at = match after {
            None => 0,
            Some(after) => self
                .found(after, "insert after")
                .map_or(self.blocks.len(), |ix| ix + 1),
        };
        self.before_change(cx);
        let key = self.fresh_key();
        let block = self.build(data, key, window, cx);
        log::info!("block editor: {} block at {at}", block.kind.label());
        let end = block
            .fields
            .first()
            .map(|field| field.read(cx).text().len());
        self.blocks.insert(at, block);
        self.after_change(cx);
        self.focus_field(key, 0, end, window, cx);
        key
    }

    /// Changes a block's kind, keeping its first field's text; fields the new kind needs start empty.
    pub fn turn_into(
        &mut self,
        key: u64,
        kind: BlockKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ix) = self.found(key, "turn") else {
            return;
        };
        self.before_change(cx);
        let first = self.blocks[ix]
            .fields
            .first()
            .map(|field| field.read(cx).text().to_string())
            .unwrap_or_default();
        let mut texts = vec![String::new(); kind.fields()];
        if let Some(text) = texts.first_mut() {
            *text = first;
        }
        log::info!("block editor: block {key} turns into {}", kind.label());
        let block = self.build(BlockData { kind, texts }, key, window, cx);
        self.blocks[ix] = block;
        self.after_change(cx);
        self.focus_field(key, 0, None, window, cx);
    }

    /// Changes a block's kind in place, as a to-do's box or a toggle's state does.
    pub fn set_kind(&mut self, key: u64, kind: BlockKind, cx: &mut Context<Self>) {
        let Some(ix) = self.found(key, "set the kind of") else {
            return;
        };
        assert!(
            kind.accepts(self.blocks[ix].fields.len()),
            "set_kind keeps the fields; turn_into changes them"
        );
        self.before_change(cx);
        self.blocks[ix].kind = kind;
        self.after_change(cx);
    }

    pub fn remove(&mut self, key: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.found(key, "remove") else {
            return;
        };
        self.before_change(cx);
        log::info!("block editor: block {key} removed");
        self.blocks.remove(ix);
        self.after_change(cx);
        if let Some(previous) = ix.checked_sub(1).and_then(|ix| self.blocks.get(ix)) {
            let key = previous.key;
            let end = previous
                .fields
                .first()
                .map(|field| field.read(cx).text().len());
            self.focus_field(key, 0, end, window, cx);
        }
    }

    pub fn duplicate(&mut self, key: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.found(key, "duplicate") else {
            return;
        };
        let data = self.snapshot(cx).blocks[ix].1.clone();
        self.insert(Some(key), data, window, cx);
    }

    /// Moves the block at `from` to place `to`.
    pub fn move_block(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        if from >= self.blocks.len() {
            log::error!(
                "block editor: no block at {from} of {} to move",
                self.blocks.len()
            );
            return;
        }
        self.before_change(cx);
        log::info!("block editor: block moved from {from} to {to}");
        let block = self.blocks.remove(from);
        self.blocks.insert(to.min(self.blocks.len()), block);
        self.after_change(cx);
    }

    /// Whether `key`'s field `field` holds focus.
    pub(crate) fn focused(&self, window: &Window, cx: &App) -> Option<(u64, usize)> {
        self.blocks.iter().find_map(|block| {
            block
                .fields
                .iter()
                .position(|field| field.focus_handle(cx).is_focused(window))
                .map(|ix| (block.key, ix))
        })
    }
}
