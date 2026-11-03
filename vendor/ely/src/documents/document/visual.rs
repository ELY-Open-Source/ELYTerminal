use gpui::{
    AnyElement, App, ClickEvent, Context, Div, ExternalPaths, FontWeight, InteractiveElement,
    IntoElement, KeyBinding, MouseButton, MouseDownEvent, ParentElement, Render,
    StatefulInteractiveElement, Styled, StyledText, Window, actions, div, list,
    prelude::FluentBuilder as _, relative,
};

use super::{MarkdownDocument, edit::Landing, parse::Block};
use crate::{
    documents::{
        MarkdownMode, TargetBlock,
        render::{HEAVY, Hooks, OpenCell, Pointed, draw_nodes, heading_step, point_lines},
    },
    forms::{self, Down, Up},
    primitives::FocusRing as _,
    theme::{ActiveTheme, Radius, TextSize, Theme},
};

actions!(markdown_document, [Undo, Redo, Indent, Outdent, Paste]);

const CONTEXT: &str = "MarkdownDocument";

/// How far a faded block recedes.
const FADED: f32 = 0.35;

/// The open block's text box: its closed font and panel.
pub(super) fn open_text(kind: Option<(TargetBlock, bool)>, scale: f32, theme: &Theme) -> Div {
    let colors = &theme.colors;
    let body = theme.text_size(TextSize::Base) * scale;
    let mono = theme.text_size(TextSize::Sm) * scale;
    let text = div()
        .debug_selector(|| "markdown-open-text".to_string())
        .text_size(body)
        .line_height(body * 1.6)
        .text_color(colors.fg);
    match kind {
        Some((TargetBlock::Heading(level), true)) => text
            .pt_2()
            .text_size(body * heading_step(level))
            .line_height(relative(1.3))
            .font_weight(HEAVY)
            .when(level == 6, |text| text.text_color(colors.fg_muted)),
        // Code, math and front matter keep their panel.
        Some((kind @ (TargetBlock::Code | TargetBlock::Math | TargetBlock::FrontMatter), true)) => {
            text.p_3()
                .rounded(theme.radius(Radius::Md))
                .bg(colors.sunken)
                .when(kind != TargetBlock::Math, |text| {
                    text.font_family(theme.mono_family.clone()).text_size(mono)
                })
                .when(kind == TargetBlock::FrontMatter, |text| {
                    text.text_color(colors.fg_muted)
                })
        }
        // Written as is: mono and muted.
        Some((_, false)) => text
            .font_family(theme.mono_family.clone())
            .line_height(gpui::phi())
            .text_color(colors.fg_muted),
        _ => text,
    }
}

/// Modes with a reader's block to keep.
fn editing_mode(mode: MarkdownMode) -> bool {
    mode == MarkdownMode::Visual
}

/// Undo and redo; Tab shifts an open block's lines.
pub(crate) fn bind_keys(cx: &mut App) {
    let context = Some(CONTEXT);
    let block = Some("MarkdownDocument > ElyInput");
    let mut bindings = vec![
        KeyBinding::new("secondary-z", Undo, context),
        KeyBinding::new("secondary-shift-z", Redo, context),
        KeyBinding::new("secondary-v", Paste, context),
        KeyBinding::new("tab", Indent, block),
        KeyBinding::new("shift-tab", Outdent, block),
    ];
    if !cfg!(target_os = "macos") {
        bindings.push(KeyBinding::new("ctrl-y", Redo, context));
    }
    cx.bind_keys(bindings);
}

impl MarkdownDocument {
    /// List item `ix`, with typewriter room at the ends.
    fn item(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let (first, last) = (ix == 0, ix == self.parsed.blocks.len());
        let room = self.room;
        div()
            .when(first, |item| item.pt(room))
            .when(last, |item| item.pb(room))
            .child(self.block_item(ix, window, cx))
            .into_any_element()
    }

    /// A block, or the end.
    fn block_item(&mut self, ix: usize, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let editing = self.mode == MarkdownMode::Visual;
        let drawn = editing && ix < self.parsed.blocks.len();
        let on_point = drawn.then(|| self.on_point(ix, cx));
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let (radius, body, mono) = (
            theme.radius(Radius::Md),
            theme.text_size(TextSize::Base) * self.look.scale,
            theme.mono_family.clone(),
        );
        // Focus fades all but the reader's block.
        let faded = self.look.focus && editing_mode(self.mode) && ix != self.current_block(cx);
        let open = self.open.as_ref();
        let Some(block) = self.parsed.blocks.get(ix) else {
            return match (open, editing) {
                (Some(open), _) if open.append => self.field(open, None, cx),
                (_, true) => self
                    .pressable(div().id("more").min_h_8(), None, cx)
                    .into_any_element(),
                (_, false) => div().into_any_element(),
            };
        };
        if let Some(open) = open.filter(|open| open.cell.is_none() && open.range == block.range) {
            return self.field(open, Some(block), cx);
        }
        // A cell open in this block, typed in place.
        let cell = open.and_then(|open| {
            let inside = block.range.start <= open.range.start && open.range.end <= block.range.end;
            Some(OpenCell {
                at: open.cell.filter(|_| inside)?,
                field: std::cell::RefCell::new(Some(
                    div()
                        .debug_selector(|| "markdown-open-cell".to_string())
                        .on_mouse_down(MouseButton::Right, Self::point_in_open(cx))
                        .rounded(radius)
                        .bg(colors.sunken)
                        .child(open.input.clone())
                        .into_any_element(),
                )),
            })
        });
        let shown = match &block.nodes {
            Some(nodes) => div()
                .flex()
                .flex_col()
                .gap_3()
                .text_size(body)
                .line_height(body * 1.6)
                .text_color(colors.fg)
                .children(draw_nodes(
                    &("render", ix).into(),
                    nodes,
                    Hooks {
                        base: &self.base,
                        on_link: &self.on_link,
                        on_heading: &self.on_heading,
                        on_point: &on_point,
                        cell: cell.as_ref(),
                    },
                    self.look.scale,
                    cx,
                )),
            None => {
                let raw = &self.parsed.text[block.range.clone()];
                let drawn = StyledText::new(raw.to_string());
                let layout = drawn.layout().clone();
                let shown = div()
                    .text_size(body)
                    .font_family(mono)
                    .text_color(colors.fg_muted)
                    .child(drawn)
                    .into_any_element();
                div().child(point_lines(
                    shown,
                    layout,
                    raw,
                    0,
                    &on_point,
                    (false, false),
                ))
            }
        };
        let block = div()
            .id(("block", ix))
            .debug_selector(move || format!("markdown-block-{ix}"))
            .px_3()
            .py_1()
            .rounded(radius)
            .when(faded, |block| block.opacity(FADED))
            .child(shown);
        match editing {
            true => self.pressable(block, Some(ix), cx).into_any_element(),
            false => block.into_any_element(),
        }
    }

    /// The open block in its closed frame, size and weight.
    fn field(
        &self,
        open: &super::Open,
        block: Option<&Block>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let point = Self::point_in_open(cx);
        let kind = block.map(|block| (block.kind, block.nodes.is_some()));
        let text = open_text(kind, self.look.scale, cx.theme());
        div()
            .debug_selector(|| "markdown-open".to_string())
            .on_mouse_down(MouseButton::Right, point)
            .px_3()
            .py_1()
            .border_1()
            .border_color(gpui::transparent_black())
            .child(text.child(open.input.clone()))
            .into_any_element()
    }

    /// A right press in the open field points there.
    fn point_in_open(
        cx: &mut Context<Self>,
    ) -> impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static {
        cx.listener(|document, event: &MouseDownEvent, window, cx| {
            window.prevent_default();
            cx.stop_propagation();
            document.point_open(Some(event.position), cx);
        })
    }

    /// `element` opens block `ix`; a right press points.
    fn pressable(
        &self,
        element: gpui::Stateful<gpui::Div>,
        ix: Option<usize>,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let hover = cx.theme().colors.hover.opacity(0.5);
        element
            .tab_index(0)
            .border_1()
            .border_color(gpui::transparent_black())
            .focus_ring(cx)
            .cursor_text()
            .hover(move |element| element.bg(hover))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |document, event: &MouseDownEvent, window, cx| {
                    // Cmd or Ctrl leaves the press to a link.
                    if event.modifiers.secondary() {
                        return;
                    }
                    window.prevent_default();
                    cx.stop_propagation();
                    document.open(ix, Landing::End, window, cx);
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |document, _: &MouseDownEvent, window, cx| {
                    window.prevent_default();
                    cx.stop_propagation();
                    document.point(ix, None, Pointed::block(), window, cx);
                }),
            )
            .on_click(
                cx.listener(move |document, event: &ClickEvent, window, cx| {
                    if event.is_keyboard() {
                        document.open(ix, Landing::End, window, cx);
                    }
                }),
            )
    }
}

impl Render for MarkdownDocument {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.take_orphan(window, cx);
        // A top line asked before its text was read.
        self.place_top(window, cx);
        self.show_waiting(window, cx);
        self.open_waiting(window, cx);
        let colors = cx.theme().colors.clone();
        let root = div()
            .size_full()
            .bg(colors.surface)
            .font_weight(FontWeight::NORMAL)
            .on_drop(cx.listener(|_, dropped: &ExternalPaths, _, cx| {
                log::info!("markdown document: {} files dropped", dropped.paths().len());
                cx.emit(super::Attachment::Files(dropped.paths().to_vec()));
            }));
        self.center_current(cx);
        // Typewriter: half a view past each end.
        self.make_room();
        let blocks = list(self.list.clone(), cx.processor(Self::item)).size_full();
        // The text column: its measure, font, centered.
        let column = |content: gpui::AnyElement, look: &super::DocumentLook| {
            div().size_full().flex().justify_center().child(
                div()
                    .h_full()
                    .w_full()
                    .when_some(look.measure, |column, measure| column.max_w(measure))
                    .when_some(look.font.clone(), |column, font| column.font_family(font))
                    .child(content),
            )
        };
        let blocks = column(blocks.into_any_element(), &self.look);
        let source = column(self.editor.clone().into_any_element(), &self.look);
        match self.mode {
            MarkdownMode::Source => root
                .capture_action(cx.listener(|document, _: &crate::editor::Paste, _, cx| {
                    if document.paste_picture(cx) {
                        cx.stop_propagation();
                    }
                }))
                .capture_action(cx.listener(|document, _: &crate::editor::Newline, _, cx| {
                    if document.source_newline(cx) {
                        cx.stop_propagation();
                    }
                }))
                .child(source)
                .into_any_element(),
            MarkdownMode::Split => root
                .flex()
                .child(div().flex_1().min_w_0().h_full().child(self.editor.clone()))
                .child(div().flex_1().min_w_0().h_full().child(blocks))
                .into_any_element(),
            MarkdownMode::Preview => root
                .track_focus(&self.focus)
                .px_2()
                .py_3()
                .child(blocks)
                .into_any_element(),
            MarkdownMode::Visual => root
                .key_context(CONTEXT)
                .track_focus(&self.focus)
                .on_action(cx.listener(|document, _: &Undo, window, cx| {
                    document.history(false, window, cx);
                }))
                .on_action(cx.listener(|document, _: &Redo, window, cx| {
                    document.history(true, window, cx);
                }))
                // One history: a block's own never runs.
                .capture_action(cx.listener(|document, _: &forms::Undo, window, cx| {
                    cx.stop_propagation();
                    document.history(false, window, cx);
                }))
                .capture_action(cx.listener(|document, _: &forms::Redo, window, cx| {
                    cx.stop_propagation();
                    document.history(true, window, cx);
                }))
                .capture_action(cx.listener(|document, _: &forms::Paste, _, cx| {
                    if document.paste_picture(cx) {
                        cx.stop_propagation();
                    }
                }))
                .on_action(cx.listener(|document, _: &Paste, _, cx| {
                    if document.paste_picture(cx) {
                        return;
                    }
                    match cx.read_from_clipboard().and_then(|item| item.text()) {
                        Some(text) => _ = document.insert(&text, cx),
                        None => log::info!("markdown document: nothing to paste"),
                    }
                }))
                .capture_action(cx.listener(|document, _: &forms::Enter, _, cx| {
                    if document.block_newline(cx) {
                        cx.stop_propagation();
                    }
                }))
                .on_action(cx.listener(|document, _: &Indent, window, cx| {
                    document.block_shift(true, window, cx);
                }))
                .on_action(cx.listener(|document, _: &Outdent, window, cx| {
                    document.block_shift(false, window, cx);
                }))
                .capture_action(cx.listener(|document, _: &Up, window, cx| {
                    if document.step(false, window, cx) {
                        cx.stop_propagation();
                    }
                }))
                .capture_action(cx.listener(|document, _: &Down, window, cx| {
                    if document.step(true, window, cx) {
                        cx.stop_propagation();
                    }
                }))
                .on_key_down(
                    cx.listener(|document, event: &gpui::KeyDownEvent, window, cx| {
                        if event.keystroke.key == "escape" && document.open.is_some() {
                            cx.stop_propagation();
                            document.commit(cx);
                            window.focus(&document.focus, cx);
                        }
                    }),
                )
                .px_2()
                .py_3()
                .child(blocks)
                .into_any_element(),
        }
    }
}
