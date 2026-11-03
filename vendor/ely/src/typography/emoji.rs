use gpui::{App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div};

use crate::theme::{ActiveTheme, TextSize};

/// One emoji in the system color font.
#[derive(IntoElement)]
pub struct Emoji {
    glyph: SharedString,
    size: TextSize,
}

impl Emoji {
    pub fn new(glyph: impl Into<SharedString>) -> Self {
        Self {
            glyph: glyph.into(),
            size: TextSize::Lg,
        }
    }

    /// GitHub shortcode without colons; `None` if unknown.
    pub fn shortcode(code: &str) -> Option<Self> {
        emojis::get_by_shortcode(code).map(|emoji| Self::new(emoji.as_str()))
    }

    pub fn size(mut self, size: TextSize) -> Self {
        self.size = size;
        self
    }
}

impl RenderOnce for Emoji {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .text_size(cx.theme().text_size(self.size))
            .child(self.glyph)
    }
}

#[cfg(test)]
mod tests {
    use super::Emoji;

    #[test]
    fn shortcodes_resolve_or_refuse() {
        let glyph = Emoji::shortcode("sparkles").map(|emoji| emoji.glyph.to_string());
        assert_eq!(glyph.as_deref(), Some("✨"));
        assert!(Emoji::shortcode("not_an_emoji_name").is_none());
    }
}
