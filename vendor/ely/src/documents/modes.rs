use std::{ops::Range, rc::Rc};

use gpui::{
    AnyElement, App, AppContext as _, ElementId, Entity, Focusable, InteractiveElement,
    IntoElement, MouseButton, ParentElement, RenderOnce, SharedString, Styled, Subscription,
    Window, div, prelude::*,
};

use super::{
    counts::{ReadingTime, WordCount},
    markdown::{block_ranges, markdown_highlights},
    render::MarkdownRenderer,
    rich::RichTextEditor,
    suggest::offers,
    toolbar::FixedFormatToolbar,
};
use crate::{
    buttons::SegmentedControl,
    forms::{Down, InputEvent, TextInput, Up},
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
};

/// How a markdown editor shows its text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkdownMode {
    /// The document as read; a pressed block opens as source.
    Visual,
    /// The markdown, styled as it is typed.
    Source,
    /// The source beside the document it makes.
    Split,
    /// The document alone, read only.
    Preview,
}

impl MarkdownMode {
    const ALL: [(Self, &'static str); 4] = [
        (Self::Visual, "Visual"),
        (Self::Source, "Source"),
        (Self::Split, "Split"),
        (Self::Preview, "Preview"),
    ];

    fn name(self) -> &'static str {
        Self::ALL
            .iter()
            .find(|(mode, _)| *mode == self)
            .map(|(_, name)| *name)
            .expect("every mode is named")
    }
}

type OnMode = Rc<dyn Fn(MarkdownMode, &mut Window, &mut App)>;

/// The open block: its document, range, field, and appending.
struct Open {
    document: String,
    range: Range<usize>,
    input: Entity<TextInput>,
    append: bool,
    _blur: Subscription,
}

#[derive(Default)]
struct Visual {
    open: Option<Open>,
}

/// `range` shifted by `written` bytes written over `replaced`.
fn shifted(range: Range<usize>, replaced: &Range<usize>, written: usize) -> Range<usize> {
    if range.start < replaced.end {
        return range;
    }
    let (start, end) = (
        range.start - replaced.len() + written,
        range.end - replaced.len() + written,
    );
    start..end
}

/// What joins an appended block to the text before.
pub(crate) fn joint(document: &str) -> &'static str {
    if document.is_empty() || document.ends_with("\n\n") {
        ""
    } else if document.ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    }
}

/// Writes the open block back as one step; the edit.
fn commit(
    state: &Entity<Visual>,
    field: &Entity<TextInput>,
    cx: &mut App,
) -> Option<(Range<usize>, usize)> {
    let open = state.update(cx, |visual, cx| {
        cx.notify();
        visual.open.take()
    })?;
    let written = open.input.read(cx).text().to_string();
    let document = field.read(cx).text().to_string();
    if document != open.document {
        log::error!(
            "markdown editor: the text changed under block {:?}; its edit is dropped",
            open.range
        );
        return None;
    }
    let written = match open.append {
        true if written.is_empty() => return Some((open.range, 0)),
        true => format!("{}{written}", joint(&document)),
        false => written,
    };
    if document[open.range.clone()] != written {
        log::info!("markdown editor: block {:?} written back", open.range);
        field.update(cx, |document, cx| {
            document.select(open.range.clone(), cx);
            document.insert(&written, cx);
        });
    }
    Some((open.range, written.len()))
}

/// Opens the block at `range` in its own field.
fn open(
    state: &Entity<Visual>,
    field: &Entity<TextInput>,
    range: Range<usize>,
    append: bool,
    at_start: bool,
    window: &mut Window,
    cx: &mut App,
) {
    let document = field.read(cx).text().to_string();
    let source = document[range.clone()].to_string();
    log::info!("markdown editor: block {range:?} opens");
    let input = cx.new(|cx| {
        TextInput::new(window, cx)
            .multi_line(1, usize::MAX)
            .highlighter(markdown_highlights)
    });
    input.update(cx, |input, cx| {
        input.set_text(source, cx);
        if at_start {
            input.select(0..0, cx);
        }
    });
    window.focus(&input.focus_handle(cx), cx);
    let blur = {
        let (state, field) = (state.clone(), field.clone());
        cx.subscribe(&input, move |_, event: &InputEvent, cx| {
            if *event == InputEvent::Blur {
                commit(&state, &field, cx);
            }
        })
    };
    state.update(cx, |visual, cx| {
        visual.open = Some(Open {
            document,
            range,
            input,
            append,
            _blur: blur,
        });
        cx.notify();
    });
}

/// The caret on the first line, or with `last` last.
pub(crate) fn at_edge(input: &TextInput, last: bool) -> bool {
    let edge = if last { input.text().len() } else { 0 };
    match (input.bounds_for(input.cursor()), input.bounds_for(edge)) {
        (Some(caret), Some(edge)) => caret.top() == edge.top(),
        _ => false,
    }
}

/// Markdown written as visual, source, or split views.
#[derive(IntoElement)]
pub struct MarkdownEditor {
    id: ElementId,
    field: Entity<TextInput>,
    mode: MarkdownMode,
    people: Vec<SharedString>,
    on_mode: Option<OnMode>,
}

impl MarkdownEditor {
    /// Give the field several lines and `markdown_highlights` as its highlighter.
    pub fn new(id: impl Into<ElementId>, field: &Entity<TextInput>, mode: MarkdownMode) -> Self {
        Self {
            id: id.into(),
            field: field.clone(),
            mode,
            people: Vec::new(),
            on_mode: None,
        }
    }

    /// Handles `@` suggests.
    pub fn people(mut self, handles: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.people = handles.into_iter().map(Into::into).collect();
        self
    }

    pub fn on_mode(
        mut self,
        handler: impl Fn(MarkdownMode, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_mode = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for MarkdownEditor {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state =
            window.use_keyed_state((self.id.clone(), "visual"), cx, |_, _| Visual::default());
        let switch = {
            let (state, field, on_mode) = (state.clone(), self.field.clone(), self.on_mode.clone());
            MarkdownMode::ALL
                .iter()
                .fold(
                    SegmentedControl::new((self.id.clone(), "mode"), self.mode.name())
                        .size(ControlSize::Sm),
                    |switch, (_, name)| switch.segment(*name, *name, None),
                )
                .on_change(move |name, window, cx| {
                    let (mode, _) = MarkdownMode::ALL
                        .into_iter()
                        .find(|(_, named)| *named == name.as_ref())
                        .expect("a segment names a mode");
                    commit(&state, &field, cx);
                    log::info!("markdown editor: {name}");
                    if let Some(on_mode) = &on_mode {
                        on_mode(mode, window, cx);
                    }
                })
        };
        match self.mode {
            MarkdownMode::Source => RichTextEditor::new(self.id, &self.field)
                .people(self.people)
                .toolbar_end(switch)
                .into_any_element(),
            MarkdownMode::Split => {
                let text = SharedString::from(self.field.read(cx).text().to_string());
                RichTextEditor::new(self.id.clone(), &self.field)
                    .people(self.people)
                    .toolbar_end(switch)
                    .beside(MarkdownRenderer::new((self.id, "preview"), text))
                    .into_any_element()
            }
            MarkdownMode::Visual => visual(self, state, switch, window, cx),
            MarkdownMode::Preview => {
                let text = SharedString::from(self.field.read(cx).text().to_string());
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().flex().justify_end().child(switch))
                    .child(MarkdownRenderer::new((self.id, "preview"), text))
                    .into_any_element()
            }
        }
    }
}

/// The visual body: blocks drawn, the open one as source.
fn visual(
    editor: MarkdownEditor,
    state: Entity<Visual>,
    switch: SegmentedControl,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let (colors, radius, frame, bar, tiny) = {
        let theme = cx.theme();
        (
            theme.colors.clone(),
            theme.radius(Radius::Md),
            theme.radius(Radius::Lg),
            theme.control_height(ControlSize::Lg),
            theme.text_size(TextSize::Xs),
        )
    };
    let document = SharedString::from(editor.field.read(cx).text().to_string());
    let blocks = block_ranges(&document);
    let opened = state
        .read(cx)
        .open
        .as_ref()
        .map(|open| (open.range.clone(), open.input.clone(), open.append));
    let field = editor.field.clone();
    let source = |input: &Entity<TextInput>, window: &mut Window, cx: &mut App| {
        div()
            .px_3()
            .py_2()
            .rounded(radius)
            .bg(colors.sunken)
            .child(
                offers(
                    (editor.id.clone(), "offers").into(),
                    input,
                    &editor.people,
                    None,
                    cx,
                )
                .wrap(input.clone(), window, cx),
            )
            .into_any_element()
    };
    let mut rows: Vec<AnyElement> = Vec::new();
    for (ix, range) in blocks.iter().enumerate() {
        if let Some((open_range, input, false)) = &opened
            && open_range == range
        {
            rows.push(source(input, window, cx));
            continue;
        }
        let (state, field, pressed, painted) = (
            state.clone(),
            field.clone(),
            range.clone(),
            document.clone(),
        );
        rows.push(
            div()
                .id((editor.id.clone(), format!("block-{ix}")))
                .px_3()
                .py_1()
                .rounded(radius)
                .cursor_text()
                .hover(|block| block.bg(colors.hover.opacity(0.5)))
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    window.prevent_default();
                    cx.stop_propagation();
                    if field.read(cx).text() != painted.as_ref() {
                        log::error!(
                            "markdown editor: block {pressed:?} left with the text; press dropped"
                        );
                        return;
                    }
                    let range = match commit(&state, &field, cx) {
                        Some((replaced, written)) => shifted(pressed.clone(), &replaced, written),
                        None => pressed.clone(),
                    };
                    open(&state, &field, range, false, false, window, cx);
                })
                .child(MarkdownRenderer::new(
                    (editor.id.clone(), format!("render-{ix}")),
                    document[range.clone()].to_string(),
                ))
                .into_any_element(),
        );
    }
    if let Some((_, input, true)) = &opened {
        rows.push(source(input, window, cx));
    }
    let (more_state, more_field) = (state.clone(), field.clone());
    let more = div()
        .id((editor.id.clone(), "more"))
        .px_3()
        .py_2()
        .cursor_text()
        .text_color(colors.fg_subtle)
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            window.prevent_default();
            commit(&more_state, &more_field, cx);
            let end = more_field.read(cx).text().len();
            open(&more_state, &more_field, end..end, true, false, window, cx);
        })
        .when(document.is_empty() && opened.is_none(), |more| {
            more.child("Start writing")
        })
        .when(!document.is_empty(), |more| more.min_h_8());
    let open_input = opened.as_ref().map(|(_, input, _)| input.clone());
    let edges = |last: bool| {
        let (state, field) = (state.clone(), field.clone());
        move |window: &mut Window, cx: &mut App| -> bool {
            let Some((range, input)) = state
                .read(cx)
                .open
                .as_ref()
                .map(|open| (open.range.clone(), open.input.clone()))
            else {
                return false;
            };
            if !at_edge(input.read(cx), last) {
                return false;
            }
            let blocks = block_ranges(field.read(cx).text());
            let target = if last {
                blocks.into_iter().find(|block| block.start >= range.end)
            } else {
                blocks
                    .into_iter()
                    .rev()
                    .find(|block| block.end <= range.start)
            };
            let Some(target) = target else { return false };
            let target = match commit(&state, &field, cx) {
                Some((replaced, written)) => shifted(target, &replaced, written),
                None => target,
            };
            open(&state, &field, target, false, last, window, cx);
            true
        }
    };
    let (up, down) = (edges(false), edges(true));
    let (escape_state, escape_field) = (state.clone(), field.clone());
    let text = document;
    div()
        .id(editor.id.clone())
        .flex()
        .flex_col()
        .rounded(frame)
        .border_1()
        .border_color(colors.border)
        .bg(colors.surface)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .h(bar)
                .px_2()
                .border_b_1()
                .border_color(colors.border)
                .child(
                    div().children(open_input.map(|input| {
                        FixedFormatToolbar::new((editor.id.clone(), "tools"), &input)
                    })),
                )
                .child(switch),
        )
        .child(
            div()
                .id((editor.id.clone(), "blocks"))
                .flex()
                .flex_col()
                .gap_1()
                .px_2()
                .py_3()
                .capture_action(move |_: &Up, window, cx| {
                    if up(window, cx) {
                        cx.stop_propagation();
                    }
                })
                .capture_action(move |_: &Down, window, cx| {
                    if down(window, cx) {
                        cx.stop_propagation();
                    }
                })
                .on_key_down(move |event, _, cx| {
                    if event.keystroke.key == "escape" && escape_state.read(cx).open.is_some() {
                        cx.stop_propagation();
                        commit(&escape_state, &escape_field, cx);
                    }
                })
                .children(rows)
                .child(more),
        )
        .child(
            div()
                .flex()
                .justify_between()
                .px_4()
                .py_2()
                .border_t_1()
                .border_color(colors.border)
                .text_size(tiny)
                .child(WordCount::new(text.clone()))
                .child(ReadingTime::new(text)),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests;
