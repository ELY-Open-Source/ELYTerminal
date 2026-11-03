use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, prelude::*,
};

use super::transfer::TransferState;
use crate::{
    buttons::{Button, ButtonVariant},
    forms::Run,
    motion::ProgressBar,
    primitives::{Icon, IconName, Severity},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
    typography::{DurationStyle, Ellipsis, MiddleEllipsis, format},
};

/// What an operation does to files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileOperation {
    Copy,
    Move,
}

/// What an operation says of itself, by what it does and where it stands.
pub(crate) fn headline(
    operation: FileOperation,
    items: usize,
    to: &str,
    state: &TransferState,
) -> String {
    let things = match items {
        1 => "1 item".to_string(),
        count => format!("{count} items"),
    };
    let (doing, done) = match operation {
        FileOperation::Copy => ("Copying", "Copied"),
        FileOperation::Move => ("Moving", "Moved"),
    };
    match state {
        TransferState::Queued => format!("{doing} {things} to {to} next"),
        TransferState::Moving { .. } => format!("{doing} {things} to {to}"),
        TransferState::Paused => format!("{doing} {things} to {to}, paused"),
        TransferState::Done => format!("{done} {things} to {to}"),
        TransferState::Failed(_) => format!("{doing} {things} to {to} stopped"),
    }
}

/// A copy or a move under way: what it does and where to, a bar, the bytes done of all with the time left, and the file it takes now. Pause or resume and cancel while it runs; a failure says why in the danger tone.
#[derive(IntoElement)]
pub struct FileOperationProgress {
    id: ElementId,
    operation: FileOperation,
    items: usize,
    to: SharedString,
    moved: u64,
    total: u64,
    state: TransferState,
    current: Option<SharedString>,
    on_pause: Option<Run>,
    on_resume: Option<Run>,
    on_cancel: Option<Run>,
}

impl FileOperationProgress {
    /// `items` things to `to`, `moved` bytes of `total` so far.
    pub fn new(
        id: impl Into<ElementId>,
        operation: FileOperation,
        items: usize,
        to: impl Into<SharedString>,
        (moved, total): (u64, u64),
        state: TransferState,
    ) -> Self {
        assert!(items > 0, "no items, {moved} of {total} bytes");
        if moved > total {
            log::error!("file operation: {moved} of {total} bytes; the bar fills");
        }
        Self {
            id: id.into(),
            operation,
            items,
            to: to.into(),
            moved,
            total,
            state,
            current: None,
            on_pause: None,
            on_resume: None,
            on_cancel: None,
        }
    }

    /// The file it takes now.
    pub fn current(mut self, name: impl Into<SharedString>) -> Self {
        self.current = Some(name.into());
        self
    }

    pub fn on_pause(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_pause = Some(Rc::new(handler));
        self
    }

    pub fn on_resume(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_resume = Some(Rc::new(handler));
        self
    }

    pub fn on_cancel(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_cancel = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for FileOperationProgress {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let small = theme.text_size(TextSize::Sm);
        let share = if self.total == 0 {
            1.0
        } else {
            self.moved as f32 / self.total as f32
        };
        let (moved, total) = (
            format::file_size(self.moved, false),
            format::file_size(self.total, false),
        );
        let detail = match &self.state {
            TransferState::Moving { rate } if *rate > 0 => {
                let left = self.total.saturating_sub(self.moved) / rate;
                format!(
                    "{moved} of {total} · {} left",
                    format::duration(left.max(1), DurationStyle::Compact)
                )
            }
            TransferState::Failed(reason) => reason.to_string(),
            TransferState::Done => total,
            _ => format!("{moved} of {total}"),
        };
        let button = |key: &'static str, label: &'static str, run: Option<Run>| {
            run.map(|run| {
                Button::new((self.id.clone(), key), label)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .on_click(move |_, window, cx| {
                        log::info!("file operation: {key}");
                        run(window, cx)
                    })
            })
        };
        let (pause, resume, cancel) = match self.state {
            TransferState::Moving { .. } => (self.on_pause, None, self.on_cancel),
            TransferState::Paused => (None, self.on_resume, self.on_cancel),
            TransferState::Queued => (None, None, self.on_cancel),
            _ => (None, None, None),
        };
        let failed = matches!(self.state, TransferState::Failed(_));
        let icon = match (&self.state, self.operation) {
            (TransferState::Failed(_), _) => Severity::Danger.icon(),
            (TransferState::Done, _) => Severity::Success.icon(),
            (_, FileOperation::Copy) => IconName::Copy,
            (_, FileOperation::Move) => IconName::FolderOpen,
        };
        let tone = match &self.state {
            TransferState::Failed(_) => Severity::Danger.color(colors),
            TransferState::Done => Severity::Success.color(colors),
            _ => colors.fg_muted,
        };
        let running = matches!(
            self.state,
            TransferState::Moving { .. } | TransferState::Paused
        );
        div()
            .debug_selector(|| "file-operation".into())
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w(theme.label_width())
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(Icon::new(icon).size(IconSize::Lg).color(tone))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(colors.fg)
                                    .child(Ellipsis::new(headline(
                                        self.operation,
                                        self.items,
                                        &self.to,
                                        &self.state,
                                    ))),
                            ),
                    )
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .children(button("pause", "Pause", pause))
                            .children(button("resume", "Resume", resume))
                            .children(button("cancel", "Cancel", cancel)),
                    ),
            )
            .when(running, |card| {
                card.child(ProgressBar::new((self.id.clone(), "bar"), share.min(1.0)))
            })
            .child(
                div()
                    .text_size(small)
                    .text_color(if failed {
                        colors.danger
                    } else {
                        colors.fg_muted
                    })
                    .child(Ellipsis::new(detail)),
            )
            .children(self.current.filter(|_| running).map(|name| {
                div()
                    .text_size(small)
                    .text_color(colors.fg_subtle)
                    .child(MiddleEllipsis::new(name))
            }))
    }
}
