use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use super::FileIcon;
use crate::{
    buttons::{ButtonVariant, IconButton},
    motion::ProgressBar,
    primitives::{FocusRing, Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
    typography::{DurationStyle, Ellipsis, MiddleEllipsis, format},
};

pub(super) type OnKey = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// Where a transfer stands.
#[derive(Clone, Debug, PartialEq)]
pub enum TransferState {
    /// Waiting its turn.
    Queued,
    /// Moving `rate` bytes a second.
    Moving {
        rate: u64,
    },
    Paused,
    Done,
    /// Stopped, and why.
    Failed(SharedString),
}

/// One file on its way: its key, its name, whether it goes up, the bytes moved of all, and where it stands.
#[derive(Clone, Debug, PartialEq)]
pub struct Transfer {
    pub key: SharedString,
    pub name: SharedString,
    pub upload: bool,
    pub moved: u64,
    pub total: u64,
    pub state: TransferState,
}

/// A transfer's line under its name: how far and how fast, or where it stopped.
pub(crate) fn status(transfer: &Transfer) -> String {
    let (moved, total) = (
        format::file_size(transfer.moved, false),
        format::file_size(transfer.total, false),
    );
    match &transfer.state {
        TransferState::Queued => format!("Waiting · {total}"),
        TransferState::Moving { rate } if *rate > 0 => {
            let left = transfer.total.saturating_sub(transfer.moved) / rate;
            format!(
                "{moved} of {total} · {}/s · {} left",
                format::file_size(*rate, false),
                format::duration(left.max(1), DurationStyle::Compact)
            )
        }
        TransferState::Moving { .. } => format!("{moved} of {total}"),
        TransferState::Paused => format!("Paused at {moved} of {total}"),
        TransferState::Done => total,
        TransferState::Failed(reason) => format!("Failed: {reason}"),
    }
}

/// What a row can ask of its host, each shown only with its handler.
#[derive(Clone, Default)]
pub(super) struct Asks {
    pub pause: Option<OnKey>,
    pub resume: Option<OnKey>,
    pub retry: Option<OnKey>,
    pub cancel: Option<OnKey>,
    pub open: Option<OnKey>,
    pub show: Option<OnKey>,
}

/// A transfer as a row: its file's icon and name, a bar while it moves or waits paused, its status, and the actions its state allows: pause or resume, retry after a failure, cancel until done; a finished one opens on a press and shows its folder.
pub(super) fn row(id: ElementId, transfer: &Transfer, asks: &Asks, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let colors = &theme.colors;
    let key = transfer.key.clone();
    let action = |name: &'static str, icon: IconName, tip: &'static str, ask: &Option<OnKey>| {
        ask.clone().map(|ask| {
            let key = key.clone();
            IconButton::new((id.clone(), name), icon)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip(tip)
                .on_click(move |_, window, cx| {
                    log::info!("transfer {key}: {name}");
                    ask(&key, window, cx)
                })
        })
    };
    let actions: Vec<_> = match transfer.state {
        TransferState::Moving { .. } => vec![
            action("pause", IconName::Pause, "Pause", &asks.pause),
            action("cancel", IconName::X, "Cancel", &asks.cancel),
        ],
        TransferState::Paused => vec![
            action("resume", IconName::Play, "Resume", &asks.resume),
            action("cancel", IconName::X, "Cancel", &asks.cancel),
        ],
        TransferState::Queued => vec![action("cancel", IconName::X, "Cancel", &asks.cancel)],
        TransferState::Failed(_) => vec![
            action("retry", IconName::RotateCw, "Retry", &asks.retry),
            action("cancel", IconName::X, "Remove", &asks.cancel),
        ],
        TransferState::Done => vec![action(
            "show",
            IconName::FolderOpen,
            "Show in folder",
            &asks.show,
        )],
    };
    let bar = matches!(
        transfer.state,
        TransferState::Moving { .. } | TransferState::Paused
    )
    .then(|| {
        let share = if transfer.total == 0 {
            0.0
        } else {
            (transfer.moved as f64 / transfer.total as f64).min(1.0) as f32
        };
        ProgressBar::new((id.clone(), "bar"), share)
    });
    let failed = matches!(transfer.state, TransferState::Failed(_));
    let way = if transfer.upload {
        IconName::Upload
    } else {
        IconName::Download
    };
    let opens = asks
        .open
        .clone()
        .filter(|_| transfer.state == TransferState::Done);
    let name = transfer.key.clone();
    div()
        .id(id.clone())
        .debug_selector(move || format!("transfer-row {name}"))
        .w_full()
        .flex()
        .items_center()
        .gap_3()
        .px_3()
        .py_2()
        .rounded(theme.radius(Radius::Md))
        .border_1()
        .border_color(gpui::transparent_black())
        .when_some(opens, |row, open| {
            let key = key.clone();
            row.cursor_pointer()
                .tab_index(0)
                .focus_ring(cx)
                .hover(|style| style.bg(colors.hover))
                .on_click(move |_, window, cx| {
                    log::info!("transfer {key}: open");
                    open(&key, window, cx)
                })
        })
        .child(
            div()
                .flex_none()
                .child(FileIcon::file(&transfer.name).size(IconSize::Lg)),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1p5()
                        .child(Icon::new(way).size(IconSize::Xs).color(colors.fg_subtle))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_color(colors.fg)
                                .child(MiddleEllipsis::new(transfer.name.clone())),
                        ),
                )
                .children(bar)
                .child(
                    div()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(if failed {
                            colors.danger
                        } else {
                            colors.fg_muted
                        })
                        .child(Ellipsis::new(status(transfer))),
                ),
        )
        .child(
            div()
                .debug_selector(move || format!("transfer-actions {key}"))
                .flex_none()
                .flex()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .children(actions.into_iter().flatten()),
        )
        .into_any_element()
}
