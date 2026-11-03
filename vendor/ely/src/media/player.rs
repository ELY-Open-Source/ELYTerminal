use std::{rc::Rc, time::Duration};

use gpui::{
    App, ElementId, Entity, ImageSource, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, Styled, Window, div, img, prelude::*, transparent_black,
};

use super::scrubber::{OnTime, Scrubber, clock};
use crate::{
    forms::{OnFlag, Run},
    motion::duration as motion_duration,
    overlays::media_button,
    primitives::{FocusRing, IconName, framed, tab_stop},
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
    typography::tabular,
};

type OnSpeed = Rc<dyn Fn(f32, &mut Window, &mut App)>;

/// Speeds a video plays at.
pub const SPEEDS: [f32; 6] = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0];
/// How long a still pointer leaves the controls up while a video plays.
const IDLE: Duration = Duration::from_millis(2500);
/// Seconds J and L jump.
const JUMP: f64 = 10.0;

/// When the pointer last moved over the player, and whether a wake is set to hide the controls.
struct Watch {
    moved: web_time::Instant,
    waking: bool,
}

/// A video the host plays, one frame at a time, in its own shape. Controls sit over it and fade once the pointer rests while it plays: play and pause, the time, the scrubber, the speed, captions, picture in picture and full screen, each only with its handler. With focus, Space and K play and pause, J and L jump ten seconds and the arrows five, C turns captions, F fills the screen.
#[derive(IntoElement)]
pub struct VideoPlayer {
    id: ElementId,
    frame: Option<ImageSource>,
    ratio: f32,
    length: Duration,
    at: Duration,
    loaded: Duration,
    playing: bool,
    speed: f32,
    caption: Option<SharedString>,
    captions: Option<bool>,
    on_play: Option<OnFlag>,
    on_seek: Option<OnTime>,
    on_speed: Option<OnSpeed>,
    on_captions: Option<OnFlag>,
    on_picture: Option<Run>,
    on_fullscreen: Option<Run>,
}

impl VideoPlayer {
    /// `frame` is the frame to show, none before the first; `ratio` its width over its height.
    pub fn new(
        id: impl Into<ElementId>,
        frame: Option<ImageSource>,
        ratio: f32,
        length: Duration,
        at: Duration,
    ) -> Self {
        assert!(!length.is_zero(), "{at:?} of no length");
        let at = super::played(at, length, "video player");
        Self {
            id: id.into(),
            frame,
            ratio: crate::primitives::checked_ratio(ratio),
            length,
            at,
            loaded: Duration::ZERO,
            playing: false,
            speed: 1.0,
            caption: None,
            captions: None,
            on_play: None,
            on_seek: None,
            on_speed: None,
            on_captions: None,
            on_picture: None,
            on_fullscreen: None,
        }
    }

    pub fn loaded(mut self, loaded: Duration) -> Self {
        self.loaded = loaded;
        self
    }

    pub fn playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }

    /// One of `SPEEDS`.
    pub fn speed(mut self, speed: f32) -> Self {
        assert!(SPEEDS.contains(&speed), "a speed of {speed}");
        self.speed = speed;
        self
    }

    /// Captions on or off, and the words showing now; without this the video has none.
    pub fn captions(mut self, on: bool, caption: Option<SharedString>) -> Self {
        self.captions = Some(on);
        self.caption = caption.filter(|_| on);
        self
    }

    /// Gets whether to play.
    pub fn on_play(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_play = Some(Rc::new(handler));
        self
    }

    pub fn on_seek(mut self, handler: impl Fn(Duration, &mut Window, &mut App) + 'static) -> Self {
        self.on_seek = Some(Rc::new(handler));
        self
    }

    pub fn on_speed(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_speed = Some(Rc::new(handler));
        self
    }

    pub fn on_captions(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_captions = Some(Rc::new(handler));
        self
    }

    /// Gets a press on picture in picture.
    pub fn on_picture(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_picture = Some(Rc::new(handler));
        self
    }

    pub fn on_fullscreen(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_fullscreen = Some(Rc::new(handler));
        self
    }
}

/// Wakes the bar, as a move or a key just now; a timer lets it rest once all is still again.
fn wake(watch: &Entity<Watch>, window: &mut Window, cx: &mut App) {
    let arm = watch.update(cx, |watch, cx| {
        watch.moved = cx.background_executor().now();
        cx.notify();
        !std::mem::replace(&mut watch.waking, true)
    });
    if !arm {
        return;
    }
    let watch = watch.clone();
    window
        .spawn(cx, async move |cx| {
            loop {
                let wait = watch.read_with(cx, |watch, cx| {
                    IDLE.saturating_sub(cx.background_executor().now().duration_since(watch.moved))
                });
                if wait.is_zero() {
                    break;
                }
                cx.background_executor().timer(wait).await;
            }
            cx.update(|_, cx| {
                watch.update(cx, |watch, cx| {
                    watch.waking = false;
                    cx.notify();
                })
            })
        })
        .detach_and_log_err(cx);
}

impl RenderOnce for VideoPlayer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let started = cx.background_executor().now();
        let watch = window.use_keyed_state((self.id.clone(), "watch"), cx, move |_, _| Watch {
            moved: started,
            waking: false,
        });
        let keys = self.on_play.is_some()
            || self.on_seek.is_some()
            || self.on_fullscreen.is_some()
            || (self.on_captions.is_some() && self.captions.is_some());
        let focus = tab_stop((self.id.clone(), "focus").into(), keys, window, cx);
        let now = cx.background_executor().now();
        let in_bar = focus.contains_focused(window, cx) && !focus.is_focused(window);
        let resting = self.playing && now.duration_since(watch.read(cx).moved) >= IDLE && !in_bar;
        let fading = crate::motion::since_change(
            &(self.id.clone(), "fade").into(),
            resting,
            motion_duration(crate::motion::BASE, cx),
            window,
            cx,
        );
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let light = colors.on_media;
        let (length, at, playing, speed) = (self.length, self.at, self.playing, self.speed);
        let button = |key: &'static str, icon: IconName, run: Run| {
            let watch = watch.clone();
            media_button(
                (self.id.clone(), key),
                icon,
                ControlSize::Sm,
                true,
                move |window, cx| match resting {
                    true => wake(&watch, window, cx),
                    false => run(window, cx),
                },
                cx,
            )
        };
        let toggle: Option<Run> = self.on_play.clone().map(|play| {
            Rc::new(move |window: &mut Window, cx: &mut App| {
                log::info!("video player: {}", if playing { "pause" } else { "play" });
                play(!playing, window, cx)
            }) as Run
        });
        let turn: Option<Run> = self
            .on_captions
            .clone()
            .zip(self.captions)
            .map(|(set, on)| {
                Rc::new(move |window: &mut Window, cx: &mut App| {
                    log::info!("video player: captions {}", if on { "off" } else { "on" });
                    set(!on, window, cx)
                }) as Run
            });
        let play = toggle.clone().map(|run| {
            let icon = if playing {
                IconName::Pause
            } else {
                IconName::Play
            };
            button("play", icon, run)
        });
        let speed_button = self.on_speed.clone().map(|set| {
            let next = SPEEDS[(SPEEDS
                .iter()
                .position(|each| *each == speed)
                .expect("a listed speed")
                + 1)
                % SPEEDS.len()];
            div()
                .flex()
                .items_center()
                .child(button(
                    "speed",
                    IconName::Gauge,
                    Rc::new(move |window, cx| {
                        log::info!("video player: speed {next}×");
                        set(next, window, cx)
                    }),
                ))
                .child(
                    tabular(div())
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(light)
                        .child(format!("{speed}×")),
                )
        });
        let captions = turn.clone().map(|run| {
            let icon = if self.captions == Some(true) {
                IconName::Captions
            } else {
                IconName::CaptionsOff
            };
            button("captions", icon, run)
        });
        let picture = self
            .on_picture
            .clone()
            .map(|run| button("picture", IconName::PictureInPicture2, run));
        let fullscreen = self
            .on_fullscreen
            .clone()
            .map(|run| button("fullscreen", IconName::Fullscreen, run));
        let scrubber = Scrubber::new((self.id.clone(), "scrubber"), length, at)
            .loaded(self.loaded)
            .over_media();
        let scrubber = match self.on_seek.clone() {
            Some(seek) => {
                let watch = watch.clone();
                scrubber.on_seek(move |time, window, cx| match resting {
                    true => wake(&watch, window, cx),
                    false => seek(time, window, cx),
                })
            }
            None => scrubber,
        };
        let bar = div()
            .flex()
            .flex_col()
            .px_2()
            .pt_1()
            .rounded(theme.radius(Radius::Md))
            .bg(colors.media_backdrop.alpha(0.6))
            .child(scrubber)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .child(
                        div().flex().items_center().children(play).child(
                            tabular(div())
                                .flex_none()
                                .px_1()
                                .text_size(theme.text_size(TextSize::Xs))
                                .text_color(light)
                                .child(format!("{} / {}", clock(at), clock(length))),
                        ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .children(speed_button)
                            .children(captions)
                            .children(picture)
                            .children(fullscreen),
                    ),
            );
        let caption = self.caption.map(|words| {
            div().flex().justify_center().child(
                div()
                    .debug_selector(|| "video-caption".into())
                    .min_w_0()
                    .max_w(theme.prose_width())
                    .px_2()
                    .py_0p5()
                    .rounded(theme.radius(Radius::Sm))
                    .bg(colors.media_backdrop.alpha(0.7))
                    .text_color(light)
                    .text_size(theme.text_size(TextSize::Base))
                    .child(words),
            )
        });
        let bar = bar.when(resting, |bar| {
            bar.debug_selector(|| "video-controls-resting".into())
        });
        let shown = if resting { 0.0 } else { 1.0 };
        let bar = bar.opacity(fading.map_or(shown, |t| crate::motion::lerp(1.0 - shown, shown, t)));
        let controls = div()
            .absolute()
            .left_0()
            .right_0()
            .bottom_0()
            .p_2()
            .flex()
            .flex_col()
            .gap_2()
            .children(caption)
            .child(bar);
        let frame = self
            .frame
            .map(|frame| img(frame).id((self.id.clone(), "frame")).size_full());
        let (moved, pressed) = (watch.clone(), watch.clone());
        let keyed = (
            toggle,
            self.on_seek.clone(),
            turn,
            self.on_fullscreen.clone(),
        );
        framed(self.ratio, cx)
            .id(self.id.clone())
            .debug_selector(|| "video-player".into())
            .bg(colors.media_backdrop)
            .border_1()
            .border_color(transparent_black())
            .track_focus(&focus)
            .when(keys, |stage| stage.focus_ring(cx))
            .children(frame)
            .child(controls)
            .on_mouse_move(move |_, window, cx| wake(&moved, window, cx))
            .on_key_down(move |event, window, cx| {
                let held = &event.keystroke.modifiers;
                if held.platform || held.control {
                    return;
                }
                let (play, seek, captions, fullscreen) = &keyed;
                let step = |by: f64| {
                    Duration::from_secs_f64(
                        (at.as_secs_f64() + by).clamp(0.0, length.as_secs_f64()),
                    )
                };
                match event.keystroke.key.as_str() {
                    "space" | "k" => match play {
                        Some(play) => play(window, cx),
                        None => return,
                    },
                    "j" | "l" | "left" | "right" => {
                        let Some(seek) = seek else {
                            return;
                        };
                        let by = match event.keystroke.key.as_str() {
                            "j" => -JUMP,
                            "l" => JUMP,
                            "left" => -5.0,
                            _ => 5.0,
                        };
                        seek(step(by), window, cx)
                    }
                    "c" => match captions {
                        Some(turn) => turn(window, cx),
                        None => return,
                    },
                    "f" => match fullscreen {
                        Some(run) => run(window, cx),
                        None => return,
                    },
                    _ => return,
                }
                cx.stop_propagation();
                wake(&pressed, window, cx);
            })
    }
}
