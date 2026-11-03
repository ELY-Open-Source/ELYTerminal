use std::{rc::Rc, time::Duration};

use gpui::{
    App, Bounds, Corners, DragMoveEvent, ElementId, EmptyView, EntityId, HoverListenerMode,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, RenderOnce,
    StatefulInteractiveElement, Styled, Window, canvas, div, fill, point, prelude::*, relative,
    size, transparent_black,
};

use super::scrubber::{OnTime, along, clock, time_at, time_tip};
use crate::{
    forms::keyed,
    layout::fit,
    primitives::{FocusRing, tab_stop},
    theme::{ActiveTheme, Radius},
};

/// Seconds an arrow key moves the time.
const STEP: f64 = 5.0;

/// `peaks` gathered into `count` bars, each the loudest peak it covers.
pub(crate) fn resample(peaks: &[f32], count: usize) -> Vec<f32> {
    assert!(
        !peaks.is_empty() && count > 0,
        "{} peaks into {count}",
        peaks.len()
    );
    // In u64: bar by length outgrows a 32-bit usize on the web.
    let edge = |bar: usize| (bar as u64 * peaks.len() as u64 / count as u64) as usize;
    (0..count)
        .map(|bar| {
            let from = edge(bar);
            let to = edge(bar + 1).max(from + 1);
            peaks[from..to].iter().copied().fold(0.0, f32::max)
        })
        .collect()
}

/// A seeking drag on a waveform, marked with its waveform.
struct Seek(EntityId);

/// The waveform's bounds and the share under the pointer.
#[derive(Default)]
struct Wave {
    bounds: Bounds<Pixels>,
    hover: Option<f32>,
}

/// A sound's waveform, its peaks from 0 to 1 in time order, dark as far as it has played; the time under the pointer shows above it. A press or a drag seeks; with focus the arrows step five seconds, the Page keys fifty, Home and End go to the ends. The host plays it.
#[derive(IntoElement)]
pub struct AudioWaveform {
    id: ElementId,
    peaks: Rc<Vec<f32>>,
    length: Duration,
    at: Duration,
    on_seek: Option<OnTime>,
}

impl AudioWaveform {
    pub fn new(
        id: impl Into<ElementId>,
        peaks: impl Into<Vec<f32>>,
        length: Duration,
        at: Duration,
    ) -> Self {
        let peaks: Vec<f32> = peaks.into();
        assert!(
            !peaks.is_empty() && peaks.iter().all(|peak| (0.0..=1.0).contains(peak)),
            "peaks lie within 0 to 1"
        );
        assert!(!length.is_zero(), "{at:?} of no length");
        let at = super::played(at, length, "audio waveform");
        Self {
            id: id.into(),
            peaks: Rc::new(peaks),
            length,
            at,
            on_seek: None,
        }
    }

    /// Gets the time to seek to.
    pub fn on_seek(mut self, handler: impl Fn(Duration, &mut Window, &mut App) + 'static) -> Self {
        self.on_seek = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for AudioWaveform {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let wave = window.use_keyed_state((self.id.clone(), "wave"), cx, |_, _| Wave::default());
        let focus = tab_stop(
            (self.id.clone(), "focus").into(),
            self.on_seek.is_some(),
            window,
            cx,
        );
        let owner = wave.entity_id();
        let (length, at, peaks) = (self.length, self.at, self.peaks);
        let played = at.as_secs_f32() / length.as_secs_f32();
        let hover = wave.read(cx).hover;
        let theme = cx.theme();
        let sizes = theme.media();
        let rem = window.rem_size();
        let (bar, gap) = (sizes.wave_bar.to_pixels(rem), sizes.wave_gap.to_pixels(rem));
        let (dark, light) = (theme.colors.fg, theme.colors.fg_subtle);
        let tip = hover.map(|share| time_tip(share, clock(time_at(share, length)), cx));
        let measured = wave.clone();
        let drawn = canvas(
            move |bounds, window, cx| {
                if measured.read(cx).bounds != bounds {
                    measured.update(cx, |wave, cx| {
                        wave.bounds = bounds;
                        cx.notify();
                    });
                    window.request_animation_frame();
                }
            },
            move |bounds, _, window, _| {
                let width = f32::from(bounds.size.width);
                let pitch = f32::from(bar + gap);
                let (count, inset) = fit(bounds.size.width, bar, gap);
                let inset = f32::from(inset);
                for (ix, peak) in resample(&peaks, count).into_iter().enumerate() {
                    let left = inset + pitch * ix as f32;
                    let tall = (bounds.size.height * peak).max(bar);
                    let top = bounds.top() + (bounds.size.height - tall) / 2.0;
                    let origin = point(bounds.left() + Pixels::from(left), top);
                    let lit = (left + f32::from(bar) / 2.0) / width <= played;
                    window.paint_quad(
                        fill(
                            Bounds::new(origin, size(bar, tall)),
                            if lit { dark } else { light },
                        )
                        .corner_radii(Corners::all(bar / 2.0)),
                    );
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let (moved, left) = (wave.clone(), wave.clone());
        div()
            .id(self.id.clone())
            .debug_selector(|| "audio-waveform".into())
            .relative()
            .h(sizes.waveform)
            .border_1()
            .border_color(transparent_black())
            .rounded(theme.radius(Radius::Sm))
            .child(drawn)
            .children(tip)
            .on_mouse_move(move |event, _, cx| {
                moved.update(cx, |wave, cx| {
                    wave.hover = along(wave.bounds, event.position);
                    cx.notify();
                })
            })
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |inside, _, cx| {
                if !inside {
                    left.update(cx, |wave, cx| {
                        wave.hover = None;
                        cx.notify();
                    })
                }
            })
            .when_some(self.on_seek, |shown, seek| {
                let (pressed, dragged, keys) = (seek.clone(), seek.clone(), seek);
                let (pressed_wave, dragged_wave) = (wave.clone(), wave.clone());
                shown
                    .track_focus(&focus)
                    .focus_ring(cx)
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                        let Some(share) = along(pressed_wave.read(cx).bounds, event.position)
                        else {
                            return;
                        };
                        let time = time_at(share, length);
                        log::info!("audio waveform: seek to {}", clock(time));
                        pressed(time, window, cx);
                    })
                    .on_drag(Seek(owner), |_, _, _, cx| cx.new(|_| EmptyView))
                    .on_drag_move(move |event: &DragMoveEvent<Seek>, window, cx| {
                        if event.drag(cx).0 != owner {
                            return;
                        }
                        let bounds = dragged_wave.read(cx).bounds;
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
                        keys(Duration::from_secs_f64(next), window, cx);
                    })
            })
    }
}

/// Sound by pitch, low to high: bars rising from a line, each level from 0 to 1. The host sends new levels as it plays.
#[derive(IntoElement)]
pub struct AudioSpectrum {
    levels: Vec<f32>,
}

impl AudioSpectrum {
    pub fn new(levels: impl Into<Vec<f32>>) -> Self {
        let levels: Vec<f32> = levels.into();
        assert!(
            !levels.is_empty() && levels.iter().all(|level| (0.0..=1.0).contains(level)),
            "levels lie within 0 to 1"
        );
        Self { levels }
    }
}

impl RenderOnce for AudioSpectrum {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (color, round) = (theme.colors.fg_muted, theme.radius(Radius::Sm));
        div()
            .debug_selector(|| "audio-spectrum".into())
            .h(theme.media().waveform)
            .flex()
            .items_end()
            .gap_0p5()
            .children(self.levels.into_iter().map(move |level| {
                div()
                    .flex_1()
                    .h(relative(level.max(0.02)))
                    .rounded_t(round)
                    .bg(color)
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::resample;

    #[test]
    fn peaks_gather_into_bars_by_their_loudest() {
        assert_eq!(resample(&[0.1, 0.9, 0.3, 0.2], 2), [0.9, 0.3]);
        assert_eq!(resample(&[0.4, 0.8], 4), [0.4, 0.4, 0.8, 0.8]);
        assert_eq!(resample(&[0.5], 3), [0.5, 0.5, 0.5]);
    }
}
