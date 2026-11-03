use std::{rc::Rc, time::Duration};

use gpui::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
};

use crate::{
    buttons::IconButton,
    chat::AudioMessage,
    forms::{OnFlag, Run},
    motion::ProgressBar,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
    typography::{
        Ellipsis,
        format::{DurationStyle, duration, percent},
        tabular,
    },
};

type OnSeek = Rc<dyn Fn(f32, &mut Window, &mut App)>;

/// A generated sound: its prompt, its model and length, a bar while it is made, then a player with its waveform. Regenerate and Download wait for it. The host plays it.
#[derive(IntoElement)]
pub struct AudioGenerationPlayer {
    id: ElementId,
    prompt: SharedString,
    model: SharedString,
    peaks: Vec<f32>,
    length: Duration,
    making: Option<f32>,
    played: Duration,
    playing: bool,
    on_toggle: Option<OnFlag>,
    on_seek: Option<OnSeek>,
    on_regenerate: Option<Run>,
    on_download: Option<Run>,
}

impl AudioGenerationPlayer {
    /// `peaks` are the waveform's heights, 0 to 1, left to right.
    pub fn new(
        id: impl Into<ElementId>,
        prompt: impl Into<SharedString>,
        model: impl Into<SharedString>,
        peaks: impl Into<Vec<f32>>,
        length: Duration,
    ) -> Self {
        let peaks: Vec<f32> = peaks.into();
        assert!(
            !peaks.is_empty() && peaks.iter().all(|peak| (0.0..=1.0).contains(peak)),
            "a sound's peaks lie within 0 to 1"
        );
        Self {
            id: id.into(),
            prompt: prompt.into(),
            model: model.into(),
            peaks,
            length,
            making: None,
            played: Duration::ZERO,
            playing: false,
            on_toggle: None,
            on_seek: None,
            on_regenerate: None,
            on_download: None,
        }
    }

    /// While it is made, the share done.
    pub fn making(mut self, share: f32) -> Self {
        assert!((0.0..=1.0).contains(&share), "a sound made to {share} of 1");
        self.making = Some(share);
        self
    }

    /// How far it has played, and whether it plays now.
    pub fn played(mut self, played: Duration, playing: bool) -> Self {
        self.played = played;
        self.playing = playing;
        self
    }

    /// Gets true to play, false to pause.
    pub fn on_toggle(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }

    /// Gets the share of the sound to seek to.
    pub fn on_seek(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_seek = Some(Rc::new(handler));
        self
    }

    pub fn on_regenerate(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_regenerate = Some(Rc::new(handler));
        self
    }

    pub fn on_download(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_download = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for AudioGenerationPlayer {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let making = self.making;
        let action = |key: &'static str, icon: IconName, tip: &'static str, run: Option<Run>| {
            run.map(|run| {
                IconButton::new((self.id.clone(), key), icon)
                    .size(ControlSize::Sm)
                    .tooltip(tip)
                    .disabled(making.is_some())
                    .on_click(move |_, window, cx| {
                        log::info!("audio generation: {key}");
                        run(window, cx)
                    })
            })
        };
        let body = match making {
            Some(share) => div()
                .flex()
                .flex_col()
                .gap_2()
                .child(ProgressBar::new((self.id.clone(), "making"), share))
                .child(
                    tabular(div())
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(colors.fg_muted)
                        .child(format!("Making · {}", percent(f64::from(share), 0, false))),
                )
                .into_any_element(),
            None => {
                let player =
                    AudioMessage::new((self.id.clone(), "player"), self.peaks, self.length)
                        .played(self.played, self.playing);
                let player = match self.on_toggle {
                    Some(toggle) => {
                        player.on_toggle(move |play, window, cx| toggle(play, window, cx))
                    }
                    None => player,
                };
                match self.on_seek {
                    Some(seek) => player.on_seek(move |share, window, cx| seek(share, window, cx)),
                    None => player,
                }
                .into_any_element()
            }
        };
        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_3()
                    .child(
                        div().flex_none().pt_0p5().child(
                            Icon::new(IconName::AudioLines)
                                .size(IconSize::Sm)
                                .color(colors.fg_muted),
                        ),
                    )
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
                                    .child(Ellipsis::new(self.prompt)),
                            )
                            .child(
                                div()
                                    .text_size(theme.text_size(TextSize::Xs))
                                    .text_color(colors.fg_muted)
                                    .child(Ellipsis::new(format!(
                                        "{} · {}",
                                        self.model,
                                        duration(self.length.as_secs(), DurationStyle::Clock)
                                    ))),
                            ),
                    )
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .gap_1()
                            .children(action(
                                "regenerate",
                                IconName::RefreshCw,
                                "Regenerate",
                                self.on_regenerate,
                            ))
                            .children(action(
                                "download",
                                IconName::Download,
                                "Download",
                                self.on_download,
                            )),
                    ),
            )
            .child(body)
    }
}
