use std::ops::Range;

use gpui::{
    AnyElement, Bounds, Context, ElementInputHandler, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, ParentElement, Pixels, Render, ScrollStrategy, Styled,
    StyledText, Window, canvas, div, fill, font, point, size, uniform_list,
};

use super::{
    decor::ReadOnlyBanner,
    keys::{CONTEXT, listen},
    layout::{Row, around, hidden, rows},
    state::{CodeEditor, EditorEvent},
    syntax::{self, Bracket, brackets, folds},
};
use crate::theme::{ActiveTheme, TextSize};

/// What one render worked out, for the rows the list builds after it.
#[derive(Default)]
pub(crate) struct Frame {
    pub rows: Vec<Row>,
    pub folds: Vec<(usize, usize)>,
    pub brackets: Vec<Bracket>,
    pub advance: Pixels,
    pub line: Pixels,
    pub matched: Option<(usize, usize)>,
    pub occurrences: Vec<Range<usize>>,
    /// The buffer stamp these offsets were found in.
    pub stamp: u64,
    /// The minimap's box, from its last paint.
    pub minimap: Bounds<Pixels>,
}

/// Lines of code a row's height holds, as leading.
const LEADING: f32 = 1.6;
/// Blocks the sticky header shows at most.
const STICKY: usize = 3;
/// Columns the minimap spans, in code characters.
const MINIMAP: f32 = 12.0;

impl CodeEditor {
    /// The text the primary selection or its word gives, for highlighting where else it appears.
    fn needle(&self) -> Option<String> {
        let primary = self.primary();
        let range = if primary.is_empty() {
            let word = self.buffer.word_at(primary.head);
            let text = &self.buffer.text()[word.clone()];
            text.chars()
                .all(|ch| ch.is_alphanumeric() || ch == '_')
                .then_some(word)?
        } else {
            primary.range()
        };
        (!range.is_empty()).then(|| self.buffer.text()[range].to_string())
    }

    /// The first line the list shows, after scrolling.
    pub(crate) fn top_line(&self) -> Option<usize> {
        let scrolled = -self.scroll.0.borrow().base_handle.offset().y;
        let first = (scrolled / self.frame.line).floor().max(0.0) as usize;
        self.frame.rows[first.min(self.frame.rows.len().saturating_sub(1))..]
            .iter()
            .find_map(|row| match row {
                Row::Line(line) => Some(*line),
                _ => None,
            })
    }

    fn sticky(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let colors = cx.theme().colors.clone();
        let top = self.top_line()?;
        let headers = around(&self.frame.folds, top, STICKY);
        if headers.is_empty() {
            return None;
        }
        let rows = headers.into_iter().map(|line| {
            let text = self.buffer.line(line).to_string();
            let styles = self.line_colors(&text, cx);
            let jump = cx.entity();
            div()
                .id(("sticky", line))
                .h(self.frame.line)
                .flex()
                .items_center()
                .cursor_pointer()
                .hover(|style| style.bg(colors.hover))
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    window.prevent_default();
                    jump.update(cx, |editor, cx| {
                        if line >= editor.buffer.lines() {
                            return;
                        }
                        let at = editor.buffer.line_range(line).start + editor.buffer.indent(line);
                        let cursor = at..at;
                        editor.select([cursor], cx);
                        editor.reveal(false, cx);
                    });
                })
                .child(
                    div()
                        .flex_none()
                        .w(self.frame.advance * (self.gutter_columns() + 1) as f32),
                )
                .child(
                    div()
                        .line_height(self.frame.line)
                        .whitespace_nowrap()
                        .text_color(colors.syntax.variable)
                        .child(StyledText::new(text).with_highlights(styles)),
                )
        });
        Some(
            div()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .bg(colors.surface)
                .border_b_1()
                .border_color(colors.border)
                .children(rows)
                .into_any_element(),
        )
    }

    fn minimap_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let colors = cx.theme().colors.clone();
        let (advance, line) = (self.frame.advance, self.frame.line);
        let tokens: Vec<Vec<(usize, usize, gpui::Hsla)>> = self
            .frame
            .rows
            .iter()
            .map(|row| match row {
                Row::Line(at) => {
                    let text = self.buffer.line(*at);
                    self.line_colors(text, cx)
                        .into_iter()
                        .filter_map(|(range, style)| {
                            let start = text[..range.start].chars().count();
                            let len = text[range].chars().count();
                            style.color.map(|color| (start, len, color))
                        })
                        .collect()
                }
                _ => Vec::new(),
            })
            .collect();
        let scroll = self.scroll.clone();
        let entity = cx.entity();
        let (drag, press) = (cx.entity(), cx.entity());
        let shade = colors.hover;
        div()
            .id("minimap")
            .relative()
            .flex_none()
            .w(advance * MINIMAP)
            .h_full()
            .border_l_1()
            .border_color(colors.border)
            .cursor_pointer()
            .child(
                canvas(
                    move |bounds, _, cx| {
                        entity.update(cx, |editor, _| editor.frame.minimap = bounds);
                    },
                    move |bounds, _, window, _| {
                        let rows = tokens.len().max(1) as f32;
                        let tall = (line * 0.15).min(bounds.size.height / rows);
                        let wide = advance * 0.12;
                        let state = scroll.0.borrow();
                        let viewport = state.base_handle.bounds().size.height;
                        let scrolled = -state.base_handle.offset().y;
                        let top = bounds.origin.y + scrolled / line * tall;
                        let extent = size(bounds.size.width, viewport / line * tall);
                        window.paint_quad(fill(
                            Bounds::new(point(bounds.origin.x, top), extent),
                            shade,
                        ));
                        for (ix, spans) in tokens.iter().enumerate() {
                            for (start, len, color) in spans {
                                let origin = point(
                                    bounds.origin.x + wide * (*start + 2) as f32,
                                    bounds.origin.y + tall * ix as f32,
                                );
                                let extent = size(wide * *len as f32, tall * 0.7);
                                window.paint_quad(fill(
                                    Bounds::new(origin, extent),
                                    color.opacity(0.7),
                                ));
                            }
                        }
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
                press.update(cx, |editor, cx| editor.jump(event.position.y, cx));
            })
            .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                if event.dragging() {
                    drag.update(cx, |editor, cx| editor.jump(event.position.y, cx));
                }
            })
            .into_any_element()
    }

    /// Scrolls so the row under a minimap point sits in the middle of the view.
    fn jump(&mut self, y: Pixels, cx: &mut Context<Self>) {
        let bounds = self.frame.minimap;
        let rows = self.frame.rows.len().max(1) as f32;
        let tall = (self.frame.line * 0.15).min(bounds.size.height / rows);
        let row = ((y - bounds.origin.y) / tall).max(0.0);
        let state = self.scroll.0.borrow();
        let viewport = state.base_handle.bounds().size.height;
        let top = (self.frame.line * row - viewport / 2.0).max(Pixels::ZERO);
        state.base_handle.set_offset(point(Pixels::ZERO, -top));
        drop(state);
        cx.notify();
    }
}

impl Render for CodeEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let mono_family = theme.mono_family.clone();
        let size = theme.text_size(TextSize::Sm) * self.options.scale;
        let pixels = size.to_pixels(window.rem_size());
        let mono = window
            .text_system()
            .resolve_font(&font(theme.mono_family.clone()));
        let advance = match window.text_system().advance(mono, pixels, 'm') {
            Ok(advance) => advance.width,
            Err(error) => {
                log::error!(
                    "code editor: no advance for the code font: {error:#}; columns guessed"
                );
                pixels * 0.6
            }
        };
        let line = (pixels * LEADING).round();
        let found = folds(&self.buffer);
        let hide = hidden(self.buffer.lines(), &found, &self.folded);
        let found_brackets = brackets(&self.buffer);
        let matched = syntax::matched(&found_brackets, self.primary().head);
        let occurrences = match self.needle() {
            Some(needle) if self.focus.is_focused(window) => {
                syntax::occurrences(&self.buffer, &needle, 0..self.buffer.lines())
            }
            _ => Vec::new(),
        };
        let minimap = self.frame.minimap;
        // Half a view of room rows: the ends center too.
        self.pad = match self.options.typewriter && line > Pixels::ZERO {
            true => (self.metrics.height / 2.0 / line).ceil() as usize,
            false => 0,
        };
        let mut shown = vec![Row::Room; self.pad];
        shown.extend(rows(&self.buffer, &hide, &self.marks));
        shown.extend(std::iter::repeat_n(Row::Room, self.pad));
        self.frame = Frame {
            rows: shown,
            folds: found,
            brackets: found_brackets,
            advance,
            line,
            matched,
            occurrences,
            stamp: self.buffer.stamp(),
            minimap,
        };
        self.metrics.advance = advance;
        self.metrics.line = line;
        self.metrics.size = pixels;
        // Typewriter: the caret's row centers as it moves.
        let centered = (self.primary_row(), self.metrics.height, line);
        if self.options.typewriter && self.centered != Some(centered) {
            self.scroll
                .scroll_to_item_strict(centered.0, ScrollStrategy::Center);
            self.centered = Some(centered);
        }
        let sticky = if self.options.sticky {
            self.sticky(cx)
        } else {
            None
        };
        let count = self.frame.rows.len();
        let list = uniform_list(
            "editor-rows",
            count,
            cx.processor(|editor, range: Range<usize>, window, cx| {
                range.map(|ix| editor.row(ix, window, cx)).collect()
            }),
        )
        .track_scroll(&self.scroll)
        .size_full();
        let (top, focus, input) = (cx.entity(), self.focus.clone(), cx.entity());
        let unlock = cx.entity();
        let banner = self.options.read_only.then(|| {
            ReadOnlyBanner::new(
                ("read-only", cx.entity_id().as_u64()),
                "This file is read-only.",
            )
            .action("Make editable", move |_, cx| {
                unlock.update(cx, |editor, cx| {
                    editor.set_read_only(false, cx);
                    cx.emit(EditorEvent::Unlock);
                })
            })
        });
        let body = div()
            .relative()
            .flex_1()
            .h_full()
            .min_h_0()
            .child(list)
            .children(sticky)
            .child(
                canvas(
                    move |bounds, window, cx| {
                        let resized = top.update(cx, |editor, _| {
                            editor.metrics.top = bounds.origin.y;
                            let resized = editor.metrics.height != bounds.size.height;
                            editor.metrics.height = bounds.size.height;
                            resized
                        });
                        // A new height centers the caret again.
                        if resized {
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
        let root = div()
            .id(("code-editor", cx.entity_id().as_u64()))
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(colors.surface)
            .font_family(mono_family)
            .text_size(size)
            .text_color(colors.fg)
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|editor, _, _, _| editor.dragging = false),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|editor, _, _, _| editor.dragging = false),
            )
            .children(banner)
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(body)
                    .children(self.options.minimap.then(|| self.minimap_view(cx))),
            )
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, cx| {
                        window.handle_input(
                            &focus,
                            ElementInputHandler::new(bounds, input.clone()),
                            cx,
                        );
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
        listen(root, cx)
    }
}
