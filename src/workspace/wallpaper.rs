//! The background picture and its gradient, where the settings put them.

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    thread::LocalKey,
};

use gpui::{
    AnyElement, App, Bounds, Hsla, ImageAssetLoader, IntoElement, ObjectFit, ParentElement, Pixels,
    RenderImage, Resource, Styled, StyledImage, Window, canvas, div, img, linear_color_stop,
    linear_gradient, point, px, size,
};
use image::{Frame, RgbaImage};

use crate::settings::{BackgroundScope, Mask, PictureRepeat, Settings, color};

/// The most tiles one picture draws; past it the picture draws once.
const MAX_TILES: usize = 400;
/// The most masked copies kept, each a whole picture's pixels.
const MASKED: usize = 4;

thread_local! {
    /// The terminal body's bounds, as the last frame laid them.
    static TERMINAL: Cell<Option<Bounds<Pixels>>> = const { Cell::new(None) };
    /// The box the picture paints in, inside Ely's resize margin.
    static GROUND: Cell<Option<Bounds<Pixels>>> = const { Cell::new(None) };
    /// The picture whose failure was logged last, so each frame does not log it again.
    static FAILED: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    /// The masked pictures by their image, mask and box, so each frame does not mask again.
    static MASKED_IMAGES: RefCell<HashMap<MaskKey, Arc<RenderImage>>> = RefCell::new(HashMap::new());
}

/// What makes a masked copy: the image, the mask and the box it covers.
#[derive(Clone, PartialEq, Eq, Hash)]
struct MaskKey {
    image: u64,
    angle: u32,
    stops: String,
    box_of: [u32; 6],
}

/// The picture's bounds inside `ground`: the terminal body, or all of it below the title bar.
fn box_of(scope: BackgroundScope, body: Bounds<Pixels>, ground: Bounds<Pixels>) -> Bounds<Pixels> {
    let shown = match scope {
        BackgroundScope::Terminal => body,
        BackgroundScope::Window => Bounds::new(
            point(ground.origin.x, body.origin.y),
            size(ground.size.width, ground.bottom() - body.origin.y),
        ),
    };
    Bounds::new(shown.origin - ground.origin, shown.size)
}

/// A canvas that records the terminal body's bounds, so the window's picture starts below the title bar.
pub(super) fn ruler() -> impl IntoElement {
    measure(&TERMINAL)
}

/// A canvas that records its box into `slot`, drawing again when it moves.
fn measure(slot: &'static LocalKey<Cell<Option<Bounds<Pixels>>>>) -> impl IntoElement {
    canvas(
        move |bounds, window, _: &mut App| {
            if slot.get() != Some(bounds) {
                slot.set(Some(bounds));
                window.request_animation_frame();
            }
        },
        |_, (), _, _| {},
    )
    .absolute()
    .inset_0()
}

/// The window's ground, the picture over it and its gradient over both, at the background's opacity; nothing without a picture.
pub(super) fn paint(
    settings: &Settings,
    base: Hsla,
    window: &mut Window,
    cx: &mut App,
) -> Option<AnyElement> {
    if settings.background_image.is_empty() {
        return None;
    }
    let ground = || {
        div()
            .absolute()
            .inset_0()
            .opacity(settings.background_opacity)
            .child(div().absolute().inset_0().bg(base))
            .child(measure(&GROUND))
    };
    let (Some(body), Some(within)) = (TERMINAL.get(), GROUND.get()) else {
        return Some(ground().into_any_element());
    };
    let bounds = box_of(settings.background_scope, body, within);
    let path: Arc<Path> = Path::new(&settings.background_image).into();
    let source = Resource::Path(path.clone());
    let Some(loaded) = window.use_asset::<ImageAssetLoader>(&source, cx) else {
        return Some(ground().into_any_element());
    };
    let image = match loaded {
        Ok(image) => image,
        Err(error) => {
            log_failure(&path, error);
            return Some(ground().into_any_element());
        }
    };
    let scale = window.scale_factor();
    let pixels = image.size(0);
    let intrinsic = (
        pixels.width.0 as f32 / scale,
        pixels.height.0 as f32 / scale,
    );
    if intrinsic.0 <= 0.0 || intrinsic.1 <= 0.0 {
        log::debug!("background: {} has no pixels", path.display());
        return Some(ground().into_any_element());
    }
    let container = (f32::from(bounds.size.width), f32::from(bounds.size.height));
    let drawn = settings.background_size.points(intrinsic, container);
    let free = (container.0 - drawn.0, container.1 - drawn.1);
    let place = settings.background_position.points(free);
    let mut boxed = div()
        .absolute()
        .overflow_hidden()
        .left(bounds.origin.x)
        .top(bounds.origin.y)
        .w(bounds.size.width)
        .h(bounds.size.height);
    let mask = settings.background_mask.then(|| {
        Mask::new(
            settings.background_mask_angle,
            &settings.background_mask_stops,
        )
    });
    let mask = mask.and_then(Result::ok);
    let once = settings.background_repeat == PictureRepeat::NoRepeat;
    for (x, y) in tiles(settings.background_repeat, place, drawn, container) {
        let masked = match (&mask, once) {
            (Some(mask), true) => masked(
                &image,
                mask,
                &settings.background_mask_stops,
                (x, y),
                drawn,
                container,
            ),
            _ => None,
        };
        let picture = masked.unwrap_or_else(|| image.clone());
        boxed = boxed.child(tile(
            &picture,
            x,
            y,
            drawn,
            settings.background_image_opacity,
        ));
    }
    if settings.background_gradient {
        boxed = boxed.child(gradient(settings));
    }
    Some(ground().child(boxed).into_any_element())
}

/// The gradient over the picture, from its first color to its last at its angle.
fn gradient(settings: &Settings) -> impl IntoElement {
    let stop =
        |text: &str, at| linear_color_stop(color(text).expect("the settings checked it"), at);
    div().absolute().inset_0().bg(linear_gradient(
        settings.background_gradient_angle,
        stop(&settings.background_gradient_from, 0.0),
        stop(&settings.background_gradient_to, 1.0),
    ))
}

/// One tile of the picture, at `x` and `y` in its box.
fn tile(
    image: &Arc<RenderImage>,
    x: f32,
    y: f32,
    drawn: (f32, f32),
    opacity: f32,
) -> impl IntoElement {
    img(image.clone())
        .absolute()
        .left(px(x))
        .top(px(y))
        .w(px(drawn.0))
        .h(px(drawn.1))
        .object_fit(ObjectFit::Fill)
        .opacity(opacity)
}

/// The picture with its mask as alpha, its straight BGRA bytes scaled; a copy kept by image, mask and box.
fn masked(
    image: &Arc<RenderImage>,
    mask: &Mask,
    stops: &str,
    place: (f32, f32),
    drawn: (f32, f32),
    container: (f32, f32),
) -> Option<Arc<RenderImage>> {
    let pixels = image.size(0);
    let (width, height) = (pixels.width.0, pixels.height.0);
    if width <= 0 || height <= 0 || container.0 <= 0.0 || container.1 <= 0.0 {
        return None;
    }
    let (width, height) = (width as usize, height as usize);
    let key = MaskKey {
        image: Arc::as_ptr(image) as usize as u64,
        angle: mask.angle.to_bits(),
        stops: stops.to_string(),
        box_of: [
            place.0.to_bits(),
            place.1.to_bits(),
            drawn.0.to_bits(),
            drawn.1.to_bits(),
            container.0.to_bits(),
            container.1.to_bits(),
        ],
    };
    if let Some(kept) = MASKED_IMAGES.with(|images| images.borrow().get(&key).cloned()) {
        return Some(kept);
    }
    let mut bytes = image.as_bytes(0)?.to_vec();
    if bytes.len() != width * height * 4 {
        return None;
    }
    for y in 0..height {
        for x in 0..width {
            let u = (place.0 + (x as f32 + 0.5) / width as f32 * drawn.0) / container.0;
            let v = (place.1 + (y as f32 + 0.5) / height as f32 * drawn.1) / container.1;
            let alpha = mask.alpha(u, v).clamp(0.0, 1.0);
            let at = (y * width + x) * 4 + 3;
            bytes[at] = (f32::from(bytes[at]) * alpha).round() as u8;
        }
    }
    let frame = Frame::new(RgbaImage::from_raw(width as u32, height as u32, bytes)?);
    let masked = Arc::new(RenderImage::new(vec![frame]));
    MASKED_IMAGES.with(|images| {
        let mut images = images.borrow_mut();
        if images.len() >= MASKED {
            images.clear();
        }
        images.insert(key, masked.clone());
    });
    Some(masked)
}

/// Where each tile starts, from the picture's place in its box; past `MAX_TILES`, the first one alone.
fn tiles(
    repeat: PictureRepeat,
    place: (f32, f32),
    drawn: (f32, f32),
    container: (f32, f32),
) -> Vec<(f32, f32)> {
    let across = matches!(repeat, PictureRepeat::Repeat | PictureRepeat::RepeatX);
    let down = matches!(repeat, PictureRepeat::Repeat | PictureRepeat::RepeatY);
    let xs = if across {
        along(place.0, drawn.0, container.0)
    } else {
        Some(vec![place.0])
    };
    let ys = if down {
        along(place.1, drawn.1, container.1)
    } else {
        Some(vec![place.1])
    };
    match (xs, ys) {
        (Some(xs), Some(ys)) if xs.len() * ys.len() <= MAX_TILES => ys
            .into_iter()
            .flat_map(|y| xs.iter().map(move |x| (*x, y)))
            .collect(),
        _ => {
            log::debug!("background: tiles of {drawn:?} pass {MAX_TILES}; one draws");
            vec![place]
        }
    }
}

/// The starts of one repeated tile across `extent`, from `at`; `None` past `MAX_TILES`.
fn along(at: f32, step: f32, extent: f32) -> Option<Vec<f32>> {
    if step <= 0.0 {
        return Some(vec![at]);
    }
    let mut start = at.rem_euclid(step);
    if start > 0.0 {
        start -= step;
    }
    let mut starts = Vec::new();
    while start < extent {
        if starts.len() == MAX_TILES {
            return None;
        }
        starts.push(start);
        start += step;
    }
    Some(starts)
}

/// Logs a picture that would not load, once per file.
fn log_failure(path: &Arc<Path>, error: impl std::fmt::Display) {
    let fresh = FAILED.with(|failed| {
        let mut failed = failed.borrow_mut();
        if failed.as_deref() == Some(&**path) {
            return false;
        }
        *failed = Some(path.to_path_buf());
        true
    });
    if fresh {
        log::error!("background: {}: {error}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body() -> Bounds<Pixels> {
        Bounds::new(point(px(220.0), px(38.0)), size(px(800.0), px(600.0)))
    }

    #[test]
    fn the_terminal_scope_paints_the_terminal_body() {
        let window = Bounds::new(point(px(0.0), px(0.0)), size(px(1200.0), px(700.0)));
        assert_eq!(box_of(BackgroundScope::Terminal, body(), window), body());
        // Inside a resize margin, the picture still lies on the body.
        let inset = Bounds::new(point(px(10.0), px(10.0)), size(px(1180.0), px(680.0)));
        let bounds = box_of(BackgroundScope::Terminal, body(), inset);
        assert_eq!(bounds.origin, point(px(210.0), px(28.0)));
    }

    #[test]
    fn the_window_scope_starts_below_the_title_bar_and_spans_the_window() {
        let window = Bounds::new(point(px(0.0), px(0.0)), size(px(1200.0), px(700.0)));
        let bounds = box_of(BackgroundScope::Window, body(), window);
        assert_eq!(bounds.origin, point(px(0.0), px(38.0)));
        assert_eq!(bounds.size, size(px(1200.0), px(662.0)));
        let inset = Bounds::new(point(px(10.0), px(10.0)), size(px(1180.0), px(680.0)));
        let bounds = box_of(BackgroundScope::Window, body(), inset);
        assert_eq!(bounds.origin, point(px(0.0), px(28.0)));
        assert_eq!(bounds.size, size(px(1180.0), px(652.0)));
    }

    #[test]
    fn a_repeated_picture_tiles_across_and_down() {
        let tiled = tiles(
            PictureRepeat::Repeat,
            (0.0, 0.0),
            (100.0, 100.0),
            (250.0, 250.0),
        );
        assert_eq!(tiled.len(), 9);
        assert!(tiled.contains(&(0.0, 0.0)));
        assert!(tiled.contains(&(200.0, 200.0)));
    }

    #[test]
    fn a_picture_out_of_place_tiles_from_before_the_box() {
        let tiled = tiles(
            PictureRepeat::RepeatX,
            (-30.0, 10.0),
            (100.0, 50.0),
            (250.0, 250.0),
        );
        assert_eq!(tiled, vec![(-30.0, 10.0), (70.0, 10.0), (170.0, 10.0)]);
    }

    #[test]
    fn one_picture_stays_where_it_was_placed() {
        let placed = tiles(
            PictureRepeat::NoRepeat,
            (12.0, 20.0),
            (100.0, 100.0),
            (250.0, 250.0),
        );
        assert_eq!(placed, vec![(12.0, 20.0)]);
    }

    #[test]
    fn a_picture_too_small_to_tile_draws_once() {
        let once = tiles(
            PictureRepeat::Repeat,
            (0.0, 0.0),
            (1.0, 1.0),
            (1000.0, 1000.0),
        );
        assert_eq!(once.len(), 1);
        let row = tiles(
            PictureRepeat::RepeatX,
            (0.0, 0.0),
            (2.0, 2.0),
            (1200.0, 100.0),
        );
        assert_eq!(row, vec![(0.0, 0.0)], "one axis past the cap draws once");
    }

    #[test]
    fn the_mask_fades_the_pictures_alpha() {
        let mut pixels = Vec::new();
        for _ in 0..4 {
            pixels.extend_from_slice(&[10, 20, 30, 255]);
        }
        let buffer = RgbaImage::from_raw(2, 2, pixels).expect("four pixels");
        let image = Arc::new(RenderImage::new(vec![Frame::new(buffer)]));
        let mask = Mask::new(90.0, "transparent, black").expect("two stops");
        let faded = masked(
            &image,
            &mask,
            "transparent, black",
            (0.0, 0.0),
            (2.0, 2.0),
            (2.0, 2.0),
        )
        .expect("a masked copy");
        let bytes = faded.as_bytes(0).expect("a frame");
        assert_eq!(bytes[3], 64, "the left is a quarter through the stops");
        assert_eq!(bytes[7], 191, "the right is three quarters");
        let kept = masked(
            &image,
            &mask,
            "transparent, black",
            (0.0, 0.0),
            (2.0, 2.0),
            (2.0, 2.0),
        )
        .expect("the same copy");
        assert!(Arc::ptr_eq(&faded, &kept), "a frame masks once");
    }
}
