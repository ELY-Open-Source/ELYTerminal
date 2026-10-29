//! Animation: an image's frames past its first, and their clock.

use std::time::{Duration, Instant};

use super::Shared;

/// kitty's gap for a new frame that names none.
pub(super) const GAP: u32 = 40;

/// How the clock moves frames, `s`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum State {
    /// Only `c` moves it, as every animation starts.
    #[default]
    Stopped,
    /// Moving, but waiting at the last frame for more.
    Loading,
    Running,
}

/// A frame past the first, composed whole, RGBA.
#[derive(Clone, Debug)]
pub(super) struct Frame {
    pub pixels: Vec<u8>,
    /// Milliseconds it shows; 0 skips it.
    pub gap: u32,
}

#[derive(Clone, Debug, Default)]
pub(super) struct Animation {
    pub frames: Vec<Frame>,
    /// The first frame's gap, 0 until `a=a` sets it.
    pub root_gap: u32,
    /// The frame shown, 0 the image's own pixels.
    pub current: usize,
    pub state: State,
    /// Loops to play, 0 for ever.
    pub loops: u32,
    pub looped: u32,
    /// When the current frame showed; none means the next tick.
    pub shown_at: Option<Instant>,
}

/// What an `a=f` command asks, kept while its chunks arrive.
#[derive(Clone, Copy, Debug)]
pub(super) struct Ask {
    pub image: u32,
    pub generation: u64,
    /// `x` and `y`, where its pixels go.
    pub at: (u32, u32),
    /// `c`, its base frame; `r`, the frame it edits.
    pub base: u32,
    pub edit: u32,
    /// `z`, its gap; `X=1` overwrites; `Y` the ground.
    pub gap: i32,
    pub overwrite: bool,
    pub background: u32,
}

impl Animation {
    /// Its frames, the first among them.
    pub fn count(&self) -> usize {
        self.frames.len() + 1
    }

    pub fn gap(&self, index: usize) -> u32 {
        match index {
            0 => self.root_gap,
            index => self.frames[index - 1].gap,
        }
    }

    pub fn set_gap(&mut self, index: usize, gap: u32) {
        match index {
            0 => self.root_gap = gap,
            index => self.frames[index - 1].gap = gap,
        }
    }

    pub fn bytes(&self) -> usize {
        self.frames.iter().map(|frame| frame.pixels.len()).sum()
    }

    /// Steps it at `now`: whether it moved, and when next.
    pub fn tick(&mut self, now: Instant) -> (bool, Option<Duration>) {
        let duration: u64 = (0..self.count())
            .map(|index| u64::from(self.gap(index)))
            .sum();
        let spent = self.loops > 0 && self.looped >= self.loops;
        if self.state == State::Stopped || self.frames.is_empty() || duration == 0 || spent {
            return (false, None);
        }
        let shown = self.shown_at.filter(|&at| at <= now).unwrap_or(now);
        self.shown_at = Some(shown);
        let gap = |animation: &Self| Duration::from_millis(animation.gap(animation.current).into());
        let mut next = shown + gap(self);
        let mut moved = false;
        // One frame a tick, no catching up, as kitty.
        if now >= next
            && let Some(index) = self.next()
        {
            (self.current, self.shown_at, moved) = (index, Some(now), true);
            next = now + gap(self);
        }
        (moved, (next > now).then(|| next - now))
    }

    /// The next frame with a gap; none while waiting.
    fn next(&mut self) -> Option<usize> {
        let mut index = self.current;
        loop {
            index = (index + 1) % self.count();
            if index == 0 {
                if self.state == State::Loading {
                    return None;
                }
                self.looped += 1;
                if self.loops > 0 && self.looped >= self.loops {
                    return None;
                }
            }
            if self.gap(index) != 0 {
                return Some(index);
            }
        }
    }
}

impl Shared {
    /// Steps the shown screen's placed animations; when to step again.
    pub fn tick(&mut self, now: Instant) -> Option<Duration> {
        let screen = self.screen();
        let mut placed: Vec<u32> = screen.placements.iter().map(|kept| kept.image).collect();
        placed.sort_unstable();
        placed.dedup();
        let mut soonest: Option<Duration> = None;
        for id in placed {
            let Some(image) = screen.store.get_mut(id) else {
                continue;
            };
            let Some(animation) = &mut image.animation else {
                continue;
            };
            let (moved, next) = animation.tick(now);
            if moved {
                log::trace!("graphics: image {id} shows frame {}", animation.current + 1);
                image.changed();
            }
            soonest = match (soonest, next) {
                (Some(soonest), Some(next)) => Some(soonest.min(next)),
                (soonest, next) => soonest.or(next),
            };
        }
        soonest
    }

    /// Gaps count afresh, as when a hidden session shows again.
    pub fn resume(&mut self) {
        for screen in &mut self.screens {
            for image in screen.store.images_mut() {
                if let Some(animation) = &mut image.animation {
                    animation.shown_at = None;
                }
            }
        }
    }
}

/// Fills a canvas with `Y`, red its top byte.
pub(super) fn fill(canvas: &mut [u8], color: u32) {
    let rgba = color.to_be_bytes();
    for pixel in canvas.chunks_exact_mut(4) {
        pixel.copy_from_slice(&rgba);
    }
}

/// Composes `source` onto `canvas` at `at`, clipped as kitty clips.
pub(super) fn compose(
    canvas: &mut [u8],
    (width, height): (u32, u32),
    source: &[u8],
    size: (u32, u32),
    at: (u32, u32),
    overwrite: bool,
) {
    if at.0 >= width || at.1 >= height {
        return;
    }
    let wide = size.0.min(width - at.0) as usize * 4;
    let tall = size.1.min(height - at.1) as usize;
    for row in 0..tall {
        let to = ((at.1 as usize + row) * width as usize + at.0 as usize) * 4;
        let from = row * size.0 as usize * 4;
        let (to, from) = (&mut canvas[to..to + wide], &source[from..from + wide]);
        if overwrite {
            to.copy_from_slice(from);
            continue;
        }
        for (to, from) in to.chunks_exact_mut(4).zip(from.chunks_exact(4)) {
            blend(to, from);
        }
    }
}

/// Source over, unpremultiplied, as Ghostty's wuffs blends.
fn blend(to: &mut [u8], from: &[u8]) {
    let (over, under) = (u32::from(from[3]), u32::from(to[3]));
    if over == 0 {
        return;
    }
    let (above, below) = (over * 255, under * (255 - over));
    let total = above + below;
    for channel in 0..3 {
        let mixed = u32::from(from[channel]) * above + u32::from(to[channel]) * below;
        to[channel] = ((mixed + total / 2) / total) as u8;
    }
    to[3] = ((total + 127) / 255) as u8;
}
