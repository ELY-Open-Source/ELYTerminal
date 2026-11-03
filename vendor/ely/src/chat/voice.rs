use std::rc::Rc;

use gpui::{App, ElementId, IntoElement, ParentElement, RenderOnce, Styled, Window, div};
use web_time::Instant;

use crate::{
    buttons::{ButtonVariant, IconButton},
    feedback::Timer,
    forms::OnFlag,
    motion::Pulse,
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, TextSize},
};

/// Starts and stops dictation: a microphone at rest, a stop square while it records.
#[derive(IntoElement)]
pub struct VoiceInputButton {
    id: ElementId,
    recording: bool,
    on_toggle: OnFlag,
}

impl VoiceInputButton {
    /// `on_toggle` gets true to start recording, false to stop.
    pub fn new(
        id: impl Into<ElementId>,
        recording: bool,
        on_toggle: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            recording,
            on_toggle: Rc::new(on_toggle),
        }
    }
}

impl RenderOnce for VoiceInputButton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let (recording, toggle) = (self.recording, self.on_toggle);
        IconButton::new(
            self.id,
            if recording {
                IconName::Square
            } else {
                IconName::Mic
            },
        )
        .variant(if recording {
            ButtonVariant::Secondary
        } else {
            ButtonVariant::Ghost
        })
        .size(ControlSize::Sm)
        .tooltip(if recording {
            "Stop dictation"
        } else {
            "Dictate"
        })
        .on_click(move |_, window, cx| {
            log::info!("voice input: recording {}", !recording);
            toggle(!recording, window, cx)
        })
    }
}

/// Sound as it records: a red dot, how long it has run, and bars that follow the latest levels, 0 to 1, oldest first. The dot's rings rest under reduced motion.
#[derive(IntoElement)]
pub struct VoiceWaveform {
    id: ElementId,
    since: Instant,
    levels: Vec<f32>,
}

impl VoiceWaveform {
    pub fn new(id: impl Into<ElementId>, since: Instant, levels: impl Into<Vec<f32>>) -> Self {
        let levels: Vec<f32> = levels.into();
        assert!(
            levels.iter().all(|level| (0.0..=1.0).contains(level)),
            "levels lie within 0 to 1"
        );
        Self {
            id: id.into(),
            since,
            levels,
        }
    }
}

impl RenderOnce for VoiceWaveform {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let tall = theme.control_height(ControlSize::Sm);
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                Pulse::new((self.id.clone(), "dot")).child(
                    div()
                        .size(theme.status_dot())
                        .rounded_full()
                        .bg(colors.danger),
                ),
            )
            .child(Timer::new((self.id.clone(), "time"), self.since).size(TextSize::Sm))
            .child(
                div()
                    .flex_1()
                    .h(tall)
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_px()
                    .children(self.levels.into_iter().map(|level| {
                        div()
                            .w(theme.progress_thickness())
                            .h(tall * level.max(0.1))
                            .rounded_full()
                            .bg(colors.fg_muted)
                    })),
            )
    }
}
