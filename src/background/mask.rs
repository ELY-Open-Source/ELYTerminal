//! A picture's mask, as CSS's `mask-image` describes it: an alpha gradient over the window.

use std::{cell::RefCell, collections::HashMap, sync::Arc};

use anyhow::Context as _;
use gpui::RenderImage;
use image::{Frame, RgbaImage};

use super::{Axis, Gradient, Length, Position, Size};

/// The most stops a mask gradient holds.
pub const MAX_STOPS: usize = 16;

/// A gradient that fades a picture in, in the window's own points.
#[derive(Clone, Debug, PartialEq)]
pub struct Mask {
    pub gradient: Gradient,
    pub size: Size,
    pub position: Position,
}

impl Mask {
    /// The mask's alpha at a point of the window.
    pub fn alpha(&self, x: f32, y: f32, window: (f32, f32)) -> f32 {
        let (width, height) = box_size(self.size, window);
        if width <= 0.0 || height <= 0.0 {
            return 0.0;
        }
        let left = side(self.position.x, window.0 - width);
        let top = side(self.position.y, window.1 - height);
        if x < left || y < top || x > left + width || y > top + height {
            return 0.0;
        }
        let u = ((x - left) / width).clamp(0.0, 1.0);
        let v = ((y - top) / height).clamp(0.0, 1.0);
        let at = along(&self.gradient, u, v);
        stop_alpha(&self.gradient, at)
    }
}

/// The mask's box, as `mask-size` gives it; a gradient has no size of its own.
fn box_size(size: Size, window: (f32, f32)) -> (f32, f32) {
    match size {
        Size::Explicit(width, height) => (
            length(width, window.0).unwrap_or(window.0),
            length(height, window.1).unwrap_or(window.1),
        ),
        _ => window,
    }
}

/// A length in points, or none for `auto`.
fn length(length: Length, extent: f32) -> Option<f32> {
    match length {
        Length::Auto => None,
        Length::Px(points) => Some(points),
        Length::Percent(share) => Some(extent * share),
    }
}

/// Where a side sits in the space `free` leaves.
fn side(axis: Axis, free: f32) -> f32 {
    match axis {
        Axis::Start => 0.0,
        Axis::End => free,
        Axis::Center => free / 2.0,
        Axis::Percent(share) => free * share,
        Axis::Px(points) => points,
    }
}

/// How far along the gradient a point of the box sits.
fn along(gradient: &Gradient, u: f32, v: f32) -> f32 {
    let angle = gradient.angle.to_radians();
    let (x, y) = (angle.sin(), -angle.cos());
    let corners = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];
    let project = |u: f32, v: f32| u * x + v * y;
    let low = corners
        .iter()
        .map(|(u, v)| project(*u, *v))
        .fold(f32::INFINITY, f32::min);
    let high = corners
        .iter()
        .map(|(u, v)| project(*u, *v))
        .fold(f32::NEG_INFINITY, f32::max);
    let span = high - low;
    if span <= f32::EPSILON {
        0.0
    } else {
        ((project(u, v) - low) / span).clamp(0.0, 1.0)
    }
}

/// The gradient's alpha at `at`.
fn stop_alpha(gradient: &Gradient, at: f32) -> f32 {
    let stops = &gradient.stops;
    if at <= stops[0].at {
        return stops[0].color.a;
    }
    for pair in stops.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        if at <= to.at {
            let span = to.at - from.at;
            let share = if span <= f32::EPSILON {
                1.0
            } else {
                (at - from.at) / span
            };
            return from.color.a + (to.color.a - from.color.a) * share;
        }
    }
    stops[stops.len() - 1].color.a
}

/// Reads `linear-gradient(...)` as a mask; its colors' alpha is the mask.
pub fn gradient(arguments: &str) -> anyhow::Result<Gradient> {
    super::gradient::linear_with(arguments, MAX_STOPS).with_context(|| "mask-image")
}

/// `image` with `mask` applied at `placement`; the same picture while the cache holds it.
pub fn masked(
    image: &Arc<RenderImage>,
    mask: &Mask,
    placement: (f32, f32, f32, f32),
    window: (f32, f32),
) -> Option<Arc<RenderImage>> {
    let key = key(image, mask, placement, window);
    if let Some(held) = CACHE.with(|cache| cache.borrow().get(&key).cloned()) {
        return Some(held);
    }
    let masked = apply(image, mask, placement, window)?;
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() >= 8 {
            cache.clear();
        }
        cache.insert(key, masked.clone());
    });
    Some(masked)
}

thread_local! {
    /// The pictures already masked, by the mask and the place they were drawn.
    static CACHE: RefCell<HashMap<String, Arc<RenderImage>>> = RefCell::new(HashMap::new());
}

/// A key of the picture, its mask and where it lands.
fn key(
    image: &Arc<RenderImage>,
    mask: &Mask,
    placement: (f32, f32, f32, f32),
    window: (f32, f32),
) -> String {
    format!("{}|{mask:?}|{placement:?}|{window:?}", image.id.0)
}

/// The picture's bytes with its alpha times the mask's, frame by frame's first.
fn apply(
    image: &Arc<RenderImage>,
    mask: &Mask,
    placement: (f32, f32, f32, f32),
    window: (f32, f32),
) -> Option<Arc<RenderImage>> {
    let size = image.size(0);
    let (width, height) = (size.width.0 as u32, size.height.0 as u32);
    if width == 0 || height == 0 {
        return None;
    }
    let mut bytes = image.as_bytes(0)?.to_vec();
    for row in 0..height {
        let y = placement.1 + (row as f32 + 0.5) / height as f32 * placement.3;
        for column in 0..width {
            let x = placement.0 + (column as f32 + 0.5) / width as f32 * placement.2;
            let alpha = mask.alpha(x, y, window);
            let at = ((row * width + column) * 4 + 3) as usize;
            bytes[at] = (f32::from(bytes[at]) * alpha).round() as u8;
        }
    }
    let buffer = RgbaImage::from_raw(width, height, bytes)?;
    Some(Arc::new(RenderImage::new(vec![Frame::new(buffer)])))
}

#[cfg(test)]
mod tests;
