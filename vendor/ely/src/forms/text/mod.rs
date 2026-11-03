mod actions;
mod edit;
mod element;
mod geometry;
mod highlight;
mod ime;

use std::{ops::Range, rc::Rc, time::Duration};

use gpui::{
    App, Bounds, Context, EventEmitter, FocusHandle, Focusable, Pixels, Point, SharedString,
    Subscription, Task, Window, WrappedLine,
};

pub(crate) use actions::{
    Backspace, Delete, DeleteWordLeft, Down, Enter, Left, Redo, Right, Submit, Undo, Up, bind_keys,
};
pub use actions::{Copy, Cut, Paste, SelectAll};
pub use edit::History;
use edit::Snapshot;
pub(crate) use edit::{from_utf16, to_utf16};
pub use highlight::Highlight;

use crate::theme::ActiveTheme;

/// What a text input tells its owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputEvent {
    Changed,
    /// Enter in one line, or the submit key in several.
    Submit,
    Focus,
    Blur,
}

type Highlighter = Rc<dyn Fn(&str, &App) -> Vec<(Range<usize>, Highlight)>>;
type Filter = Rc<dyn Fn(char) -> bool>;
pub(crate) type Fit = Rc<dyn Fn(&str, Range<usize>, &str) -> (String, usize)>;

/// The last shaped text: one entry per hard line, with its display offset.
pub(crate) struct Layout {
    pub lines: Rc<[(usize, WrappedLine)]>,
    pub bounds: Bounds<Pixels>,
    pub line_height: Pixels,
    /// The lines show the placeholder, not content.
    pub placeholder: bool,
}

const BULLET: char = '•';
const BLINK: Duration = Duration::from_millis(530);

/// Editable text: one line or many, with selection, undo, IME and clipboard.
pub struct TextInput {
    focus: FocusHandle,
    text: String,
    placeholder: SharedString,
    label: SharedString,
    selection: Range<usize>,
    reversed: bool,
    marked: Option<Range<usize>>,
    composing: Option<Snapshot>,
    rows: Option<(usize, usize)>,
    masked: bool,
    filter: Option<Filter>,
    fit: Option<Fit>,
    max_len: Option<usize>,
    highlighter: Option<Highlighter>,
    disabled: bool,
    history: History<Snapshot>,
    /// The selection the last edit replaced.
    edited_from: Range<usize>,
    /// Edits applied, text changed or not.
    edits: u64,
    pub(crate) layout: Option<Layout>,
    pub(crate) scroll: Point<Pixels>,
    selecting: bool,
    caret_on: bool,
    /// Blink runs only while focused, or unseen carets redraw forever.
    focused: bool,
    blink_epoch: u64,
    _blink: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<InputEvent> for TextInput {}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl TextInput {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle().tab_stop(true);
        let subscriptions = vec![
            cx.on_focus(&focus, window, |input, _, cx| {
                input.focused = true;
                input.restart_blink(cx);
                cx.emit(InputEvent::Focus);
            }),
            cx.on_blur(&focus, window, |input, _, cx| {
                input.focused = false;
                input.selecting = false;
                input._blink = None;
                cx.emit(InputEvent::Blur);
                cx.notify();
            }),
        ];
        Self {
            focus,
            text: String::new(),
            placeholder: SharedString::default(),
            label: SharedString::default(),
            selection: 0..0,
            reversed: false,
            marked: None,
            composing: None,
            rows: None,
            masked: false,
            filter: None,
            fit: None,
            max_len: None,
            highlighter: None,
            disabled: false,
            history: History::default(),
            edited_from: 0..0,
            edits: 0,
            layout: None,
            scroll: Point::default(),
            selecting: false,
            caret_on: true,
            focused: false,
            blink_epoch: 0,
            _blink: None,
            _subscriptions: subscriptions,
        }
    }

    /// Several lines: grows from `min` rows to `max`, then scrolls.
    pub fn multi_line(mut self, min: usize, max: usize) -> Self {
        assert!(0 < min && min <= max, "text rows {min}..={max} are empty");
        self.rows = Some((min, max));
        self
    }

    pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// The name assistive technology reads; the placeholder otherwise.
    pub fn label(mut self, text: impl Into<SharedString>) -> Self {
        self.label = text.into();
        self
    }

    /// Shows bullets instead of the text, and keeps it off the clipboard.
    pub fn masked(mut self) -> Self {
        self.masked = true;
        self
    }

    /// Drops typed or pasted characters `allow` rejects.
    pub fn filter(mut self, allow: impl Fn(char) -> bool + 'static) -> Self {
        self.filter = Some(Rc::new(allow));
        self
    }

    /// Longest text, in characters.
    pub fn max_len(mut self, len: usize) -> Self {
        self.max_len = Some(len);
        self
    }

    /// Colors spans of the text as it changes.
    pub fn highlighter(
        mut self,
        highlight: impl Fn(&str, &App) -> Vec<(Range<usize>, Highlight)> + 'static,
    ) -> Self {
        self.highlighter = Some(Rc::new(highlight));
        self
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// The text as committed: while an input method composes, the text before it began.
    pub(crate) fn committed(&self) -> &str {
        self.composing
            .as_ref()
            .map_or(&self.text, |start| &start.text)
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn is_masked(&self) -> bool {
        self.masked
    }

    pub fn is_multi_line(&self) -> bool {
        self.rows.is_some()
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub fn rows(&self) -> Option<(usize, usize)> {
        self.rows
    }

    pub fn placeholder_text(&self) -> &SharedString {
        &self.placeholder
    }

    pub fn label_text(&self) -> &SharedString {
        &self.label
    }

    pub fn selection(&self) -> Range<usize> {
        self.selection.clone()
    }

    pub fn cursor(&self) -> usize {
        if self.reversed {
            self.selection.start
        } else {
            self.selection.end
        }
    }

    pub fn set_placeholder(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.placeholder = text.into();
        cx.notify();
    }

    pub fn set_label(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.label = text.into();
        cx.notify();
    }

    pub fn set_masked(&mut self, masked: bool, cx: &mut Context<Self>) {
        self.masked = masked;
        cx.notify();
    }

    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        self.disabled = disabled;
        self.focus = self.focus.clone().tab_stop(!disabled);
        cx.notify();
    }

    /// Replaces all text and puts the caret at its end. Not an undo step.
    pub fn set_text(&mut self, text: impl Into<String>, cx: &mut Context<Self>) {
        let text = self.admit(&text.into(), 0);
        let text = match &self.fit {
            Some(fit) => {
                let base = self.committed();
                fit(base, 0..base.len(), &text).0
            }
            None => text,
        };
        self.selection = text.len()..text.len();
        self.text = text;
        self.marked = None;
        self.composing = None;
        cx.emit(InputEvent::Changed);
        cx.notify();
    }

    /// Reshapes every edit, as a mask does: text, replaced range and typed text in; text and caret out. Undo starts afresh, since earlier text need not fit.
    pub(crate) fn set_fit(&mut self, fit: Fit) {
        self.fit = Some(fit);
        self.history = History::default();
    }

    /// Replaces the selection, as typing would. An undo step.
    pub fn insert(&mut self, text: &str, cx: &mut Context<Self>) {
        self.replace(self.selection.clone(), text, false, cx);
    }

    /// Selects `range`; one outside the text or inside a character is logged and left.
    pub fn select(&mut self, range: Range<usize>, cx: &mut Context<Self>) {
        let fits = range.start <= range.end
            && self.text.is_char_boundary(range.start)
            && self.text.is_char_boundary(range.end);
        if !fits {
            log::error!(
                "text input: no selection {range:?} in {} bytes",
                self.text.len()
            );
            return;
        }
        self.selection = range;
        self.reversed = false;
        self.restart_blink(cx);
    }

    /// Keeps what `filter` and `max_len` allow of `incoming`.
    fn admit(&self, incoming: &str, keep: usize) -> String {
        let allowed: String = incoming
            .chars()
            .filter(|ch| self.filter.as_ref().is_none_or(|allow| allow(*ch)))
            .collect();
        match self.max_len {
            Some(max) => allowed.chars().take(max.saturating_sub(keep)).collect(),
            None => allowed,
        }
    }

    /// The selection the last edit replaced, for an owner's undo to go back to.
    pub(crate) fn edited_from(&self) -> Range<usize> {
        self.edited_from.clone()
    }

    /// How many edits it took, and whether an input method is composing.
    pub(crate) fn edits(&self) -> (u64, bool) {
        (self.edits, self.marked.is_some())
    }

    pub(crate) fn replace(
        &mut self,
        range: Range<usize>,
        incoming: &str,
        typing: bool,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        self.edited_from = self.selection.clone();
        let incoming = if self.rows.is_none() {
            incoming.replace(['\n', '\r'], " ")
        } else {
            incoming.replace("\r\n", "\n")
        };
        let kept = self.text.chars().count() - self.text[range.clone()].chars().count();
        let incoming = self.admit(&incoming, kept);
        if incoming.is_empty() && range.is_empty() {
            return;
        }
        self.edits += 1;
        let composed = self.composing.take();
        let before = composed.clone().unwrap_or_else(|| self.snapshot());
        let caret = match &self.fit {
            Some(fit) => {
                let (base, range) = match &composed {
                    Some(start) => (start.text.as_str(), start.selection.clone()),
                    None => (self.text.as_str(), range),
                };
                let (text, caret) = fit(base, range, &incoming);
                self.text = text;
                caret
            }
            None => {
                self.text.replace_range(range.clone(), &incoming);
                range.start + incoming.len()
            }
        };
        self.selection = caret..caret;
        self.reversed = false;
        self.marked = None;
        self.commit(before, typing, cx);
        self.restart_blink(cx);
    }

    /// Makes `before` one undo step, if the text moved on from it.
    fn commit(&mut self, before: Snapshot, typing: bool, cx: &mut Context<Self>) {
        if before.text != self.text {
            self.history.record(before, typing);
            cx.emit(InputEvent::Changed);
        }
    }

    /// The state as committed, so undo never keeps an input method's text before it commits.
    fn snapshot(&self) -> Snapshot {
        self.composing.clone().unwrap_or_else(|| Snapshot {
            text: self.text.clone(),
            selection: self.selection.clone(),
        })
    }

    pub(crate) fn restore(&mut self, snapshot: Snapshot, cx: &mut Context<Self>) {
        self.text = snapshot.text;
        self.selection = snapshot.selection;
        self.reversed = false;
        self.marked = None;
        self.composing = None;
        cx.emit(InputEvent::Changed);
        self.restart_blink(cx);
    }

    pub(crate) fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selection = offset..offset;
        self.reversed = false;
        self.restart_blink(cx);
    }

    pub(crate) fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.reversed {
            self.selection.start = offset;
        } else {
            self.selection.end = offset;
        }
        if self.selection.end < self.selection.start {
            self.reversed = !self.reversed;
            self.selection = self.selection.end..self.selection.start;
        }
        self.restart_blink(cx);
    }

    /// Shows the caret now and restarts its blink.
    fn restart_blink(&mut self, cx: &mut Context<Self>) {
        self.caret_on = true;
        self.blink_epoch += 1;
        let epoch = self.blink_epoch;
        let still = cx.theme().reduced_motion;
        self._blink = (!still && self.focused).then(|| {
            cx.spawn(async move |input, cx| {
                loop {
                    cx.background_executor().timer(BLINK).await;
                    let alive = input.update(cx, |input, cx| {
                        if input.blink_epoch == epoch {
                            input.caret_on = !input.caret_on;
                            cx.notify();
                        }
                    });
                    if alive.is_err() {
                        return;
                    }
                }
            })
        });
        cx.notify();
    }

    /// What the element shows: bullets when masked.
    pub(crate) fn display_text(&self) -> String {
        if self.masked {
            BULLET.to_string().repeat(self.grapheme_count())
        } else {
            self.text.clone()
        }
    }

    fn grapheme_count(&self) -> usize {
        unicode_segmentation::UnicodeSegmentation::graphemes(self.text.as_str(), true).count()
    }

    pub(crate) fn display_offset(&self, offset: usize) -> usize {
        if self.masked {
            edit::masked_offset(&self.text, offset, BULLET)
        } else {
            offset
        }
    }

    pub(crate) fn content_offset(&self, display: usize) -> usize {
        if self.masked {
            edit::unmasked_offset(&self.text, display, BULLET)
        } else {
            display
        }
    }

    pub(crate) fn highlights(&self, cx: &App) -> Vec<(Range<usize>, Highlight)> {
        match (&self.highlighter, self.masked) {
            (Some(highlight), false) => highlight(&self.text, cx),
            _ => Vec::new(),
        }
    }

    pub(crate) fn marked(&self) -> Option<Range<usize>> {
        self.marked.clone()
    }

    pub(crate) fn selecting(&self) -> bool {
        self.selecting
    }

    pub(crate) fn caret_on(&self) -> bool {
        self.caret_on
    }

    pub(crate) fn focus(&self) -> &FocusHandle {
        &self.focus
    }
}
