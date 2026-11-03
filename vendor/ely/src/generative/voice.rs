use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
    transparent_black,
};

use crate::{
    buttons::{ButtonVariant, IconButton},
    data_display::Badge,
    forms::OnValue,
    primitives::{Icon, IconName, tab_stop},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
    typography::Ellipsis,
};

/// A voice to speak with: its key, its name, a line on how it sounds, and tags such as its language.
#[derive(Clone, Debug, PartialEq)]
pub struct Voice {
    pub key: SharedString,
    pub name: SharedString,
    pub about: SharedString,
    pub tags: Vec<SharedString>,
}

type OnPlay = Rc<dyn Fn(Option<SharedString>, &mut Window, &mut App)>;

/// Voices to speak with, a row each: a sample to play, the name and how it sounds, and its tags, which drop below when narrow. A press or Enter chooses a row; the chosen one takes the active wash and a check. The host plays the samples.
#[derive(IntoElement)]
pub struct TTSVoicePicker {
    id: ElementId,
    voices: Vec<Voice>,
    selected: Option<SharedString>,
    playing: Option<SharedString>,
    on_select: Option<OnValue>,
    on_play: Option<OnPlay>,
}

impl TTSVoicePicker {
    pub fn new(id: impl Into<ElementId>, voices: impl IntoIterator<Item = Voice>) -> Self {
        Self {
            id: id.into(),
            voices: voices.into_iter().collect(),
            selected: None,
            playing: None,
            on_select: None,
            on_play: None,
        }
    }

    pub fn selected(mut self, key: impl Into<SharedString>) -> Self {
        self.selected = Some(key.into());
        self
    }

    /// The voice whose sample plays now.
    pub fn playing(mut self, key: impl Into<SharedString>) -> Self {
        self.playing = Some(key.into());
        self
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Gets the voice whose sample to play, or none to stop.
    pub fn on_play(
        mut self,
        handler: impl Fn(Option<SharedString>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_play = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for TTSVoicePicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        for key in [&self.selected, &self.playing].into_iter().flatten() {
            if !self.voices.iter().any(|voice| &voice.key == key) {
                log::error!("voice picker: voice {key} is not listed; none marked");
            }
        }
        let pickable = self.on_select.is_some();
        let focuses: Vec<_> = self
            .voices
            .iter()
            .map(|voice| {
                let focus = tab_stop(
                    (self.id.clone(), format!("focus-{}", voice.key)).into(),
                    pickable,
                    window,
                    cx,
                );
                let focused = focus.is_focused(window);
                (focus, focused)
            })
            .collect();
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let label = theme.label_width();
        let rows = self
            .voices
            .into_iter()
            .zip(focuses)
            .map(|(voice, (focus, focused))| {
                let chosen = self.selected.as_ref() == Some(&voice.key);
                let sounding = self.playing.as_ref() == Some(&voice.key);
                let (pick, play, key) = (
                    self.on_select.clone(),
                    self.on_play.clone(),
                    voice.key.clone(),
                );
                let sample = play.map(|play| {
                    let key = key.clone();
                    IconButton::new(
                        (self.id.clone(), format!("play-{}", voice.key)),
                        if sounding {
                            IconName::CircleStop
                        } else {
                            IconName::Play
                        },
                    )
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .tooltip(if sounding {
                        "Stop the sample"
                    } else {
                        "Play a sample"
                    })
                    .on_click(move |_, window, cx| {
                        cx.stop_propagation();
                        let next = (!sounding).then(|| key.clone());
                        log::info!("voice picker: sample {next:?}");
                        play(next, window, cx)
                    })
                });
                div()
                    .id((self.id.clone(), format!("voice-{}", voice.key)))
                    .w_full()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_x_3()
                    .gap_y_1()
                    .px_2()
                    .py_2()
                    .rounded(theme.radius(Radius::Md))
                    .border_1()
                    .border_color(if focused {
                        colors.focus
                    } else {
                        transparent_black()
                    })
                    .when(chosen, |row| row.bg(colors.active))
                    .when_some(pick, |row, pick| {
                        row.track_focus(&focus)
                            .when(!chosen, |row| row.hover(|style| style.bg(colors.hover)))
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.prevent_default()
                            })
                            .on_click(move |_, window, cx| {
                                if chosen {
                                    return;
                                }
                                log::info!("voice picker: chose {key}");
                                pick(&key, window, cx)
                            })
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w(label)
                            .flex()
                            .items_center()
                            .gap_2()
                            .children(sample.map(|sample| div().flex_none().child(sample)))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(theme.text_size(TextSize::Sm))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(colors.fg)
                                            .child(Ellipsis::new(voice.name)),
                                    )
                                    .child(
                                        div()
                                            .text_size(theme.text_size(TextSize::Xs))
                                            .text_color(colors.fg_muted)
                                            .child(Ellipsis::new(voice.about)),
                                    ),
                            )
                            .when(chosen, |name| {
                                name.child(
                                    div().flex_none().child(
                                        Icon::new(IconName::Check)
                                            .size(IconSize::Sm)
                                            .color(colors.accent),
                                    ),
                                )
                            }),
                    )
                    .child(
                        div()
                            .debug_selector(|| format!("voice-tags-{}", voice.key))
                            .flex_none()
                            .max_w_full()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .children(voice.tags.into_iter().map(Badge::new)),
                    )
            });
        div().flex().flex_col().gap_0p5().children(rows)
    }
}
