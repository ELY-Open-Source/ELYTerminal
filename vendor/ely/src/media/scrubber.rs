use std::{rc::Rc, time::Duration};

use gpui::{
    App, Bounds, Div, DragMoveEvent, ElementId, EmptyView, EntityId, HoverListenerMode,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, Point, Rems, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, canvas, div, prelude::*, relative,
    transparent_black,
};

use crate::{
    forms::keyed,
    primitives::{FocusRing, tab_stop},
    theme::{ActiveTheme, Radius, TextSize},
    typography::{
        Ellipsis,
        format::{DurationStyle, duration},
        tabular,
    },
};

pub(crate) type OnTime = Rc<dyn Fn(Duration, &mut Window, &mut App)>;

/// Seconds an arrow key moves the time; Page keys move ten times as far.
const STEP: f64 = 5.0;

/// Where `pointer` falls along `bounds`, a share of its width held to it; none on a rail layout left no width.
pub(crate) fn along(bounds: Bounds<Pixels>, pointer: Point<Pixels>) -> Option<f32> {
    let width = f32::from(bounds.size.width);
    (width > 0.0).then(|| (f32::from(pointer.x - bounds.left()) / width).clamp(0.0, 1.0))
}

/// The time at `share` of `length`, the share held to the track.
pub(crate) fn time_at(share: f32, length: Duration) -> Duration {
    length.mul_f32(share.clamp(0.0, 1.0))
}

/// The chapter `at` falls in: the last to start at or before it.
pub(crate) fn chapter_at(starts: &[Duration], at: Duration) -> Option<usize> {
    starts.iter().rposition(|start| *start <= at)
}

/// A time as the crate's clock reads it.
pub(crate) fn clock(at: Duration) -> String {
    duration(at.as_secs(), DurationStyle::Clock)
}

/// The time under the pointer over a track. The free width splits by the pointer's share, so the tip stays inside the track and under the pointer.
pub(crate) fn time_tip(share: f32, text: String, cx: &App) -> Div {
    let theme = cx.theme();
    let side = |share: f32| div().flex_basis(relative(share));
    div()
        .absolute()
        .left_0()
        .bottom_full()
        .w_full()
        .flex()
        .items_end()
        .child(side(share))
        .child(
            tabular(div())
                .flex_none()
                .max_w_full()
                .mb_1()
                .px_1p5()
                .rounded(theme.radius(Radius::Sm))
                .bg(theme.colors.tooltip_bg)
                .text_color(theme.colors.tooltip_fg)
                .text_size(theme.text_size(TextSize::Xs))
                .debug_selector(|| "time-tip".into())
                .child(Ellipsis::new(text)),
        )
        .child(side(1.0 - share))
}

/// The scrubber's knob, round; its track is padded by half of it.
pub(crate) fn knob(cx: &App) -> Rems {
    cx.theme().slider_thumb() * 0.75
}

/// A seeking drag, marked with its scrubber.
struct Scrub(EntityId);

/// The track's bounds and the share under the pointer.
#[derive(Default)]
struct Track {
    bounds: Bounds<Pixels>,
    hover: Option<f32>,
}

/// Where a video or a song stands along its length: what has loaded, how far it has played, its chapters, and the time under the pointer. A press or a drag seeks; with focus the arrows step five seconds, the Page keys fifty, Home and End go to the ends. The host plays it.
#[derive(IntoElement)]
pub struct Scrubber {
    id: ElementId,
    length: Duration,
    at: Duration,
    loaded: Duration,
    chapters: Vec<(Duration, SharedString)>,
    over_media: bool,
    on_seek: Option<OnTime>,
}

impl Scrubber {
    /// `at` is how far it has played, within `length`.
    pub fn new(id: impl Into<ElementId>, length: Duration, at: Duration) -> Self {
        assert!(!length.is_zero(), "{at:?} of no length");
        let at = super::played(at, length, "scrubber");
        Self {
            id: id.into(),
            length,
            at,
            loaded: Duration::ZERO,
            chapters: Vec::new(),
            over_media: false,
            on_seek: None,
        }
    }

    /// Light on a dark layer over pictures, as a player's controls sit.
    pub fn over_media(mut self) -> Self {
        self.over_media = true;
        self
    }

    /// How far it has loaded.
    pub fn loaded(mut self, loaded: Duration) -> Self {
        if loaded > self.length {
            log::error!("scrubber: loaded {loaded:?} of {:?}; it fills", self.length);
        }
        self.loaded = loaded.min(self.length);
        self
    }

    /// A chapter from `start`, after the last one given.
    pub fn chapter(mut self, start: Duration, title: impl Into<SharedString>) -> Self {
        let after = self.chapters.last().is_none_or(|(last, _)| *last < start);
        assert!(after, "a chapter at {start:?} out of order");
        if start >= self.length {
            log::error!(
                "scrubber: a chapter at {start:?} past {:?}; left out",
                self.length
            );
            return self;
        }
        self.chapters.push((start, title.into()));
        self
    }

    /// Gets the time to seek to.
    pub fn on_seek(mut self, handler: impl Fn(Duration, &mut Window, &mut App) + 'static) -> Self {
        self.on_seek = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Scrubber {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let track = window.use_keyed_state((self.id.clone(), "track"), cx, |_, _| Track::default());
        let focus = tab_stop(
            (self.id.clone(), "focus").into(),
            self.on_seek.is_some(),
            window,
            cx,
        );
        let focused = focus.is_focused(window);
        let owner = track.entity_id();
        let (length, at) = (self.length, self.at);
        let share = |time: Duration| time.as_secs_f32() / length.as_secs_f32();
        let starts: Vec<Duration> = self.chapters.iter().map(|(start, _)| *start).collect();
        let hover = track.read(cx).hover;
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let (played, lane) = match self.over_media {
            true => (colors.on_media, colors.on_media.opacity(0.25)),
            false => (colors.fg, colors.border),
        };
        let (line, thumb) = (theme.slider_track(), knob(cx));
        let mut bounds: Vec<Duration> = if starts.first() == Some(&Duration::ZERO) {
            starts.clone()
        } else {
            std::iter::once(Duration::ZERO)
                .chain(starts.iter().copied())
                .collect()
        };
        bounds.push(length);
        let lanes = bounds.windows(2).map(|pair| {
            let (start, end) = (pair[0], pair[1]);
            let within = |time: Duration| {
                ((time.as_secs_f32() - start.as_secs_f32()) / (end - start).as_secs_f32())
                    .clamp(0.0, 1.0)
            };
            div()
                .relative()
                .h(line)
                .flex_basis(relative(share(end - start)))
                .flex_grow_1()
                .rounded_full()
                .bg(lane)
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .h_full()
                        .w(relative(within(self.loaded)))
                        .rounded_full()
                        .bg(played.opacity(0.35)),
                )
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .h_full()
                        .w(relative(within(at)))
                        .rounded_full()
                        .bg(played),
                )
        });
        let tip = hover.map(|hovered| {
            let time = time_at(hovered, length);
            let name = chapter_at(&starts, time).map(|ix| self.chapters[ix].1.clone());
            let text = match name {
                Some(name) => format!("{} · {name}", clock(time)),
                None => clock(time),
            };
            time_tip(hovered, text, cx)
        });
        let knob = (hover.is_some() || focused).then(|| {
            div()
                .absolute()
                .left(relative(share(at)))
                .top(relative(0.5))
                .size_0()
                .flex()
                .items_center()
                .justify_center()
                .child(div().flex_none().size(thumb).rounded_full().bg(played))
        });
        let measured = track.clone();
        let rail = div()
            .debug_selector(|| "scrubber-rail".into())
            .relative()
            .w_full()
            .flex()
            .gap_0p5()
            .items_center()
            .h(thumb)
            .children(lanes)
            .child(
                canvas(
                    move |bounds, window, cx| {
                        if measured.read(cx).bounds != bounds {
                            measured.update(cx, |track, cx| {
                                track.bounds = bounds;
                                cx.notify();
                            });
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .children(knob)
            .children(tip);
        let (moved, left) = (track.clone(), track.clone());
        div()
            .id(self.id.clone())
            .debug_selector(|| "scrubber".into())
            .px(thumb / 2.0)
            .rounded(theme.radius(Radius::Sm))
            .border_1()
            .border_color(transparent_black())
            .child(rail)
            .on_mouse_move(move |event, _, cx| {
                moved.update(cx, |track, cx| {
                    track.hover = along(track.bounds, event.position);
                    cx.notify();
                })
            })
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |inside, _, cx| {
                if !inside {
                    left.update(cx, |track, cx| {
                        track.hover = None;
                        cx.notify();
                    })
                }
            })
            .when_some(self.on_seek, |scrubber, seek| {
                let (pressed, dragged, keys) = (seek.clone(), seek.clone(), seek);
                let (pressed_track, dragged_track) = (track.clone(), track.clone());
                scrubber
                    .track_focus(&focus)
                    .focus_ring(cx)
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                        let Some(share) = along(pressed_track.read(cx).bounds, event.position)
                        else {
                            return;
                        };
                        let time = time_at(share, length);
                        log::info!("scrubber: seek to {}", clock(time));
                        pressed(time, window, cx);
                    })
                    .on_drag(Scrub(owner), |_, _, _, cx| cx.new(|_| EmptyView))
                    .on_drag_move(move |event: &DragMoveEvent<Scrub>, window, cx| {
                        if event.drag(cx).0 != owner {
                            return;
                        }
                        let bounds = dragged_track.read(cx).bounds;
                        let Some(share) = along(bounds, event.event.position) else {
                            return;
                        };
                        dragged(time_at(share, length), window, cx);
                    })
                    .on_key_down(move |event, window, cx| {
                        let (now, end) = (at.as_secs_f64(), length.as_secs_f64());
                        let Some(next) = keyed(&event.keystroke.key, now, 0.0, end, STEP) else {
                            return;
                        };
                        cx.stop_propagation();
                        let time = Duration::from_secs_f64(next);
                        log::info!("scrubber: seek to {}", clock(time));
                        keys(time, window, cx);
                    })
            })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{chapter_at, clock, time_at};

    #[test]
    fn a_share_is_a_time_held_to_the_track() {
        let length = Duration::from_secs(200);
        assert_eq!(time_at(0.25, length), Duration::from_secs(50));
        assert_eq!(time_at(1.4, length), length);
        assert_eq!(time_at(-0.2, length), Duration::ZERO);
    }

    #[test]
    fn a_time_falls_in_the_last_chapter_begun() {
        let starts = [Duration::from_secs(10), Duration::from_secs(60)];
        assert_eq!(chapter_at(&starts, Duration::from_secs(5)), None);
        assert_eq!(chapter_at(&starts, Duration::from_secs(10)), Some(0));
        assert_eq!(chapter_at(&starts, Duration::from_secs(90)), Some(1));
        assert_eq!(clock(Duration::from_secs(83)), "01:23");
    }
}
