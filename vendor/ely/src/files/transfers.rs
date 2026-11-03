use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, prelude::*,
};

use super::transfer::{Asks, OnKey, Transfer, TransferState, row};
use crate::{
    buttons::{Button, ButtonVariant},
    forms::Run,
    theme::{ActiveTheme, ControlSize, TextSize},
};

/// Keys asserted once a list.
fn unique(transfers: &[Transfer]) {
    for (ix, transfer) in transfers.iter().enumerate() {
        assert!(
            !transfers[..ix]
                .iter()
                .any(|other| other.key == transfer.key),
            "transfer {} twice",
            transfer.key
        );
    }
}

/// How many move and how many wait, as a header says it.
pub(crate) fn tally(transfers: &[Transfer]) -> String {
    let moving = transfers
        .iter()
        .filter(|each| matches!(each.state, TransferState::Moving { .. }))
        .count();
    let waiting = transfers
        .iter()
        .filter(|each| matches!(each.state, TransferState::Queued | TransferState::Paused))
        .count();
    format!("{moving} moving · {waiting} waiting")
}

macro_rules! asks {
    ($name:ident) => {
        pub fn $name(
            mut self,
            handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
        ) -> Self {
            self.asks.$name = Some(Rc::new(handler) as OnKey);
            self
        }
    };
}

/// Uploads and downloads in one queue, a row a file: which way it goes, its bar, its status, and pause, resume, retry or cancel. How many move and wait sits above.
#[derive(IntoElement)]
pub struct TransferQueue {
    id: ElementId,
    transfers: Vec<Transfer>,
    asks: Asks,
}

impl TransferQueue {
    pub fn new(id: impl Into<ElementId>, transfers: impl IntoIterator<Item = Transfer>) -> Self {
        let transfers: Vec<Transfer> = transfers.into_iter().collect();
        unique(&transfers);
        Self {
            id: id.into(),
            transfers,
            asks: Asks::default(),
        }
    }

    asks!(pause);
    asks!(resume);
    asks!(retry);
    asks!(cancel);
}

impl RenderOnce for TransferQueue {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .debug_selector(|| "transfer-queue".into())
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .px_3()
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(theme.colors.fg_muted)
                    .child(tally(&self.transfers)),
            )
            .children(self.transfers.iter().map(|transfer| {
                row(
                    (self.id.clone(), format!("transfer-{}", transfer.key)).into(),
                    transfer,
                    &self.asks,
                    cx,
                )
            }))
    }
}

/// Downloads as a browser keeps them: those on their way above, those finished below under a header with Clear. A finished one opens with a press and shows its folder; the rest pause, resume, retry or cancel.
#[derive(IntoElement)]
pub struct DownloadManager {
    id: ElementId,
    downloads: Vec<Transfer>,
    asks: Asks,
    on_clear: Option<Run>,
}

impl DownloadManager {
    pub fn new(id: impl Into<ElementId>, downloads: impl IntoIterator<Item = Transfer>) -> Self {
        let downloads: Vec<Transfer> = downloads.into_iter().collect();
        unique(&downloads);
        for download in &downloads {
            assert!(!download.upload, "{} goes up, not down", download.key);
        }
        Self {
            id: id.into(),
            downloads,
            asks: Asks::default(),
            on_clear: None,
        }
    }

    asks!(pause);
    asks!(resume);
    asks!(retry);
    asks!(cancel);
    asks!(open);
    asks!(show);

    /// Clears the finished ones from the list.
    pub fn on_clear(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_clear = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for DownloadManager {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let (done, going): (Vec<&Transfer>, Vec<&Transfer>) = self
            .downloads
            .iter()
            .partition(|download| download.state == TransferState::Done);
        let drawn = |download: &Transfer| {
            row(
                (self.id.clone(), format!("download-{}", download.key)).into(),
                download,
                &self.asks,
                cx,
            )
        };
        let clear = self.on_clear.map(|clear| {
            Button::new((self.id.clone(), "clear"), "Clear")
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .on_click(move |_, window, cx| {
                    log::info!("download manager: cleared the finished");
                    clear(window, cx)
                })
        });
        let finished = (!done.is_empty()).then(|| {
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .px_3()
                .pt_2()
                .child(
                    div()
                        .text_size(theme.text_size(TextSize::Sm))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(colors.fg_muted)
                        .child("Finished"),
                )
                .children(clear)
        });
        let empty = self.downloads.is_empty().then(|| {
            div()
                .px_3()
                .py_4()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(colors.fg_muted)
                .child("No downloads yet.")
        });
        div()
            .debug_selector(|| "download-manager".into())
            .flex()
            .flex_col()
            .gap_1()
            .children(empty)
            .children(going.into_iter().map(drawn))
            .children(finished)
            .children(done.into_iter().map(drawn))
    }
}
