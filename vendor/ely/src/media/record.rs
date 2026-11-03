use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, Styled, Window, div,
    prelude::*,
};
use web_time::Instant;

use crate::{
    buttons::{Button, ButtonVariant, IconButton, ToggleButton, ToggleItem},
    data_display::tone,
    feedback::Timer,
    forms::{OnFlag, Run},
    motion::Pulse,
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
};

/// Where a mic meter turns amber, then red.
const LOUD: (f32, f32) = (0.8, 0.95);
/// Segments along a mic meter.
const SEGMENTS: usize = 24;

/// How many of a mic meter's segments a level lights.
pub(crate) fn lit(level: f32) -> usize {
    (level * SEGMENTS as f32).round() as usize
}

/// A device toggle, such as a microphone's: its icon shows on or off, and a press flips it, logged under `owner`.
pub(crate) fn device_toggle(
    owner: &'static str,
    id: &ElementId,
    key: &'static str,
    icons: (IconName, IconName),
    tip: &'static str,
    (on, set): (bool, OnFlag),
) -> ToggleButton {
    let item = ToggleItem::new(key)
        .icon(if on { icons.0 } else { icons.1 })
        .tooltip(tip);
    ToggleButton::new((id.clone(), key), item, on)
        .size(ControlSize::Sm)
        .on_toggle(move |to, window, cx| {
            log::info!("{owner}: {key} {}", if to { "on" } else { "off" });
            set(to, window, cx)
        })
}

/// Where a screen recording stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recording {
    Idle,
    /// Running, counted from `since`; a resume moves `since` on by the pause.
    Running {
        since: Instant,
    },
    /// Paused at `at`, counted from `since`.
    Paused {
        since: Instant,
        at: Instant,
    },
}

/// A screen recording's controls in one pill. Idle, Record starts it; running, a live dot and the time it has run sit before pause and stop; paused, the time holds and resume takes pause's place. Mic and camera toggles follow, each shown with its handler.
#[derive(IntoElement)]
pub struct ScreenRecorderControls {
    id: ElementId,
    state: Recording,
    mic: Option<(bool, OnFlag)>,
    camera: Option<(bool, OnFlag)>,
    on_record: Option<Run>,
    on_pause: Option<Run>,
    on_resume: Option<Run>,
    on_stop: Option<Run>,
}

impl ScreenRecorderControls {
    pub fn new(id: impl Into<ElementId>, state: Recording) -> Self {
        if let Recording::Paused { since, at } = state {
            assert!(since <= at, "a pause before its recording started");
        }
        Self {
            id: id.into(),
            state,
            mic: None,
            camera: None,
            on_record: None,
            on_pause: None,
            on_resume: None,
            on_stop: None,
        }
    }

    pub fn on_record(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_record = Some(Rc::new(handler));
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

    pub fn on_stop(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_stop = Some(Rc::new(handler));
        self
    }

    /// Whether the microphone records too, and what gets the change.
    pub fn mic(
        mut self,
        on: bool,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.mic = Some((on, Rc::new(handler)));
        self
    }

    /// Whether the camera records too, and what gets the change.
    pub fn camera(
        mut self,
        on: bool,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.camera = Some((on, Rc::new(handler)));
        self
    }
}

impl RenderOnce for ScreenRecorderControls {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let id = self.id.clone();
        let action = |key: &'static str, icon: IconName, tip: &'static str, run: Option<Run>| {
            run.map(|run| {
                IconButton::new((id.clone(), key), icon)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .tooltip(tip)
                    .on_click(move |_, window, cx| {
                        log::info!("screen recorder: {key}");
                        run(window, cx)
                    })
            })
        };
        let dot = |live: bool| -> AnyElement {
            let color = if live { colors.danger } else { colors.fg_muted };
            let dot = div().size(theme.status_dot()).rounded_full().bg(color);
            match live {
                true => Pulse::new((id.clone(), "live"))
                    .child(dot)
                    .into_any_element(),
                false => dot.into_any_element(),
            }
        };
        let time = |since: Instant, at: Option<Instant>| {
            Timer::new((id.clone(), "time"), since)
                .stopped(at)
                .size(TextSize::Sm)
        };
        let body = match self.state {
            Recording::Idle => div().children(self.on_record.map(|run| {
                Button::new((id.clone(), "record"), "Record")
                    .icon(IconName::CircleDot)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .on_click(move |_, window, cx| {
                        log::info!("screen recorder: record");
                        run(window, cx)
                    })
            })),
            Recording::Running { since } => div()
                .flex()
                .items_center()
                .gap_2()
                .pl_2()
                .child(dot(true))
                .child(time(since, None))
                .children(action("pause", IconName::Pause, "Pause", self.on_pause))
                .children(action("stop", IconName::Square, "Stop", self.on_stop)),
            Recording::Paused { since, at } => div()
                .flex()
                .items_center()
                .gap_2()
                .pl_2()
                .child(dot(false))
                .child(time(since, Some(at)))
                .children(action("resume", IconName::Play, "Resume", self.on_resume))
                .children(action("stop", IconName::Square, "Stop", self.on_stop)),
        };
        let toggle = |key: &'static str,
                      icons: (IconName, IconName),
                      tip: &'static str,
                      set: Option<(bool, OnFlag)>| {
            set.map(|set| device_toggle("screen recorder", &id, key, icons, tip, set))
        };
        let devices = self.mic.is_some() || self.camera.is_some();
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap_1()
            .p_1()
            .rounded_full()
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .child(body)
            .when(devices, |pill| {
                pill.child(
                    div()
                        .w_px()
                        .h(theme.icon_size(IconSize::Md))
                        .bg(colors.border),
                )
            })
            .children(toggle(
                "mic",
                (IconName::Mic, IconName::MicOff),
                "Microphone",
                self.mic,
            ))
            .children(toggle(
                "camera",
                (IconName::Video, IconName::VideoOff),
                "Camera",
                self.camera,
            ))
    }
}

/// A microphone's level as it comes in, 0 to 1: segments lit up to it, amber near the top and red at the top. The host sends levels as they come.
#[derive(IntoElement)]
pub struct MicLevelMeter {
    level: f32,
}

impl MicLevelMeter {
    pub fn new(level: f32) -> Self {
        assert!((0.0..=1.0).contains(&level), "a level of {level}");
        Self { level }
    }
}

impl RenderOnce for MicLevelMeter {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let lit = lit(self.level);
        let (unlit, round) = (colors.border, theme.radius(Radius::Sm));
        div()
            .debug_selector(|| "mic-level".into())
            .h(theme.meter_track())
            .flex()
            .gap_0p5()
            .children((0..SEGMENTS).map(|ix| {
                let share = (ix + 1) as f32 / SEGMENTS as f32;
                let color = if ix < lit {
                    tone(share, LOUD, colors)
                } else {
                    unlit
                };
                div().flex_1().h_full().rounded(round).bg(color)
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::{SEGMENTS, lit};

    #[test]
    fn a_level_lights_the_segments_up_to_it() {
        assert_eq!(lit(0.0), 0);
        assert_eq!(lit(0.02), 0);
        assert_eq!(lit(0.03), 1);
        assert_eq!(lit(0.5), SEGMENTS / 2);
        assert_eq!(lit(1.0), SEGMENTS);
    }
}
