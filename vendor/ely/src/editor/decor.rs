use std::{ops::Range, rc::Rc};

use gpui::{
    App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div,
};

use crate::{
    buttons::{Button, ButtonVariant},
    primitives::{Icon, IconName, Severity},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
};

/// A problem in a span of code: squiggled under it and marked in the gutter.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub range: Range<usize>,
    pub severity: Severity,
    pub message: SharedString,
}

/// A note set into the code at an offset, such as an inferred type or a parameter name.
#[derive(Clone, Debug, PartialEq)]
pub struct InlayHint {
    pub offset: usize,
    pub text: SharedString,
}

/// Actions over a line, such as "3 references" or "Run test", in a quiet row above it.
#[derive(Clone, Debug, PartialEq)]
pub struct CodeLens {
    pub line: usize,
    pub items: Vec<SharedString>,
}

/// A suggestion shown in grey at an offset; its first line continues the line, the rest follow below.
#[derive(Clone, Debug, PartialEq)]
pub struct GhostText {
    pub offset: usize,
    pub text: SharedString,
}

/// A change against the saved text: lines added from `line` on, and the lines they replaced, shown above them.
#[derive(Clone, Debug, PartialEq)]
pub struct DiffHunk {
    pub line: usize,
    pub added: usize,
    pub removed: Vec<SharedString>,
}

/// A line's state against version control.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitMark {
    Added,
    Modified,
    /// Lines were removed just below this one.
    Deleted,
}

type OnUnlock = Rc<dyn Fn(&mut Window, &mut App)>;

/// A strip above code that cannot be changed here, why, and a way to change it anyway.
#[derive(IntoElement)]
pub struct ReadOnlyBanner {
    id: ElementId,
    message: SharedString,
    action: Option<(SharedString, OnUnlock)>,
}

impl ReadOnlyBanner {
    pub fn new(id: impl Into<ElementId>, message: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            message: message.into(),
            action: None,
        }
    }

    pub fn action(
        mut self,
        label: impl Into<SharedString>,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.action = Some((label.into(), Rc::new(handler)));
        self
    }
}

impl RenderOnce for ReadOnlyBanner {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        div()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_1p5()
            .bg(colors.warning.opacity(0.1))
            .border_b_1()
            .border_color(colors.warning.opacity(0.3))
            .font_family(theme.font_family.clone())
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(colors.fg)
            .child(
                Icon::new(IconName::Lock)
                    .size(IconSize::Sm)
                    .color(colors.warning),
            )
            .child(div().flex_1().child(self.message))
            .children(self.action.map(|(label, handler)| {
                Button::new(self.id, label)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .on_click(move |_, window, cx| handler(window, cx))
            }))
    }
}

/// Which mode a modal editor is in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VimMode {
    #[default]
    Normal,
    Insert,
    Visual,
    Replace,
}

impl VimMode {
    fn word(self) -> &'static str {
        match self {
            VimMode::Normal => "NORMAL",
            VimMode::Insert => "INSERT",
            VimMode::Visual => "VISUAL",
            VimMode::Replace => "REPLACE",
        }
    }
}

/// The modal editing mode as a small tag, tinted by mode, and a pending command after it.
#[derive(IntoElement)]
pub struct VimModeIndicator {
    mode: VimMode,
    pending: Option<SharedString>,
}

impl VimModeIndicator {
    pub fn new(mode: VimMode) -> Self {
        Self {
            mode,
            pending: None,
        }
    }

    /// Keys typed toward a command, such as `d2`.
    pub fn pending(mut self, keys: impl Into<SharedString>) -> Self {
        self.pending = Some(keys.into());
        self
    }
}

impl RenderOnce for VimModeIndicator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let ink = match self.mode {
            VimMode::Normal => colors.fg_muted,
            VimMode::Insert => colors.success,
            VimMode::Visual => colors.warning,
            VimMode::Replace => colors.danger,
        };
        div()
            .flex()
            .items_center()
            .gap_1p5()
            .text_size(theme.text_size(TextSize::Xs))
            .child(
                div()
                    .px_1p5()
                    .rounded(theme.radius(Radius::Sm))
                    .bg(ink.opacity(0.14))
                    .text_color(ink)
                    .font_weight(FontWeight::SEMIBOLD)
                    .font_family(theme.mono_family.clone())
                    .child(self.mode.word()),
            )
            .children(self.pending.map(|keys| {
                div()
                    .font_family(theme.mono_family.clone())
                    .text_color(colors.fg_muted)
                    .child(keys)
            }))
    }
}
