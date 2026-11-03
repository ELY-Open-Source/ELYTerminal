use std::{rc::Rc, time::Duration};

use gpui::{
    App, Bounds, CursorStyle, DragMoveEvent, ElementId, EmptyView, EntityId, ImageSource,
    InteractiveElement, IntoElement, MouseButton, ObjectFit, ParentElement, Pixels, RenderOnce,
    StatefulInteractiveElement, Styled, Window, canvas, div, img, prelude::*, relative,
    transparent_black,
};

use super::scrubber::{OnTime, along, clock, time_at};
use crate::{
    primitives::{FocusRing, tab_stop},
    theme::ActiveTheme,
};

type OnTrim = Rc<dyn Fn((Duration, Duration), &mut Window, &mut App)>;

/// Least span a trim keeps.
const LEAST: Duration = Duration::from_secs(1);

/// Which end of a trim a press holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum End {
    In,
    Out,
}

/// The trim after `end` moves to `to`, kept within `length` and at least `LEAST` long.
pub(crate) fn trimmed(
    trim: (Duration, Duration),
    end: End,
    to: Duration,
    length: Duration,
) -> (Duration, Duration) {
    let (start, stop) = trim;
    match end {
        End::In => (to.min(stop.saturating_sub(LEAST)), stop),
        End::Out => (start, to.max(start + LEAST).min(length)),
    }
}

/// The end of `trim` within `reach`, both shares of `length`, from `at`; the nearer when both are.
pub(crate) fn end_at(
    trim: (Duration, Duration),
    at: f32,
    reach: f32,
    length: Duration,
) -> Option<End> {
    let share = |time: Duration| time.as_secs_f32() / length.as_secs_f32();
    let (start, stop) = ((share(trim.0) - at).abs(), (share(trim.1) - at).abs());
    match (start <= reach, stop <= reach) {
        (true, true) if stop < start => Some(End::Out),
        (true, _) => Some(End::In),
        (false, true) => Some(End::Out),
        _ => None,
    }
}

/// A drag on the strip, marked with its strip.
struct Hold(EntityId);

/// The strip's bounds and the end the last press took.
#[derive(Default)]
struct Strip {
    bounds: Bounds<Pixels>,
    held: Option<End>,
}

/// A video's frames laid along its length under a playhead, and with a trim, the part kept between two handles. A press seeks and a drag scrubs; a handle's drag moves its end. With focus, the arrows step the playhead a frame's span, and I and O set the trim's ends at the playhead.
#[derive(IntoElement)]
pub struct VideoThumbnailStrip {
    id: ElementId,
    frames: Vec<ImageSource>,
    length: Duration,
    at: Duration,
    trim: Option<(Duration, Duration)>,
    on_seek: Option<OnTime>,
    on_trim: Option<OnTrim>,
}

impl VideoThumbnailStrip {
    /// `frames` spread evenly along `length`.
    pub fn new(
        id: impl Into<ElementId>,
        frames: impl IntoIterator<Item = impl Into<ImageSource>>,
        length: Duration,
        at: Duration,
    ) -> Self {
        let frames: Vec<ImageSource> = frames.into_iter().map(Into::into).collect();
        assert!(
            !frames.is_empty() && !length.is_zero(),
            "{} frames, {at:?} of {length:?}",
            frames.len()
        );
        let at = super::played(at, length, "thumbnail strip");
        Self {
            id: id.into(),
            frames,
            length,
            at,
            trim: None,
            on_seek: None,
            on_trim: None,
        }
    }

    /// The part kept, from its start to its end.
    pub fn trim(mut self, start: Duration, end: Duration) -> Self {
        assert!(start + LEAST <= end, "a trim of {start:?} to {end:?}");
        if end > self.length {
            log::error!(
                "thumbnail strip: a trim to {end:?} past {:?}; it ends there",
                self.length
            );
        }
        let end = end.min(self.length);
        if start + LEAST > end {
            log::error!("thumbnail strip: a trim from {start:?} leaves too little; none kept");
            self.trim = None;
            return self;
        }
        self.trim = Some((start, end));
        self
    }

    pub fn on_seek(mut self, handler: impl Fn(Duration, &mut Window, &mut App) + 'static) -> Self {
        self.on_seek = Some(Rc::new(handler));
        self
    }

    pub fn on_trim(
        mut self,
        handler: impl Fn((Duration, Duration), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_trim = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for VideoThumbnailStrip {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let strip = window.use_keyed_state((self.id.clone(), "strip"), cx, |_, _| Strip::default());
        let editable = self.on_seek.is_some() || self.on_trim.is_some();
        let focus = tab_stop((self.id.clone(), "focus").into(), editable, window, cx);
        let owner = strip.entity_id();
        let (length, at, trim) = (self.length, self.at, self.trim);
        let span = length / self.frames.len() as u32;
        let share = move |time: Duration| time.as_secs_f32() / length.as_secs_f32();
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let sizes = theme.media();
        let (tall, handle) = (sizes.strip, sizes.trim_handle);
        let reach = handle.to_pixels(window.rem_size());
        let frames = self.frames.into_iter().enumerate().map(|(ix, frame)| {
            div().flex_1().min_w_0().h_full().overflow_hidden().child(
                img(frame)
                    .id((self.id.clone(), format!("frame-{ix}")))
                    .size_full()
                    .object_fit(ObjectFit::Cover),
            )
        });
        let veil = |from: f32, to: f32| {
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(relative(from))
                .w(relative(to - from))
                .bg(colors.media_backdrop.alpha(0.55))
        };
        let grip = |at: f32, cursor: CursorStyle| {
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(relative(at))
                .w_0()
                .flex()
                .justify_center()
                .child(
                    div()
                        .flex_none()
                        .w(handle)
                        .h_full()
                        .rounded(theme.radius(crate::theme::Radius::Sm))
                        .bg(colors.accent)
                        .cursor(cursor),
                )
        };
        let kept = trim.map(|(start, stop)| {
            let (from, to) = (share(start), share(stop));
            div()
                .absolute()
                .inset_0()
                .child(veil(0.0, from))
                .child(veil(to, 1.0))
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left(relative(from))
                        .w(relative(to - from))
                        .border_y_2()
                        .border_color(colors.accent),
                )
                .child(grip(from, CursorStyle::ResizeLeftRight))
                .child(grip(to, CursorStyle::ResizeLeftRight))
        });
        let playhead = div()
            .absolute()
            .top_0()
            .bottom_0()
            .left(relative(share(at)))
            .w_0()
            .border_l_2()
            .border_color(colors.on_media)
            .debug_selector(|| "strip-playhead".into());
        let measured = strip.clone();
        let row = div()
            .relative()
            .h(tall)
            .flex()
            .debug_selector(|| "video-strip".into())
            .children(frames)
            .child(
                canvas(
                    move |bounds, window, cx| {
                        if measured.read(cx).bounds != bounds {
                            measured.update(cx, |strip, cx| {
                                strip.bounds = bounds;
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
            .children(kept)
            .child(playhead);
        let (on_seek, on_trim) = (self.on_seek, self.on_trim);
        let act = move |held: Option<End>, time: Duration, window: &mut Window, cx: &mut App| match (
            held, trim, &on_trim, &on_seek,
        ) {
            (Some(end), Some(trim), Some(set), _) => {
                set(trimmed(trim, end, time, length), window, cx)
            }
            (None, _, _, Some(seek)) => seek(time, window, cx),
            _ => {}
        };
        let act = Rc::new(act);
        let (pressed, moved, keys) = (act.clone(), act.clone(), act);
        let (pressed_strip, moved_strip) = (strip.clone(), strip.clone());
        div()
            .id(self.id.clone())
            .px(handle / 2.0)
            .border_1()
            .border_color(transparent_black())
            .child(row)
            .when(editable, |outer| {
                outer
                    .track_focus(&focus)
                    .focus_ring(cx)
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                        let bounds = pressed_strip.read(cx).bounds;
                        let Some(at) = along(bounds, event.position) else {
                            return;
                        };
                        let reach = f32::from(reach) / f32::from(bounds.size.width);
                        let held = trim.and_then(|trim| end_at(trim, at, reach, length));
                        pressed_strip.update(cx, |strip, _| strip.held = held);
                        if held.is_none() {
                            let time = time_at(at, length);
                            log::info!("video strip: seek to {}", clock(time));
                            pressed(None, time, window, cx);
                        }
                    })
                    .on_drag(Hold(owner), |_, _, _, cx| cx.new(|_| EmptyView))
                    .on_drag_move(move |event: &DragMoveEvent<Hold>, window, cx| {
                        if event.drag(cx).0 != owner {
                            return;
                        }
                        let (bounds, held) = {
                            let strip = moved_strip.read(cx);
                            (strip.bounds, strip.held)
                        };
                        let Some(share) = along(bounds, event.event.position) else {
                            return;
                        };
                        moved(held, time_at(share, length), window, cx);
                    })
                    .on_key_down(move |event, window, cx| {
                        let command = &event.keystroke.modifiers;
                        if command.platform || command.control {
                            return;
                        }
                        let (held, time) = match event.keystroke.key.as_str() {
                            "left" => (None, at.saturating_sub(span)),
                            "right" => (None, (at + span).min(length)),
                            "i" if trim.is_some() => (Some(End::In), at),
                            "o" if trim.is_some() => (Some(End::Out), at),
                            _ => return,
                        };
                        cx.stop_propagation();
                        log::info!("video strip: {:?} at {}", held, clock(time));
                        keys(held, time, window, cx);
                    })
            })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{End, VideoThumbnailStrip, end_at, trimmed};

    const TEN: Duration = Duration::from_secs(10);

    #[test]
    fn a_trim_past_the_video_ends_at_it_or_keeps_none() {
        let strip = || VideoThumbnailStrip::new("strip", ["a.jpg"], TEN, Duration::ZERO);
        let second = Duration::from_secs(1);
        assert_eq!(
            strip().trim(second * 2, second * 12).trim,
            Some((second * 2, TEN))
        );
        let gone = strip()
            .trim(second * 2, second * 6)
            .trim(second * 11, second * 13);
        assert_eq!(gone.trim, None);
    }

    #[test]
    fn a_trim_end_moves_and_stops_short_of_the_other_and_the_length() {
        let (trim, length) = ((TEN, TEN * 5), TEN * 10);
        assert_eq!(trimmed(trim, End::In, TEN * 2, length), (TEN * 2, TEN * 5));
        assert_eq!(
            trimmed(trim, End::In, TEN * 6, length),
            (TEN * 5 - Duration::from_secs(1), TEN * 5)
        );
        assert_eq!(
            trimmed(trim, End::Out, TEN / 2, length),
            (TEN, TEN + Duration::from_secs(1))
        );
        assert_eq!(trimmed(trim, End::Out, TEN * 20, length), (TEN, length));
    }

    #[test]
    fn a_press_holds_the_nearer_end_within_reach() {
        let (trim, length) = ((TEN, TEN * 5), TEN * 10);
        assert_eq!(end_at(trim, 0.11, 0.02, length), Some(End::In));
        assert_eq!(end_at(trim, 0.49, 0.02, length), Some(End::Out));
        assert_eq!(end_at(trim, 0.3, 0.02, length), None);
        let close = (TEN * 5, TEN * 5 + Duration::from_secs(1));
        assert_eq!(
            end_at(close, 0.509, 0.02, length),
            Some(End::Out),
            "the nearer of two"
        );
    }
}
