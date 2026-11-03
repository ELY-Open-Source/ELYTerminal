use std::rc::Rc;

use gpui::{App, ElementId, IntoElement, ParentElement, RenderOnce, Styled, Window, div};

use super::player::SPEEDS;
use crate::{
    buttons::{ButtonVariant, IconButton, ToggleButton, ToggleItem},
    forms::{OnFlag, OnNumber, Run, Slider},
    menus::{DropdownMenu, Menu, MenuItem},
    primitives::IconName,
    theme::ControlSize,
};

type OnSpeed = Rc<dyn Fn(f32, &mut Window, &mut App)>;
type OnRepeat = Rc<dyn Fn(Repeat, &mut Window, &mut App)>;

/// The speaker for a level from 0 to 1: struck through when silent, with more waves as it grows.
pub(crate) fn speaker(level: f32, muted: bool) -> IconName {
    match level {
        _ if muted || level == 0.0 => IconName::VolumeX,
        level if level < 1.0 / 3.0 => IconName::Volume,
        level if level < 2.0 / 3.0 => IconName::Volume1,
        _ => IconName::Volume2,
    }
}

/// A volume from 0 to 1 and a mute: a speaker that mutes and unmutes, drawn by the level, beside a slider whose arrows step a twentieth. Muted, the slider rests at zero, and moving it unmutes.
#[derive(IntoElement)]
pub struct VolumeControl {
    id: ElementId,
    level: f32,
    muted: bool,
    on_level: Option<OnNumber>,
    on_mute: Option<OnFlag>,
}

impl VolumeControl {
    pub fn new(id: impl Into<ElementId>, level: f32, muted: bool) -> Self {
        assert!((0.0..=1.0).contains(&level), "a volume of {level}");
        Self {
            id: id.into(),
            level,
            muted,
            on_level: None,
            on_mute: None,
        }
    }

    pub fn on_level(mut self, handler: impl Fn(f64, &mut Window, &mut App) + 'static) -> Self {
        self.on_level = Some(Rc::new(handler));
        self
    }

    /// Gets true to mute, false to unmute.
    pub fn on_mute(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_mute = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for VolumeControl {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let muted = self.muted;
        let speaker = IconButton::new((self.id.clone(), "mute"), speaker(self.level, muted))
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Sm)
            .tooltip(if muted { "Unmute" } else { "Mute" })
            .disabled(self.on_mute.is_none());
        let speaker = match self.on_mute.clone() {
            Some(mute) => speaker.on_click(move |_, window, cx| {
                log::info!("volume: {}", if muted { "unmuted" } else { "muted" });
                mute(!muted, window, cx)
            }),
            None => speaker,
        };
        let shown = if muted { 0.0 } else { self.level };
        let slider = Slider::new((self.id.clone(), "level"), f64::from(shown))
            .range(0.0, 1.0)
            .step(0.05)
            .disabled(self.on_level.is_none());
        let slider = match self.on_level {
            Some(level) => {
                let mute = self.on_mute;
                slider.on_change(move |to, window, cx| {
                    if let Some(mute) = mute.as_ref().filter(|_| muted && to > 0.0) {
                        log::info!("volume: unmuted by the slider");
                        mute(false, window, cx);
                    }
                    level(to, window, cx)
                })
            }
            None => slider,
        };
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(div().flex_none().child(speaker))
            .child(div().flex_1().min_w_0().child(slider))
    }
}

/// How fast it plays, one of `SPEEDS`, chosen from a menu under its button.
#[derive(IntoElement)]
pub struct PlaybackSpeedControl {
    id: ElementId,
    speed: f32,
    on_change: Option<OnSpeed>,
}

impl PlaybackSpeedControl {
    pub fn new(id: impl Into<ElementId>, speed: f32) -> Self {
        assert!(SPEEDS.contains(&speed), "a speed of {speed}");
        Self {
            id: id.into(),
            speed,
            on_change: None,
        }
    }

    pub fn on_change(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for PlaybackSpeedControl {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let menu = SPEEDS.iter().fold(Menu::new(), |menu, speed| {
            let item = MenuItem::radio(format!("{speed}×"), *speed == self.speed);
            menu.item(match self.on_change.clone() {
                Some(change) => {
                    let speed = *speed;
                    item.on_click(move |window, cx| {
                        log::info!("playback speed: {speed}×");
                        change(speed, window, cx)
                    })
                }
                None => item.disabled(true),
            })
        });
        DropdownMenu::new(self.id, format!("{}×", self.speed), menu)
            .icon(IconName::Gauge)
            .variant(ButtonVariant::Ghost)
    }
}

/// Whether a list plays again: not, all of it, or the one song.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Repeat {
    Off,
    All,
    One,
}

impl Repeat {
    /// The next along a press: off, all, one, off.
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::All,
            Self::All => Self::One,
            Self::One => Self::Off,
        }
    }
}

/// The transport: shuffle, back, play or pause, forward and repeat, each shown only with its handler; shuffle and repeat stay lit while on.
#[derive(IntoElement)]
pub struct MediaControls {
    id: ElementId,
    playing: bool,
    shuffle: bool,
    repeat: Repeat,
    on_play: Option<OnFlag>,
    on_back: Option<Run>,
    on_forward: Option<Run>,
    on_shuffle: Option<OnFlag>,
    on_repeat: Option<OnRepeat>,
}

impl MediaControls {
    pub fn new(id: impl Into<ElementId>, playing: bool) -> Self {
        Self {
            id: id.into(),
            playing,
            shuffle: false,
            repeat: Repeat::Off,
            on_play: None,
            on_back: None,
            on_forward: None,
            on_shuffle: None,
            on_repeat: None,
        }
    }

    /// Gets true to play, false to pause.
    pub fn on_play(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_play = Some(Rc::new(handler));
        self
    }

    pub fn on_back(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_back = Some(Rc::new(handler));
        self
    }

    pub fn on_forward(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_forward = Some(Rc::new(handler));
        self
    }

    /// Whether it shuffles, and what gets the change.
    pub fn shuffle(
        mut self,
        on: bool,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.shuffle = on;
        self.on_shuffle = Some(Rc::new(handler));
        self
    }

    /// How it repeats, and what gets the next along a press.
    pub fn repeat(
        mut self,
        repeat: Repeat,
        handler: impl Fn(Repeat, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.repeat = repeat;
        self.on_repeat = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for MediaControls {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let step = |key: &'static str, icon: IconName, tip: &'static str, run: Option<Run>| {
            run.map(|run| {
                IconButton::new((id.clone(), key), icon)
                    .variant(ButtonVariant::Ghost)
                    .tooltip(tip)
                    .on_click(move |_, window, cx| {
                        log::info!("media controls: {key}");
                        run(window, cx)
                    })
            })
        };
        let playing = self.playing;
        let play = self.on_play.map(|play| {
            let icon = if playing {
                IconName::Pause
            } else {
                IconName::Play
            };
            IconButton::new((id.clone(), "play"), icon)
                .variant(ButtonVariant::Primary)
                .size(ControlSize::Lg)
                .tooltip(if playing { "Pause" } else { "Play" })
                .on_click(move |_, window, cx| {
                    log::info!("media controls: {}", if playing { "pause" } else { "play" });
                    play(!playing, window, cx)
                })
        });
        let shuffle = self.on_shuffle.map(|set| {
            let item = ToggleItem::new("shuffle")
                .icon(IconName::Shuffle)
                .tooltip("Shuffle");
            ToggleButton::new((id.clone(), "shuffle"), item, self.shuffle).on_toggle(
                move |to, window, cx| {
                    log::info!("media controls: shuffle {to}");
                    set(to, window, cx)
                },
            )
        });
        let repeat = self.on_repeat.map(|set| {
            let now = self.repeat;
            let icon = if now == Repeat::One {
                IconName::Repeat1
            } else {
                IconName::Repeat
            };
            let item = ToggleItem::new("repeat").icon(icon).tooltip("Repeat");
            ToggleButton::new((id.clone(), "repeat"), item, now != Repeat::Off).on_toggle(
                move |_, window, cx| {
                    log::info!("media controls: repeat {:?}", now.next());
                    set(now.next(), window, cx)
                },
            )
        });
        div()
            .flex()
            .items_center()
            .justify_center()
            .gap_2()
            .children(shuffle)
            .children(step("back", IconName::SkipBack, "Back", self.on_back))
            .children(play)
            .children(step(
                "forward",
                IconName::SkipForward,
                "Forward",
                self.on_forward,
            ))
            .children(repeat)
    }
}

#[cfg(test)]
mod tests {
    use super::{Repeat, speaker};
    use crate::primitives::IconName;

    #[test]
    fn the_speaker_grows_waves_with_the_level_and_strikes_through_when_silent() {
        assert_eq!(speaker(0.8, true), IconName::VolumeX);
        assert_eq!(speaker(0.0, false), IconName::VolumeX);
        assert_eq!(speaker(0.2, false), IconName::Volume);
        assert_eq!(speaker(0.5, false), IconName::Volume1);
        assert_eq!(speaker(0.9, false), IconName::Volume2);
    }

    #[test]
    fn repeat_steps_off_all_one_and_round() {
        assert_eq!(Repeat::Off.next(), Repeat::All);
        assert_eq!(Repeat::All.next(), Repeat::One);
        assert_eq!(Repeat::One.next(), Repeat::Off);
    }
}
