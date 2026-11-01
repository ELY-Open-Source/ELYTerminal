//! Paints a background style across the whole window.

use std::{cell::RefCell, collections::HashSet, path::Path, sync::Arc};

use gpui::{
    AnyElement, App, Hsla, ImageAssetLoader, IntoElement, ObjectFit, ParentElement, RenderImage,
    Resource, Styled, StyledImage, Window, div, img, linear_color_stop, linear_gradient, px,
};

use super::{Axis, Layer, Length, Paint, Repeat, Size};

/// The most tiles one repeating picture draws, past which one stands alone.
const MAX_TILES: usize = 400;

thread_local! {
    /// What failed already, so a frame does not log it again.
    static LOGGED: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

/// The background's layers across the window, as see-through as the window, or none when it paints nothing.
pub fn render(
    background: &super::Background,
    opacity: f32,
    base: Hsla,
    window: &mut Window,
    cx: &mut App,
) -> Option<AnyElement> {
    if background.is_empty() {
        return None;
    }
    // The picture's own opacity rides inside the stack, so the window's opacity still lets the desktop through.
    let alpha = background.opacity.unwrap_or(1.0);
    let viewport = window.viewport_size();
    let container = (f32::from(viewport.width), f32::from(viewport.height));
    let mut layers: Vec<AnyElement> = Vec::new();
    // The columns' glass leaves a painted background's ground to it, so the theme grounds it here.
    layers.push(div().absolute().inset_0().bg(base).into_any_element());
    if let Some(color) = background.color {
        layers.push(
            div()
                .absolute()
                .inset_0()
                .bg(color)
                .opacity(alpha)
                .into_any_element(),
        );
    }
    let mut painter = Painter {
        opacity: alpha,
        container,
        scale: window.scale_factor(),
        mask: background.mask.clone(),
        cx,
    };
    // Topmost first in the style, so the painter takes them back to front.
    for layer in background.layers.iter().rev() {
        match &layer.paint {
            Paint::Gradient(gradient) => layers.push(painter.gradient(gradient)),
            Paint::Image(path) => layers.extend(painter.picture(path, layer, window)),
        }
    }
    Some(
        div()
            .absolute()
            .inset_0()
            .overflow_hidden()
            .opacity(opacity)
            .children(layers)
            .into_any_element(),
    )
}

/// A window's background, painted layer by layer.
struct Painter<'a> {
    opacity: f32,
    /// The window's size, in the points elements are laid out in.
    container: (f32, f32),
    scale: f32,
    /// The gradient every picture fades through.
    mask: Option<super::Mask>,
    cx: &'a mut App,
}

impl Painter<'_> {
    /// A gradient covers the window, as CSS paints one.
    fn gradient(&mut self, gradient: &super::Gradient) -> AnyElement {
        let [from, to] = &gradient.stops[..] else {
            unreachable!("a gradient holds two stops");
        };
        div()
            .absolute()
            .inset_0()
            .bg(linear_gradient(
                gradient.angle,
                linear_color_stop(from.color, from.at),
                linear_color_stop(to.color, to.at),
            ))
            .opacity(self.opacity)
            .into_any_element()
    }

    /// One picture, sized, placed and tiled as its layer says.
    fn picture(&mut self, path: &Path, layer: &Layer, window: &mut Window) -> Vec<AnyElement> {
        let source = Resource::Path(Arc::from(path));
        let image = match window.use_asset::<ImageAssetLoader>(&source, self.cx) {
            Some(Ok(image)) => image,
            Some(Err(error)) => {
                log_once(format!("image:{}", path.display()), || {
                    log::error!("background: {}: {error}", path.display());
                });
                return Vec::new();
            }
            None => return Vec::new(),
        };
        let size = image.size(0);
        let intrinsic = (
            size.width.0 as f32 / self.scale,
            size.height.0 as f32 / self.scale,
        );
        if intrinsic.0 <= 0.0 || intrinsic.1 <= 0.0 {
            return Vec::new();
        }
        let (width, height) = sized(layer.size, intrinsic, self.container);
        if width <= 0.0 || height <= 0.0 {
            return Vec::new();
        }
        let x = offset(layer.position.x, self.container.0 - width);
        let y = offset(layer.position.y, self.container.1 - height);
        let image = match &self.mask {
            Some(mask) if layer.repeat == Repeat::NoRepeat => {
                match super::mask::masked(&image, mask, (x, y, width, height), self.container) {
                    Some(masked) => masked,
                    None => {
                        log_once(format!("mask:{}", path.display()), || {
                            log::error!("background: {} cannot be masked", path.display());
                        });
                        image.clone()
                    }
                }
            }
            Some(_) => {
                log_once(format!("mask-repeat:{}", path.display()), || {
                    log::error!(
                        "background: {} repeats, so its mask passes by",
                        path.display()
                    );
                });
                image.clone()
            }
            None => image.clone(),
        };
        let tiles = placed(layer.repeat, x, y, width, height, self.container);
        if tiles.is_empty() {
            log_once(format!("tiles:{}", path.display()), || {
                log::error!(
                    "background: {} makes more than {MAX_TILES} tiles; one draws",
                    path.display()
                );
            });
            return single(&image, x, y, width, height, self.opacity);
        }
        tiles
            .into_iter()
            .map(|(x, y)| tile(&image, x, y, width, height, self.opacity))
            .collect()
    }
}

/// A placed picture at `x`, `y`, `width` by `height`.
fn tile(
    image: &Arc<RenderImage>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    opacity: f32,
) -> AnyElement {
    img(image.clone())
        .absolute()
        .left(px(x))
        .top(px(y))
        .w(px(width))
        .h(px(height))
        .object_fit(ObjectFit::Fill)
        .opacity(opacity)
        .into_any_element()
}

/// The one tile of a repeat past the cap.
fn single(
    image: &Arc<RenderImage>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    opacity: f32,
) -> Vec<AnyElement> {
    vec![tile(image, x, y, width, height, opacity)]
}

/// Every tile `repeat` asks for, or none past the cap.
fn placed(
    repeat: Repeat,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    container: (f32, f32),
) -> Vec<(f32, f32)> {
    let xs: Vec<f32> = if matches!(repeat, Repeat::Repeat | Repeat::RepeatX) {
        along(x, width, container.0)
    } else {
        vec![x]
    };
    let ys: Vec<f32> = if matches!(repeat, Repeat::Repeat | Repeat::RepeatY) {
        along(y, height, container.1)
    } else {
        vec![y]
    };
    if xs.len().saturating_mul(ys.len()) > MAX_TILES || xs.len() > MAX_TILES || ys.len() > MAX_TILES
    {
        return Vec::new();
    }
    ys.iter()
        .flat_map(|y| xs.iter().map(move |x| (*x, *y)))
        .collect()
}

/// The starts of tiles of `step` that cover `extent` from `at`, edges included.
fn along(at: f32, step: f32, extent: f32) -> Vec<f32> {
    if step <= 0.0 {
        return Vec::new();
    }
    let mut first = at.rem_euclid(step);
    if first > 0.0 {
        first -= step;
    }
    let mut starts = Vec::new();
    let mut start = first;
    while start < extent && starts.len() <= MAX_TILES {
        starts.push(start);
        start += step;
    }
    starts
}

/// Where a picture sits in the space `free` leaves, as CSS resolves its position.
fn offset(axis: Axis, free: f32) -> f32 {
    match axis {
        Axis::Start => 0.0,
        Axis::End => free,
        Axis::Center => free / 2.0,
        Axis::Percent(share) => free * share,
        Axis::Px(px) => px,
    }
}

/// The picture's box, as CSS's `background-size` gives it.
fn sized(size: Size, intrinsic: (f32, f32), container: (f32, f32)) -> (f32, f32) {
    let (iw, ih) = intrinsic;
    match size {
        Size::Cover => {
            let scale = (container.0 / iw).max(container.1 / ih);
            (iw * scale, ih * scale)
        }
        Size::Contain => {
            let scale = (container.0 / iw).min(container.1 / ih);
            (iw * scale, ih * scale)
        }
        Size::Fill => container,
        Size::Auto => intrinsic,
        Size::Explicit(width, height) => {
            let width = length_of(width, container.0);
            let height = length_of(height, container.1);
            match (width, height) {
                (Some(width), Some(height)) => (width, height),
                (Some(width), None) => (width, ih * width / iw),
                (None, Some(height)) => (iw * height / ih, height),
                (None, None) => intrinsic,
            }
        }
    }
}

/// A length's points, or none for `auto`.
fn length_of(length: Length, extent: f32) -> Option<f32> {
    match length {
        Length::Auto => None,
        Length::Px(points) => Some(points),
        Length::Percent(share) => Some(extent * share),
    }
}

/// Runs `log` the first time `key` fails.
fn log_once(key: String, log: impl FnOnce()) {
    let first = LOGGED.with(|logged| logged.borrow_mut().insert(key));
    if first {
        log();
    }
}

/// The container's own size, for tests and callers that have no window.
#[cfg(test)]
mod tests;
