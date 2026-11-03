mod apart;
mod attach;
mod block;
mod cells;
mod edit;
mod finding;
mod format;
mod lists;
mod look;
mod parse;
mod place;
mod point;
mod tables;
#[cfg(all(test, feature = "test-support"))]
mod tests;
mod visual;

pub(crate) use visual::bind_keys;

use std::{ops::Range, path::Path, rc::Rc, sync::Arc, time::Duration};

use gpui::{
    App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable, ListAlignment,
    ListState, Pixels, Subscription, Task, Window,
};

pub use crate::documents::tree::DocumentHeading;
pub use attach::Attachment;
pub use block::BlockCommand;
pub use format::{EditRefusal, InlineFormat};
pub use look::DocumentLook;
use parse::{Parsed, alike, parsed};
pub use point::{DocumentTarget, TargetBlock};
pub use tables::{TableEdit, TableRefusal};

use super::{
    MarkdownMode,
    markdown::markdown_highlights,
    modes::joint,
    render::{CellAt, OnHeading, OnLink},
};
use crate::{
    editor::{CodeEditor, EditorEvent, LineNumbers},
    forms::TextInput,
};

/// Texts this long parse at once; longer ones elsewhere.
const AT_ONCE: usize = 128 << 10;

/// Longer texts are never read: source alone.
pub const LARGEST_READ: usize = 2 << 20;

/// Typing waits this long for a pause before a read.
const PAUSE: Duration = Duration::from_millis(300);

/// What the document tells its owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocumentEvent {
    /// The text changed, typed in any mode or replaced.
    Changed,
    /// A new read of the text: blocks and headings.
    Parsed,
    /// A secondary press moved the caret onto this.
    Pointed(DocumentTarget),
    /// A long format, accepted, did not happen.
    EditRefused(EditRefusal),
}

/// A top line asked, numbered so late callbacks know theirs.
#[derive(Clone, Copy, Debug)]
struct Placement {
    line: usize,
    ask: u64,
    /// Scrolled; its frame callback finishes it.
    placing: bool,
}

/// The block open as its source, in its own field.
struct Open {
    range: Range<usize>,
    input: Entity<TextInput>,
    /// Opened past the end, to add a block.
    append: bool,
    /// A table cell alone; its bare pipes escape.
    cell: Option<CellAt>,
    /// Watches an overlay that borrowed focus.
    lent: Option<Subscription>,
    _subscriptions: Vec<Subscription>,
}

/// A Markdown file: one text, one history, three views.
pub struct MarkdownDocument {
    editor: Entity<CodeEditor>,
    mode: MarkdownMode,
    base: Option<Arc<Path>>,
    parsed: Arc<Parsed>,
    /// Text changes counted; a read of an older one lapses.
    edits: u64,
    _parsing: Option<Task<()>>,
    editing: bool,
    list: ListState,
    open: Option<Open>,
    /// A dropped block's focus, for the document to take.
    orphan: Option<FocusHandle>,
    /// A top line asked for, until the view holds it.
    top: Option<Placement>,
    asks: u64,
    /// Its own write-backs whose changes are still to come.
    writes: usize,
    /// A match asked before its text was read; its version.
    showing: Option<(Range<usize>, u64)>,
    /// A block asked open before its text was read.
    waiting: Option<(Option<usize>, edit::Landing, u64)>,
    look: DocumentLook,
    /// Typing in the open block, counted.
    typed: u64,
    /// Typewriter room past each end of the list.
    room: Pixels,
    /// The block, text, typing and heights last centered.
    centered: Option<(usize, u64, u64, [u64; 3])>,
    focus: FocusHandle,
    /// A secondary press took focus.
    pointing: bool,
    /// The latest press; older lookups lapse.
    pressed: u64,
    on_link: Option<OnLink>,
    on_heading: Option<OnHeading>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<DocumentEvent> for MarkdownDocument {}

impl Focusable for MarkdownDocument {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        match self.mode {
            MarkdownMode::Source | MarkdownMode::Split => self.editor.focus_handle(cx),
            MarkdownMode::Visual | MarkdownMode::Preview => self.focus.clone(),
        }
    }
}

impl MarkdownDocument {
    pub fn new(
        text: impl Into<String>,
        mode: MarkdownMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let editor = cx.new(|cx| {
            CodeEditor::new(text, window, cx)
                .language("Markdown")
                .line_numbers(LineNumbers::Hidden)
                .hide_indent_guides()
                .highlighter(markdown_highlights)
        });
        let focus = cx.focus_handle();
        let subscriptions = vec![
            cx.subscribe(&editor, |document, _, event, cx| {
                if *event == EditorEvent::Changed {
                    document.changed(cx);
                }
            }),
            cx.on_focus(&focus, window, Self::focused),
        ];
        let overdraw = window.rem_size() * 40.0;
        let this = cx.entity().downgrade();
        let on_heading: OnHeading = Rc::new(move |at, window, cx| {
            this.update(cx, |document, cx| document.reveal_at(at, window, cx))
                .expect("a heading list lives in its document");
        });
        let mut document = Self {
            editor,
            mode,
            base: None,
            parsed: Arc::default(),
            edits: 0,
            _parsing: None,
            editing: false,
            list: ListState::new(1, ListAlignment::Top, overdraw),
            open: None,
            orphan: None,
            top: None,
            asks: 0,
            writes: 0,
            showing: None,
            waiting: None,
            look: DocumentLook::default(),
            typed: 0,
            centered: None,
            room: Pixels::ZERO,
            focus,
            pointing: false,
            pressed: 0,
            on_link: None,
            on_heading: Some(on_heading),
            _subscriptions: subscriptions,
        };
        document.reparse(true, cx);
        document
    }

    /// The folder relative pictures load from.
    pub fn set_base(&mut self, folder: impl Into<Arc<Path>>, cx: &mut Context<Self>) {
        self.base = Some(folder.into());
        cx.notify();
    }

    /// Takes link presses; without it the system opens them.
    pub fn on_link(&mut self, handler: impl Fn(&str, &mut Window, &mut App) + 'static) {
        self.on_link = Some(Rc::new(handler));
    }

    pub fn editor(&self) -> &Entity<CodeEditor> {
        &self.editor
    }

    pub fn mode(&self) -> MarkdownMode {
        self.mode
    }

    /// Shows `mode` at the same line; an open block writes.
    pub fn set_mode(&mut self, mode: MarkdownMode, window: &mut Window, cx: &mut Context<Self>) {
        self.commit(cx);
        log::info!("markdown document: {:?} to {mode:?}", self.mode);
        if !self.read_now(cx) {
            self.reparse(true, cx);
        }
        let listed = |mode| matches!(mode, MarkdownMode::Visual | MarkdownMode::Preview);
        // Visual and preview share one list and its scroll.
        let top = (!(listed(self.mode) && listed(mode))).then(|| self.top_line(cx));
        self.mode = mode;
        if let Some(top) = top {
            self.set_top_line(top, window, cx);
        }
        let focus = self.focus_handle(cx);
        window.focus(&focus, cx);
        // Preview to visual keeps one handle: no focus event.
        self.focused(window, cx);
        cx.notify();
    }

    /// The text now, an open block's typing included.
    pub fn text(&self, cx: &App) -> String {
        let text = self.editor.read(cx).text();
        let Some(open) = &self.open else {
            return text.to_string();
        };
        let typed = open.input.read(cx).text();
        let written = match open.append {
            true if typed.is_empty() => String::new(),
            true => format!("{}{typed}", joint(text)),
            false if open.cell.is_some() => {
                cells::kept(typed, cells::edges(text, open.range.clone()))
            }
            false => typed.to_string(),
        };
        [&text[..open.range.start], &written, &text[open.range.end..]].concat()
    }

    /// Replaces the text in one step; open typing drops.
    pub fn set_text(&mut self, text: impl Into<String>, cx: &mut Context<Self>) {
        self.drop_open(cx);
        self.editor
            .update(cx, |editor, cx| editor.set_text(text, cx));
    }

    /// New text, undo afresh; caret and top line stay.
    pub fn reset(&mut self, text: impl Into<String>, cx: &mut Context<Self>) {
        let ((line, column), top) = (self.caret(cx), self.top_line(cx));
        self.drop_open(cx);
        self.editor.update(cx, |editor, cx| editor.reset(text, cx));
        self.set_caret(line, column, cx);
        // Placed once the new text is read.
        self.asks += 1;
        self.top = Some(Placement {
            line: top,
            ask: self.asks,
            placing: false,
        });
        cx.notify();
    }

    /// The headings as last read, with lines from zero.
    pub fn outline(&self, cx: &App) -> Vec<(DocumentHeading, usize)> {
        let editor = self.editor.read(cx);
        self.parsed
            .headings
            .iter()
            .filter(|heading| heading.at <= editor.text().len())
            .map(|heading| (heading.clone(), editor.buffer.line_of(heading.at)))
            .collect()
    }

    /// Whether the blocks shown read the text now.
    pub fn read_now(&self, cx: &App) -> bool {
        self.parsed.text == self.editor.read(cx).text()
    }

    fn changed(&mut self, cx: &mut Context<Self>) {
        // Only the document writes while a block is open.
        if self.writes > 0 {
            self.writes -= 1;
        } else if self.open.is_some() {
            log::error!("markdown document: the text changed under the open block");
            self.drop_open(cx);
        }
        self.replace_top();
        self.edits += 1;
        cx.emit(DocumentEvent::Changed);
        // Typed source reads after a pause.
        let typed = matches!(self.mode, MarkdownMode::Source | MarkdownMode::Split);
        self.reparse(!typed, cx);
    }

    /// Reads the text: short ones now, else after a pause.
    fn reparse(&mut self, now: bool, cx: &mut Context<Self>) {
        let (text, edits) = (self.editor.read(cx).text().to_string(), self.edits);
        if text.len() > LARGEST_READ {
            self._parsing = None;
            log::info!("markdown document: {} bytes, too long to read", text.len());
            // Nothing read: no blocks, no headings.
            let unread = Parsed {
                text,
                ..Parsed::default()
            };
            return self.adopt(unread, edits, cx);
        }
        if now && text.len() <= AT_ONCE {
            self._parsing = None;
            return self.adopt(parsed(&text), edits, cx);
        }
        self._parsing = Some(cx.spawn(async move |document, cx| {
            if !now {
                cx.background_executor().timer(PAUSE).await;
            }
            let read = cx.background_spawn(async move { parsed(&text) }).await;
            document
                .update(cx, |document, cx| document.adopt(read, edits, cx))
                .ok();
        }));
    }

    /// Takes the read of edit `edits`; older reads drop.
    fn adopt(&mut self, read: Parsed, edits: u64, cx: &mut Context<Self>) {
        if edits != self.edits {
            return log::debug!("markdown document: read {edits} lapsed");
        }
        let (head, tail) = alike(&self.parsed, &read);
        let old = self.parsed.blocks.len();
        self.list
            .splice(head..old - tail, read.blocks.len() - head - tail);
        let headings: Vec<usize> = read.headings.iter().map(|heading| heading.at).collect();
        self.editor
            .update(cx, |editor, _| editor.set_anchors(headings));
        self.parsed = Arc::new(read);
        cx.emit(DocumentEvent::Parsed);
        cx.notify();
    }

    /// Closes the open block without writing it.
    fn drop_open(&mut self, cx: &mut Context<Self>) {
        if let Some(open) = self.open.take() {
            self.orphan = Some(open.input.focus_handle(cx));
            log::info!("markdown document: the open block closed unwritten");
            cx.notify();
        }
    }
}
