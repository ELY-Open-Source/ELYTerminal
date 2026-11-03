use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
};

use crate::{
    buttons::{Button, ButtonVariant, IconButton},
    forms::OnValue,
    motion::ProgressBar,
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, TextSize},
    typography::{
        Ellipsis,
        format::{DurationStyle, duration, file_size, percent},
    },
};

/// Where a model's download stands.
#[derive(Clone, Debug, PartialEq)]
pub enum DownloadState {
    Queued,
    /// Under way: the share fetched, and bytes a second.
    Fetching(f32, u64),
    Paused(f32),
    Done,
    Failed(SharedString),
}

/// One model to fetch: its key, its name, its size in bytes, and where it stands.
#[derive(Clone, Debug, PartialEq)]
pub struct Download {
    pub key: SharedString,
    pub name: SharedString,
    pub bytes: u64,
    pub state: DownloadState,
}

/// What a download's state says under its name: how much of how much, how fast, and how long is left.
pub(crate) fn status(download: &Download) -> String {
    let of = |share: f32| {
        let done = (download.bytes as f64 * f64::from(share)) as u64;
        format!(
            "{} of {}",
            file_size(done, false),
            file_size(download.bytes, false)
        )
    };
    match &download.state {
        DownloadState::Queued => "Queued".to_string(),
        DownloadState::Fetching(share, 0) => of(*share),
        DownloadState::Fetching(share, speed) => {
            let left = (download.bytes as f64 * f64::from(1.0 - share)) as u64 / speed;
            format!(
                "{} · {}/s · about {} left",
                of(*share),
                file_size(*speed, false),
                duration(left.max(1), DurationStyle::Compact)
            )
        }
        DownloadState::Paused(share) => format!(
            "Paused at {} · {}",
            percent(f64::from(*share), 0, false),
            of(*share)
        ),
        DownloadState::Done => file_size(download.bytes, false),
        DownloadState::Failed(reason) => format!("Failed · {reason}"),
    }
}

/// Models on their way to this machine: each row its name, how much of how much with speed and time left, a bar while it comes; Pause and Resume, Cancel, Retry once it fails, and Delete once it is here.
#[derive(IntoElement)]
pub struct ModelDownloadManager {
    id: ElementId,
    downloads: Vec<Download>,
    on_pause: Option<OnValue>,
    on_resume: Option<OnValue>,
    on_cancel: Option<OnValue>,
    on_retry: Option<OnValue>,
    on_delete: Option<OnValue>,
}

impl ModelDownloadManager {
    pub fn new(id: impl Into<ElementId>, downloads: impl IntoIterator<Item = Download>) -> Self {
        let downloads: Vec<Download> = downloads.into_iter().collect();
        for download in &downloads {
            if let DownloadState::Fetching(share, _) | DownloadState::Paused(share) = download.state
            {
                assert!(
                    (0.0..=1.0).contains(&share),
                    "download {} at {share} of 1",
                    download.key
                );
            }
        }
        Self {
            id: id.into(),
            downloads,
            on_pause: None,
            on_resume: None,
            on_cancel: None,
            on_retry: None,
            on_delete: None,
        }
    }

    pub fn on_pause(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_pause = Some(Rc::new(handler));
        self
    }

    pub fn on_resume(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_resume = Some(Rc::new(handler));
        self
    }

    pub fn on_cancel(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_cancel = Some(Rc::new(handler));
        self
    }

    pub fn on_retry(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_retry = Some(Rc::new(handler));
        self
    }

    /// Gets a downloaded model to take off this machine.
    pub fn on_delete(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_delete = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ModelDownloadManager {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let icon = |key: &SharedString,
                    what: &'static str,
                    glyph: IconName,
                    tip: &'static str,
                    handler: &Option<OnValue>| {
            handler.clone().map(|handler| {
                let key = key.clone();
                IconButton::new((self.id.clone(), format!("{what}-{key}")), glyph)
                    .size(ControlSize::Sm)
                    .tooltip(tip)
                    .on_click(move |_, window, cx| {
                        log::info!("model downloads: {what} {key}");
                        handler(&key, window, cx)
                    })
                    .into_any_element()
            })
        };
        let rows = self.downloads.iter().map(|download| {
            let key = &download.key;
            let actions: Vec<_> = match &download.state {
                DownloadState::Fetching(..) => vec![
                    icon(key, "pause", IconName::Pause, "Pause", &self.on_pause),
                    icon(key, "cancel", IconName::X, "Cancel", &self.on_cancel),
                ],
                DownloadState::Paused(_) => vec![
                    icon(key, "resume", IconName::Play, "Resume", &self.on_resume),
                    icon(key, "cancel", IconName::X, "Cancel", &self.on_cancel),
                ],
                DownloadState::Queued => {
                    vec![icon(key, "cancel", IconName::X, "Cancel", &self.on_cancel)]
                }
                DownloadState::Failed(_) => vec![
                    self.on_retry.clone().map(|retry| {
                        let key = key.clone();
                        Button::new((self.id.clone(), format!("retry-{key}")), "Retry")
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Sm)
                            .on_click(move |_, window, cx| {
                                log::info!("model downloads: retry {key}");
                                retry(&key, window, cx)
                            })
                            .into_any_element()
                    }),
                    icon(key, "delete", IconName::X, "Remove", &self.on_delete),
                ],
                DownloadState::Done => vec![icon(
                    key,
                    "delete",
                    IconName::Trash2,
                    "Delete from this machine",
                    &self.on_delete,
                )],
            };
            let bar = match download.state {
                DownloadState::Fetching(share, _) | DownloadState::Paused(share) => Some(
                    ProgressBar::new((self.id.clone(), format!("bar-{key}")), share),
                ),
                _ => None,
            };
            let failed = matches!(download.state, DownloadState::Failed(_));
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap_2()
                .py_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap_0p5()
                                .child(
                                    div()
                                        .text_size(theme.text_size(TextSize::Sm))
                                        .text_color(colors.fg)
                                        .child(Ellipsis::new(download.name.clone())),
                                )
                                .child(
                                    div()
                                        .text_size(theme.text_size(TextSize::Xs))
                                        .text_color(if failed {
                                            colors.danger
                                        } else {
                                            colors.fg_muted
                                        })
                                        .child(Ellipsis::new(status(download))),
                                ),
                        )
                        .child(
                            div()
                                .flex_none()
                                .flex()
                                .items_center()
                                .gap_1()
                                .children(actions.into_iter().flatten()),
                        ),
                )
                .children(bar)
        });
        div().flex().flex_col().children(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::{Download, DownloadState, status};

    fn download(state: DownloadState) -> Download {
        Download {
            key: "model".into(),
            name: "Orchid 8B".into(),
            bytes: 4_000_000_000,
            state,
        }
    }

    #[test]
    fn a_status_says_how_much_how_fast_and_how_long() {
        let fetching = download(DownloadState::Fetching(0.25, 40_000_000));
        assert_eq!(
            status(&fetching),
            "1.0 GB of 4.0 GB · 40 MB/s · about 1m 15s left"
        );
        assert_eq!(
            status(&download(DownloadState::Fetching(0.5, 0))),
            "2.0 GB of 4.0 GB"
        );
        assert_eq!(
            status(&download(DownloadState::Paused(0.5))),
            "Paused at 50% · 2.0 GB of 4.0 GB"
        );
        assert_eq!(status(&download(DownloadState::Done)), "4.0 GB");
        assert_eq!(
            status(&download(DownloadState::Failed("disk full".into()))),
            "Failed · disk full"
        );
    }
}
