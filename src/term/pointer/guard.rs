//! The question before a paste a shell would run at once.

use alacritty_terminal::term::TermMode;
use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    i18n::text,
    overlays::{Close, Dialog},
    theme::{ActiveTheme, TextSize},
    typography::Caption,
};
use gpui::{App, Context, ParentElement, Styled, div};

use crate::{sessions::plain, settings::SettingsStore, term::view::TerminalView};

/// The lines a preview shows.
const SHOWN: usize = 8;

/// A line break outside a bracketed paste runs what came before it.
pub(super) fn risky(text: &str, bracketed: bool) -> bool {
    !bracketed && text.contains(['\n', '\r'])
}

/// Its lines, broken at CR LF, CR or LF, as a shell reads them.
fn lines(text: &str) -> Vec<&str> {
    let mut lines: Vec<&str> = text
        .split("\r\n")
        .flat_map(|part| part.split(['\r', '\n']))
        .collect();
    if lines.last() == Some(&"") {
        lines.pop();
    }
    lines
}

impl TerminalView {
    /// Pastes `text`, or holds it for the user's word when a shell would run it.
    pub(super) fn guard(&mut self, text: String, cx: &mut Context<Self>) {
        let bracketed = self
            .pty
            .term
            .lock()
            .mode()
            .contains(TermMode::BRACKETED_PASTE);
        if SettingsStore::get(cx).paste_protection && risky(&text, bracketed) {
            log::info!(
                "terminal {}: a paste of {} lines waits for the user",
                self.pty.pid,
                lines(&text).len()
            );
            self.pasting = Some(text);
            return cx.notify();
        }
        // One mode for check and send.
        self.paste_as(&text, bracketed, cx);
    }

    /// The user's word on the held paste.
    fn settle(&mut self, go: bool, cx: &mut Context<Self>) {
        let Some(text) = self.pasting.take() else {
            return;
        };
        log::info!(
            "terminal {}: the held paste {}",
            self.pty.pid,
            if go { "goes" } else { "is dropped" }
        );
        if go {
            self.paste_text(&text, cx);
        }
        cx.notify();
    }

    /// The question over the window; Escape cancels.
    pub(in crate::term) fn paste_prompt(&self, cx: &Context<Self>) -> Option<Dialog> {
        let pasting = self.pasting.as_ref()?;
        let lines = lines(pasting);
        let count = lines.len();
        let view = cx.entity();
        let settle = move |go| {
            let view = view.clone();
            move |cx: &mut App| view.update(cx, |view, cx| view.settle(go, cx))
        };
        let closed = settle(false);
        let theme = cx.theme();
        let shown = lines
            .into_iter()
            .take(SHOWN)
            .map(|line| div().child(plain(line)));
        let more = count.saturating_sub(SHOWN);
        let rest = (more > 0)
            .then(|| Caption::new(text(cx, "et.paste.more", &[("count", &more.to_string())])));
        let preview = div()
            .flex()
            .flex_col()
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(theme.colors.fg)
            .children(shown)
            .children(rest);
        let button = |id: &'static str, key: &str, go, variant| {
            let label = text(cx, key, &[]);
            let pick = settle(go);
            move |close: Close| {
                Button::new(id, label)
                    .variant(variant)
                    .on_click(move |_, window, cx| {
                        pick(cx);
                        close(window, cx);
                    })
            }
        };
        Some(
            Dialog::new(
                "paste-guard",
                text(cx, "et.paste.title", &[("count", &count.to_string())]),
                move |_, cx| closed(cx),
            )
            .detail(text(cx, "et.paste.detail", &[]))
            .child(preview)
            .action(button(
                "paste-cancel",
                "et.paste.cancel",
                false,
                ButtonVariant::Secondary,
            ))
            .action(button(
                "paste-go",
                "et.paste.go",
                true,
                ButtonVariant::Primary,
            )),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{lines, risky};

    #[test]
    fn a_line_break_outside_a_bracketed_paste_is_risky() {
        assert!(risky("rm -rf build\nmake\n", false));
        assert!(risky("one\rtwo", false));
        assert!(!risky("one line", false));
        assert!(!risky("two\nlines", true), "a bracketed paste runs nothing");
    }

    #[test]
    fn a_lone_cr_breaks_a_line_as_lf_and_cr_lf_do() {
        assert_eq!(lines("one\rtwo\rthree"), ["one", "two", "three"]);
        assert_eq!(lines("a\r\nb\nc\r\n"), ["a", "b", "c"]);
        assert_eq!(lines("a\n\nb"), ["a", "", "b"]);
    }
}
