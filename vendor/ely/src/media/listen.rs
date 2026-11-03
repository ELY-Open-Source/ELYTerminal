use std::{rc::Rc, time::Duration};

use gpui::{
    Animation, AnimationExt, App, ElementId, ImageSource, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div, relative,
};

use super::{
    controls::{MediaControls, VolumeControl},
    scrubber::{OnTime, Scrubber, clock, knob},
};
use crate::{
    forms::OnValue,
    lists::{ListItem, SelectableList},
    primitives::{Icon, IconName, Image},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
    typography::{Ellipsis, tabular},
};

/// A song: its key, title, artist and length.
#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    pub key: SharedString,
    pub title: SharedString,
    pub artist: SharedString,
    pub length: Duration,
}

/// How long the playing bars take to rise and fall.
const SWAY: Duration = Duration::from_millis(900);

/// Three bars that rise and fall while a song plays; they rest while it pauses and under reduced motion.
fn playing(id: ElementId, moving: bool, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let (color, round) = (theme.colors.accent, theme.radius(Radius::Sm));
    let (wide, still) = (theme.media().wave_bar, theme.reduced_motion);
    div()
        .flex_none()
        .flex()
        .items_end()
        .gap_0p5()
        .h(theme.icon_size(IconSize::Sm))
        .children((0..3).map(move |ix| {
            let rest = [0.45, 0.9, 0.65][ix];
            let bar = div().w(wide).h(relative(rest)).rounded_t(round).bg(color);
            if moving && !still {
                bar.with_animation(
                    (id.clone(), format!("bar-{ix}")),
                    Animation::new(SWAY).repeat(),
                    move |bar, t| {
                        let phase = (t + ix as f32 / 3.0) * std::f32::consts::PI;
                        bar.h(relative(0.3 + 0.7 * phase.sin().abs()))
                    },
                )
                .into_any_element()
            } else {
                bar.into_any_element()
            }
        }))
}

/// Songs in order, a row a song: its number, title and artist, and its length. The song under way is lit, its bars moving while it plays. Up and Down move; Enter or a double press plays a song.
#[derive(IntoElement)]
pub struct Playlist {
    id: ElementId,
    tracks: Vec<Track>,
    current: Option<SharedString>,
    playing: bool,
    on_play: Option<OnValue>,
}

impl Playlist {
    pub fn new(id: impl Into<ElementId>, tracks: impl IntoIterator<Item = Track>) -> Self {
        let tracks: Vec<Track> = tracks.into_iter().collect();
        for (ix, track) in tracks.iter().enumerate() {
            assert!(
                !tracks[..ix].iter().any(|other| other.key == track.key),
                "track {} twice",
                track.key
            );
        }
        Self {
            id: id.into(),
            tracks,
            current: None,
            playing: false,
            on_play: None,
        }
    }

    /// The song under way, by key, and whether it plays.
    pub fn current(mut self, key: impl Into<SharedString>, playing: bool) -> Self {
        let key = key.into();
        if !self.tracks.iter().any(|track| track.key == key) {
            log::error!("playlist: no track {key}; none under way");
            self.current = None;
            self.playing = false;
            return self;
        }
        self.current = Some(key);
        self.playing = playing;
        self
    }

    /// Gets the key of the song to play.
    pub fn on_play(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_play = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Playlist {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (muted, lead) = (theme.colors.fg_muted, theme.icon_size(IconSize::Lg));
        let small = theme.text_size(TextSize::Sm);
        let mut list = SelectableList::new(self.id.clone());
        for (ix, track) in self.tracks.into_iter().enumerate() {
            let now = self.current.as_ref() == Some(&track.key);
            let mark = if now {
                playing(
                    (self.id.clone(), format!("now-{ix}")).into(),
                    self.playing,
                    cx,
                )
                .into_any_element()
            } else {
                tabular(div())
                    .text_size(small)
                    .text_color(muted)
                    .child(format!("{}", ix + 1))
                    .into_any_element()
            };
            let row = ListItem::new((self.id.clone(), format!("track-{ix}")), track.title)
                .description(track.artist)
                .leading(div().w(lead).flex().justify_center().child(mark))
                .trailing(
                    tabular(div())
                        .text_size(small)
                        .text_color(muted)
                        .child(clock(track.length)),
                );
            list = list.row(track.key, row);
        }
        let list = list.selected(self.current);
        match self.on_play {
            Some(play) => list.on_activate(move |key, window, cx| {
                log::info!("playlist: play {key}");
                play(key, window, cx)
            }),
            None => list,
        }
    }
}

/// A song the host plays: its cover, title and artist, where it stands along a scrubber with the time played and left, the transport, and the volume.
#[derive(IntoElement)]
pub struct AudioPlayer {
    id: ElementId,
    title: SharedString,
    artist: SharedString,
    length: Duration,
    at: Duration,
    artwork: Option<ImageSource>,
    controls: Option<MediaControls>,
    volume: Option<VolumeControl>,
    on_seek: Option<OnTime>,
}

impl AudioPlayer {
    pub fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        artist: impl Into<SharedString>,
        length: Duration,
        at: Duration,
    ) -> Self {
        assert!(!length.is_zero(), "{at:?} of no length");
        let at = super::played(at, length, "now playing");
        Self {
            id: id.into(),
            title: title.into(),
            artist: artist.into(),
            length,
            at,
            artwork: None,
            controls: None,
            volume: None,
            on_seek: None,
        }
    }

    /// The cover, a square picture.
    pub fn artwork(mut self, source: impl Into<ImageSource>) -> Self {
        self.artwork = Some(source.into());
        self
    }

    /// The transport under the song.
    pub fn controls(mut self, controls: MediaControls) -> Self {
        self.controls = Some(controls);
        self
    }

    pub fn volume(mut self, volume: VolumeControl) -> Self {
        self.volume = Some(volume);
        self
    }

    /// Gets the time to seek to.
    pub fn on_seek(mut self, handler: impl Fn(Duration, &mut Window, &mut App) + 'static) -> Self {
        self.on_seek = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for AudioPlayer {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let (side, round) = (theme.media().artwork, theme.radius(Radius::Md));
        let cover = match self.artwork {
            Some(source) => Image::new((self.id.clone(), "artwork"), source)
                .size(side)
                .rounded(round)
                .into_any_element(),
            None => div()
                .size(side)
                .rounded(round)
                .bg(colors.sunken)
                .flex()
                .items_center()
                .justify_center()
                .child(
                    Icon::new(IconName::Music)
                        .size(IconSize::Lg)
                        .color(colors.fg_subtle),
                )
                .into_any_element(),
        };
        let scrubber = Scrubber::new((self.id.clone(), "scrubber"), self.length, self.at);
        let scrubber = match self.on_seek {
            Some(seek) => scrubber.on_seek(move |time, window, cx| seek(time, window, cx)),
            None => scrubber,
        };
        let times = |text: String| {
            tabular(div())
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(colors.fg_muted)
                .child(text)
        };
        let left = format!("-{}", clock(self.length - self.at));
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(div().flex_none().child(cover))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().text_color(colors.fg).child(Ellipsis::new(self.title)))
                            .child(
                                div()
                                    .text_size(theme.text_size(TextSize::Sm))
                                    .text_color(colors.fg_muted)
                                    .child(Ellipsis::new(self.artist)),
                            ),
                    ),
            )
            .child(
                div().flex().flex_col().gap_1().child(scrubber).child(
                    div()
                        .px(knob(cx) / 2.0)
                        .flex()
                        .justify_between()
                        .child(times(clock(self.at)))
                        .child(times(left)),
                ),
            )
            .children(self.controls)
            .children(self.volume)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Playlist, Track};

    #[test]
    fn a_track_gone_from_the_list_leaves_none_under_way() {
        let track = Track {
            key: "a".into(),
            title: "Tall Windows".into(),
            artist: "Aster Quartet".into(),
            length: Duration::from_secs(200),
        };
        let list = Playlist::new("list", [track])
            .current("a", true)
            .current("gone", true);
        assert_eq!((list.current, list.playing), (None, false));
    }
}
