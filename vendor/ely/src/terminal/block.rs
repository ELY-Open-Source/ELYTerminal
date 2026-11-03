use std::{rc::Rc, time::Duration};

use gpui::{
    App, ClipboardItem, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use super::ansi::{AnsiText, printed};
use crate::{
    buttons::{ButtonVariant, IconButton},
    motion::Spinner,
    primitives::{Disclosure, Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
    typography::{DurationStyle, format},
};

/// Where a command stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandState {
    Running,
    Done { code: i32, took: Duration },
}

type Run = Rc<dyn Fn(&mut Window, &mut App)>;
type OnFold = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// One command and what it printed, as a block: where it ran, how it ended and how long it took. Its output folds away; the command and output copy, and it runs again.
#[derive(IntoElement)]
pub struct CommandBlock {
    id: ElementId,
    cwd: SharedString,
    command: SharedString,
    output: SharedString,
    state: CommandState,
    folded: bool,
    on_fold: Option<OnFold>,
    on_rerun: Option<Run>,
}

impl CommandBlock {
    /// `output` as printed, escape codes and all.
    pub fn new(
        id: impl Into<ElementId>,
        cwd: impl Into<SharedString>,
        command: impl Into<SharedString>,
        output: impl Into<SharedString>,
        state: CommandState,
    ) -> Self {
        Self {
            id: id.into(),
            cwd: cwd.into(),
            command: command.into(),
            output: output.into(),
            state,
            folded: false,
            on_fold: None,
            on_rerun: None,
        }
    }

    pub fn folded(mut self, folded: bool) -> Self {
        self.folded = folded;
        self
    }

    pub fn on_fold(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_fold = Some(Rc::new(handler));
        self
    }

    pub fn on_rerun(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_rerun = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for CommandBlock {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let (mark, outcome): (gpui::AnyElement, SharedString) = match self.state {
            CommandState::Running => (
                Spinner::new((self.id.clone(), "spinner"))
                    .size(IconSize::Sm)
                    .into_any_element(),
                "Running".into(),
            ),
            CommandState::Done { code: 0, took } => (
                Icon::new(IconName::CircleCheck)
                    .size(IconSize::Sm)
                    .color(colors.success)
                    .into_any_element(),
                format::duration(took.as_secs().max(1), DurationStyle::Compact).into(),
            ),
            CommandState::Done { code, took } => (
                Icon::new(IconName::CircleX)
                    .size(IconSize::Sm)
                    .color(colors.danger)
                    .into_any_element(),
                format!(
                    "Exit {code} · {}",
                    format::duration(took.as_secs().max(1), DurationStyle::Compact)
                )
                .into(),
            ),
        };
        let copy = |key: &'static str, icon, words: &'static str, text: String| {
            IconButton::new((self.id.clone(), key), icon)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip(words)
                .on_click(move |_, _, cx| {
                    log::info!("command block: {words}");
                    cx.write_to_clipboard(ClipboardItem::new_string(text.clone()))
                })
        };
        let plain = printed(&self.output).0;
        let (fold, folded) = (self.on_fold, self.folded);
        let has_output = !plain.trim().is_empty();
        div()
            .id(self.id.clone())
            .flex()
            .flex_col()
            .gap_1()
            .py_2()
            .pl_3()
            .pr_2()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(mark)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_baseline()
                            .gap_2()
                            .child(
                                div()
                                    .font_family(theme.mono_family.clone())
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(colors.fg)
                                    .child(self.command.clone()),
                            )
                            .child(div().text_color(colors.fg_subtle).child(self.cwd)),
                    )
                    .child(div().text_color(colors.fg_muted).child(outcome))
                    .child(copy(
                        "copy-command",
                        IconName::Copy,
                        "Copy command",
                        self.command.to_string(),
                    ))
                    .child(copy(
                        "copy-output",
                        IconName::FileText,
                        "Copy output",
                        plain,
                    ))
                    .children(self.on_rerun.map(|rerun| {
                        IconButton::new((self.id.clone(), "rerun"), IconName::RotateCcw)
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Sm)
                            .tooltip("Run again")
                            .on_click(move |_, window, cx| rerun(window, cx))
                    }))
                    .when(has_output, |header| {
                        header.child(
                            div()
                                .id((self.id.clone(), "fold"))
                                .cursor_pointer()
                                .when_some(fold, |toggle, fold| {
                                    toggle.on_click(move |_, window, cx| fold(!folded, window, cx))
                                })
                                .child(Disclosure::new((self.id.clone(), "chevron"), !folded)),
                        )
                    }),
            )
            .when(has_output && !folded, |block| {
                block.child(div().pl_6().child(AnsiText::new(self.output)))
            })
    }
}
