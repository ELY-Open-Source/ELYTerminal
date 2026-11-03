use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, MouseButton, ParentElement, Stateful,
    Styled, StyledText, Window, div, prelude::*,
};

use super::{
    chat::InlineChat,
    gutter::Cell,
    layout::{CHAT_ROWS, Row},
    state::{CodeEditor, EditorEvent},
};
use crate::theme::{ActiveTheme, TextSize};

/// How far a faded line recedes.
const FADED: f32 = 0.35;

impl CodeEditor {
    /// Row `ix` as the list draws it.
    pub(crate) fn row(&self, ix: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let base = div()
            .id(("editor-row", ix))
            .w_full()
            .h(self.frame.line)
            .flex();
        let caret = self.buffer.line_of(self.primary().head);
        let base = match self.frame.rows[ix] {
            Row::Line(line) => base.debug_selector(move || format!("editor-line-{line}")),
            _ => base,
        };
        match self.frame.rows[ix] {
            Row::Line(line) if self.options.focus_line && line != caret => {
                self.line_row(base.opacity(FADED), ix, line, window, cx)
            }
            Row::Line(line) => self.line_row(base, ix, line, window, cx),
            Row::Lens(at) => self.lens_row(base, ix, at, cx),
            Row::Removed(hunk, at) => self.removed_row(base, ix, hunk, at, cx),
            Row::Ghost(at) => self.ghost_row(base, ix, at, cx),
            Row::Chat(at) => self.chat_row(base, ix, at, cx),
            Row::Room => base.into_any_element(),
        }
    }

    fn lens_row(
        &self,
        base: Stateful<gpui::Div>,
        ix: usize,
        at: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let gutter = self.gutter(Cell::Blank, ix, cx);
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let lens = &self.marks.lenses[at];
        let indent = self.buffer.indent(lens.line);
        let items = lens.items.iter().enumerate().map(|(item, words)| {
            let line = lens.line;
            div()
                .id(("lens", item))
                .cursor_pointer()
                .hover(|style| style.text_color(colors.link))
                .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                .on_click(cx.listener(move |_, _, _, cx| {
                    log::info!("code lens: line {line}, item {item}");
                    cx.emit(EditorEvent::Lens(line, item));
                }))
                .child(words.clone())
        });
        let items: Vec<AnyElement> = items
            .enumerate()
            .flat_map(|(place, item)| {
                let gap = (place > 0).then(|| div().child("·").into_any_element());
                gap.into_iter().chain([item.into_any_element()])
            })
            .collect();
        base.child(gutter)
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .pl(self.frame.advance * (indent + 1) as f32)
                    .font_family(theme.font_family.clone())
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.fg_subtle)
                    .children(items),
            )
            .into_any_element()
    }

    fn removed_row(
        &self,
        base: Stateful<gpui::Div>,
        ix: usize,
        hunk: usize,
        at: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colors = cx.theme().colors.clone();
        let text = self.marks.hunks[hunk].removed[at].clone();
        base.child(self.gutter(Cell::Removed, ix, cx))
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .pl(self.frame.advance)
                    .bg(colors.danger.opacity(0.08))
                    .line_height(self.frame.line)
                    .whitespace_nowrap()
                    .text_color(colors.fg_muted)
                    .child(text),
            )
            .into_any_element()
    }

    fn ghost_row(
        &self,
        base: Stateful<gpui::Div>,
        ix: usize,
        at: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colors = cx.theme().colors.clone();
        let ghost = self
            .marks
            .ghost
            .as_ref()
            .expect("a ghost row has its ghost");
        let lines: Vec<&str> = ghost.text.split('\n').collect();
        let text = lines[at].to_string();
        let rest = (at + 1 == lines.len()).then(|| {
            let line = self.buffer.line_of(ghost.offset);
            let rest =
                self.buffer.text()[ghost.offset..self.buffer.line_range(line).end].to_string();
            let styles = self.line_colors(&rest, cx);
            StyledText::new(rest).with_highlights(styles)
        });
        base.child(self.gutter(Cell::Blank, ix, cx))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .pl(self.frame.advance)
                    .line_height(self.frame.line)
                    .whitespace_nowrap()
                    .child(div().italic().text_color(colors.fg_subtle).child(text))
                    .children(rest),
            )
            .into_any_element()
    }

    fn chat_row(
        &self,
        base: Stateful<gpui::Div>,
        ix: usize,
        at: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let gutter = self.gutter(Cell::Blank, ix, cx);
        let theme = cx.theme();
        let row = base.child(gutter);
        if at > 0 {
            return row.into_any_element();
        }
        let asked = cx.entity();
        let closed = cx.entity();
        let field = self.chat_field.clone();
        row.child(
            div().relative().flex_1().h_full().child(
                InlineChat::new(("inline-chat", ix), &self.chat_field)
                    .absolute()
                    .top_0()
                    .left(self.frame.advance)
                    .w(self.frame.advance * 72.0)
                    .h(self.frame.line * CHAT_ROWS as f32)
                    .font_family(theme.font_family.clone())
                    .on_ask(move |prompt, _, cx| {
                        field.update(cx, |field, cx| field.set_text("", cx));
                        asked.update(cx, |_, cx| cx.emit(EditorEvent::Asked(prompt)));
                    })
                    .on_close(move |window, cx| {
                        closed.update(cx, |editor, cx| editor.set_chat(None, window, cx));
                    }),
            ),
        )
        .into_any_element()
    }
}
