use std::{rc::Rc, time::Duration};

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, MouseButton, ObjectFit, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
    relative, transparent_black,
};

use super::media::shaped;
use crate::{
    buttons::{ButtonVariant, IconButton},
    documents::source,
    forms::OnFlag,
    primitives::{FocusRing, Icon, IconName, Image, tab_stop},
    theme::{ActiveTheme, ContainerSize, ControlSize, IconSize, Radius, TextSize},
    typography::{DurationStyle, format::duration, tabular},
};

type OnSeek = Rc<dyn Fn(f32, &mut Window, &mut App)>;

/// The share of a clip Left and Right move on its waveform.
const STEP: f32 = 0.05;

/// A clip's time as it plays: what played and the length, in clock digits.
fn clock(played: Duration, length: Duration) -> String {
    format!(
        "{} / {}",
        duration(played.as_secs(), DurationStyle::Clock),
        duration(length.as_secs(), DurationStyle::Clock)
    )
}

/// A voice note or a song: play and pause, a waveform lit as far as it has played, and the time; a press on the waveform seeks there, and so do Left and Right once it has focus. The host plays it.
#[derive(IntoElement)]
pub struct AudioMessage {
    id: ElementId,
    peaks: Rc<Vec<f32>>,
    length: Duration,
    played: Duration,
    playing: bool,
    on_toggle: Option<OnFlag>,
    on_seek: Option<OnSeek>,
}

impl AudioMessage {
    /// `peaks` are the waveform's heights, 0 to 1, left to right.
    pub fn new(id: impl Into<ElementId>, peaks: impl Into<Vec<f32>>, length: Duration) -> Self {
        let peaks: Vec<f32> = peaks.into();
        assert!(!peaks.is_empty(), "a waveform has peaks");
        assert!(
            peaks.iter().all(|peak| (0.0..=1.0).contains(peak)),
            "peaks lie within 0 to 1"
        );
        Self {
            id: id.into(),
            peaks: Rc::new(peaks),
            length,
            played: Duration::ZERO,
            playing: false,
            on_toggle: None,
            on_seek: None,
        }
    }

    /// How far it has played, and whether it plays now.
    pub fn played(mut self, played: Duration, playing: bool) -> Self {
        self.played = played.min(self.length);
        self.playing = playing;
        self
    }

    /// Gets true to play, false to pause.
    pub fn on_toggle(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }

    /// Gets the share of the clip to seek to.
    pub fn on_seek(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_seek = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for AudioMessage {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus = (self.on_seek.as_ref())
            .map(|_| tab_stop((self.id.clone(), "wave").into(), true, window, cx));
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let share = if self.length.is_zero() {
            0.0
        } else {
            self.played.as_secs_f32() / self.length.as_secs_f32()
        };
        let count = self.peaks.len();
        let playing = self.playing;
        let tall = theme.control_height(ControlSize::Md);
        div()
            .max_w(theme.prose_width())
            .flex()
            .items_center()
            .gap_3()
            .px_2()
            .py_1p5()
            .rounded_full()
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .children(self.on_toggle.map(|toggle| {
                IconButton::new(
                    (self.id.clone(), "toggle"),
                    if playing {
                        IconName::Pause
                    } else {
                        IconName::Play
                    },
                )
                .variant(ButtonVariant::Ghost)
                .tooltip(if playing { "Pause" } else { "Play" })
                .on_click(move |_, window, cx| {
                    log::info!("audio message: playing {}", !playing);
                    toggle(!playing, window, cx)
                })
            }))
            .child(
                div()
                    .id((self.id.clone(), "wave"))
                    .flex_1()
                    .min_w_0()
                    .h(tall)
                    .flex()
                    .items_center()
                    .gap_px()
                    .rounded(theme.radius(Radius::Sm))
                    .border_1()
                    .border_color(transparent_black())
                    .when_some(focus.zip(self.on_seek.clone()), |wave, (focus, seek)| {
                        wave.track_focus(&focus).focus_ring(cx).on_key_down(
                            move |event, window, cx| {
                                let step = match event.keystroke.key.as_str() {
                                    "left" => -STEP,
                                    "right" => STEP,
                                    _ => return,
                                };
                                cx.stop_propagation();
                                let next = (share + step).clamp(0.0, 1.0);
                                log::info!("audio message: seek to {next:.2}");
                                seek(next, window, cx)
                            },
                        )
                    })
                    .children(self.peaks.iter().enumerate().map(|(ix, peak)| {
                        let seek = self.on_seek.clone();
                        let lit = (ix as f32 + 0.5) / count as f32 <= share;
                        div()
                            .id((self.id.clone(), format!("bar-{ix}")))
                            .flex_1()
                            .h_full()
                            .flex()
                            .items_center()
                            .when_some(seek, |bar, seek| {
                                bar.cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, |_, window, _| {
                                        window.prevent_default()
                                    })
                                    .on_click(move |_, window, cx| {
                                        seek((ix as f32 + 0.5) / count as f32, window, cx)
                                    })
                            })
                            .child(
                                div()
                                    .w_full()
                                    .h(tall * peak.max(0.08))
                                    .rounded_full()
                                    .bg(if lit { colors.accent } else { colors.fg_subtle }),
                            )
                    })),
            )
            .child(
                tabular(
                    div()
                        .flex_none()
                        .pr_2()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(colors.fg_muted),
                )
                .child(clock(self.played, self.length)),
            )
    }
}

/// A video in a message: the frame the host shows, its poster or the playing frame, with play and pause over it and the time. The host decodes and plays it.
#[derive(IntoElement)]
pub struct VideoMessage {
    id: ElementId,
    frame: SharedString,
    size: (f32, f32),
    length: Duration,
    played: Duration,
    playing: bool,
    on_toggle: Option<OnFlag>,
}

impl VideoMessage {
    /// `frame` is the picture to show, a file or a web address; `width` and `height` the video's pixels.
    pub fn new(
        id: impl Into<ElementId>,
        frame: impl Into<SharedString>,
        width: f32,
        height: f32,
        length: Duration,
    ) -> Self {
        Self {
            id: id.into(),
            frame: frame.into(),
            size: (width, height),
            length,
            played: Duration::ZERO,
            playing: false,
            on_toggle: None,
        }
    }

    pub fn played(mut self, played: Duration, playing: bool) -> Self {
        self.played = played.min(self.length);
        self.playing = playing;
        self
    }

    /// Gets true to play, false to pause.
    pub fn on_toggle(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for VideoMessage {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let widest = theme
            .container_width(ContainerSize::Sm)
            .to_pixels(window.rem_size());
        let share = if self.length.is_zero() {
            0.0
        } else {
            self.played.as_secs_f32() / self.length.as_secs_f32()
        };
        let playing = self.playing;
        let toggle = self.on_toggle.clone();
        let video = shaped(self.size.0, self.size.1, widest)
            .id(self.id.clone())
            .relative()
            .bg(colors.media_backdrop)
            .when_some(toggle, |video, toggle| {
                video.cursor_pointer().on_click(move |_, window, cx| {
                    log::info!("video message: playing {}", !playing);
                    toggle(!playing, window, cx)
                })
            })
            .child(
                Image::new((self.id.clone(), "frame"), source(&self.frame))
                    .fit(ObjectFit::Contain)
                    .size_full(),
            )
            .when(!playing, |video| {
                video.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .size(theme.fab_size())
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_full()
                                .bg(colors.media_backdrop.opacity(0.6))
                                .child(
                                    Icon::new(IconName::Play)
                                        .size(IconSize::Lg)
                                        .color(colors.on_media),
                                ),
                        ),
                )
            })
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .flex()
                    .flex_col()
                    .child(
                        div().flex().justify_end().px_2().pb_1().child(
                            tabular(
                                div()
                                    .px_1p5()
                                    .rounded(theme.radius(Radius::Sm))
                                    .bg(colors.media_backdrop.opacity(0.6))
                                    .text_size(theme.text_size(TextSize::Xs))
                                    .text_color(colors.on_media),
                            )
                            .child(clock(self.played, self.length)),
                        ),
                    )
                    .child(
                        div()
                            .h(theme.progress_thickness())
                            .w_full()
                            .bg(colors.on_media.opacity(0.25))
                            .child(div().h_full().w(relative(share)).bg(colors.on_media)),
                    ),
            );
        div().child(video)
    }
}
