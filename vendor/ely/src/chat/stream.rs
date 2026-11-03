use std::time::Duration;

use gpui::{
    App, ElementId, HighlightStyle, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    StyledText, Window, div,
};
use unicode_segmentation::UnicodeSegmentation;
use web_time::Instant;

use super::code::CodeBlock;
use crate::{
    documents::MarkdownRenderer,
    motion::Blink,
    theme::{ActiveTheme, Radius, TextSize},
};

/// Graphemes a stream shows per second when it keeps up.
const PACE: f32 = 90.0;
/// The longest a backlog takes to show, however much arrived at once.
const CATCH_UP: Duration = Duration::from_millis(350);
/// The mark where more will come, set inline after the text.
const CARET: &str = "▍";

/// How far a reveal has come, in graphemes with the fraction carried, and when it last moved.
struct Reveal {
    shown: f32,
    at: Instant,
}

/// Graphemes shown after `elapsed`, from `shown` toward `total`: a steady pace, faster when far behind.
pub(crate) fn advanced(shown: f32, total: usize, elapsed: Duration) -> f32 {
    let behind = total as f32 - shown;
    if behind <= 0.0 {
        return total as f32;
    }
    let rate = PACE.max(behind / CATCH_UP.as_secs_f32());
    (shown + rate * elapsed.as_secs_f32()).min(total as f32)
}

/// Graphemes shown next: `advanced` toward `total` while live, or all of them once finished or when motion rests.
pub(crate) fn next_shown(
    shown: f32,
    total: usize,
    still: bool,
    elapsed: Duration,
    live: bool,
) -> f32 {
    if still || !live {
        return total as f32;
    }
    advanced(shown.min(total as f32), total, elapsed)
}

/// Where the first `count` graphemes of `text` end, in bytes.
pub(crate) fn grapheme_end(text: &str, count: usize) -> usize {
    text.grapheme_indices(true)
        .nth(count)
        .map_or(text.len(), |(at, _)| at)
}

/// How much of `text` shows now, in bytes: a live stream reveals from its first frame, a finished one shows whole. Asks for frames until caught up; under reduced motion, all at once.
fn revealed(id: &ElementId, text: &str, live: bool, window: &mut Window, cx: &mut App) -> usize {
    let total = text.graphemes(true).count();
    let now = cx.background_executor().now();
    let state = window.use_keyed_state((id.clone(), "reveal"), cx, |_, _| Reveal {
        shown: if live { 0.0 } else { total as f32 },
        at: now,
    });
    let (shown, at) = (state.read(cx).shown, state.read(cx).at);
    let shown = next_shown(
        shown,
        total,
        cx.theme().reduced_motion,
        now.saturating_duration_since(at),
        live,
    );
    state.update(cx, |reveal, _| *reveal = Reveal { shown, at: now });
    if (shown as usize) < total {
        window.request_animation_frame();
    }
    grapheme_end(text, shown as usize)
}

/// Text as it arrives: what came in shows at a steady pace, catching up when far behind, with a caret while more may come.
#[derive(IntoElement)]
pub struct StreamingText {
    id: ElementId,
    text: SharedString,
    live: bool,
}

impl StreamingText {
    /// `live` while more may arrive; a finished message shows whole.
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>, live: bool) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            live,
        }
    }
}

impl RenderOnce for StreamingText {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let end = revealed(&self.id, &self.text, self.live, window, cx);
        let caret = cx.theme().colors.fg_muted;
        let shown = if self.live {
            format!("{}{CARET}", &self.text[..end])
        } else {
            self.text[..end].to_string()
        };
        let styles = self
            .live
            .then(|| {
                (
                    end..end + CARET.len(),
                    HighlightStyle {
                        color: Some(caret),
                        ..HighlightStyle::default()
                    },
                )
            })
            .into_iter();
        StyledText::new(shown).with_highlights(styles)
    }
}

/// Where more will come before any text has: a small block that blinks while it waits, still under reduced motion.
#[derive(IntoElement)]
pub struct StreamingCursor {
    id: ElementId,
}

impl StreamingCursor {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self { id: id.into() }
    }
}

impl RenderOnce for StreamingCursor {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let size = theme.text_size(TextSize::Base);
        Blink::new(self.id).child(
            div()
                .w(size * 0.5)
                .h(size * 1.2)
                .rounded(theme.radius(Radius::Sm))
                .bg(theme.colors.fg_muted),
        )
    }
}

/// Markdown as it arrives, revealed like streaming text: headings, lists, tables and math take shape as their lines land, code in code blocks.
#[derive(IntoElement)]
pub struct StreamingMarkdown {
    id: ElementId,
    source: SharedString,
    live: bool,
}

impl StreamingMarkdown {
    /// `live` while more may arrive; a finished message shows whole.
    pub fn new(id: impl Into<ElementId>, source: impl Into<SharedString>, live: bool) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
            live,
        }
    }
}

impl RenderOnce for StreamingMarkdown {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let end = revealed(&self.id, &self.source, self.live, window, cx);
        let shown = if self.live {
            format!("{}{CARET}", &self.source[..end])
        } else {
            self.source[..end].to_string()
        };
        MarkdownRenderer::new((self.id.clone(), "text"), shown).code(|id, language, code| {
            let block = CodeBlock::new(id, code);
            match language {
                Some(language) => block.language(language),
                None => block,
            }
            .into_any_element()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reveal_keeps_pace_and_catches_up() {
        let tick = Duration::from_millis(100);
        assert_eq!(
            advanced(0.0, 5, tick),
            5.0,
            "a short line shows within a tick"
        );
        let long = advanced(0.0, 1000, tick);
        assert!(long > PACE * 0.1, "far behind, it speeds up: {long}");
        assert!(long < 1000.0, "but still reveals: {long}");
        assert_eq!(advanced(12.0, 10, tick), 10.0, "shorter text clamps");
    }

    #[test]
    fn a_finished_stream_shows_whole() {
        let tick = Duration::from_millis(16);
        assert_eq!(
            next_shown(3.0, 400, false, tick, false),
            400.0,
            "done: whole at once"
        );
        assert!(
            next_shown(3.0, 400, false, tick, true) < 400.0,
            "live: it keeps revealing"
        );
        assert_eq!(
            next_shown(3.0, 400, true, tick, true),
            400.0,
            "still motion: whole"
        );
    }

    #[test]
    fn a_count_of_graphemes_ends_on_a_boundary() {
        assert_eq!(grapheme_end("héllo", 2), 3);
        assert_eq!(grapheme_end("👩‍🚀 ok", 1), "👩‍🚀".len());
        assert_eq!(grapheme_end("ok", 9), 2);
    }
}
