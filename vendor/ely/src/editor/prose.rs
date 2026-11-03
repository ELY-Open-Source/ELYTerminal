use std::{fmt, ops::Range, rc::Rc};

use gpui::{App, Context, FontStyle, HighlightStyle, StrikethroughStyle};

use super::{buffer::Buffer, cursor::Selection, state::CodeEditor, syntax};
use crate::{
    editor::{EditorEvent, anchors},
    forms::{Highlight, History},
};

/// Undo keeps at most this many bytes of past text.
pub(crate) const HISTORY_BYTES: usize = 64 << 20;

/// Colors for one line of text.
type Paint = dyn Fn(&str, &App) -> Vec<(Range<usize>, Highlight)>;

/// The owner's colors for each line.
#[derive(Clone)]
pub(crate) struct LineHighlighter(pub Rc<Paint>);

impl fmt::Debug for LineHighlighter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LineHighlighter")
    }
}

impl CodeEditor {
    /// Colors each line with the owner's highlighter, not code's.
    pub fn highlighter(
        mut self,
        highlight: impl Fn(&str, &App) -> Vec<(Range<usize>, Highlight)> + 'static,
    ) -> Self {
        self.options.highlighter = Some(LineHighlighter(Rc::new(highlight)));
        self
    }

    /// New text with no history: undo starts here.
    pub fn reset(&mut self, text: impl Into<String>, cx: &mut Context<Self>) {
        self.buffer = Buffer::new(text);
        self.selections = vec![Selection::caret(0)];
        self.history = History::default();
        self.folded.clear();
        (self.marked, self.composing) = (None, None);
        anchors::clear(&mut self.marks);
        cx.emit(EditorEvent::Changed);
        self.restart_blink(cx);
    }

    /// Keeps undo within its byte budget, logging what went.
    pub(crate) fn cap_history(&mut self) {
        let dropped = self
            .history
            .trim(self.history_bytes, |step| step.text.len());
        if dropped > 0 {
            log::info!("code editor: {dropped} far undo steps dropped");
        }
    }

    /// A line's colors, as the owner or as code.
    pub(crate) fn line_colors(&self, text: &str, cx: &App) -> Vec<(Range<usize>, HighlightStyle)> {
        let Some(LineHighlighter(highlight)) = &self.options.highlighter else {
            return syntax::colors(text, cx);
        };
        let rule = crate::theme::ActiveTheme::theme(cx).underline_thickness();
        highlight(text, cx)
            .into_iter()
            .map(|(range, highlight)| {
                let style = HighlightStyle {
                    color: Some(highlight.color),
                    background_color: highlight.background,
                    font_weight: highlight.weight,
                    font_style: highlight.italic.then_some(FontStyle::Italic),
                    strikethrough: highlight.strike.then_some(StrikethroughStyle {
                        thickness: rule,
                        color: Some(highlight.color),
                    }),
                    ..HighlightStyle::default()
                };
                (range, style)
            })
            .collect()
    }
}
