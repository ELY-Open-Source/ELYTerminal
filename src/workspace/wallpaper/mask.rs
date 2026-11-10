//! The picture's mask: its alpha in the picture's straight BGRA bytes.

use std::{cell::RefCell, sync::Arc};

use gpui::{ImageId, RenderImage, Window};
use image::{Frame, RgbaImage, imageops};

use crate::settings::{Mask, Settings};

thread_local! {
    /// The last masked picture and what made it, so a frame like the last does not mask again.
    static KEPT: RefCell<Option<(Key, Arc<RenderImage>)>> = const { RefCell::new(None) };
}

/// What makes a masked copy: the picture, the mask's settings, and every length that places it.
#[derive(Clone, PartialEq)]
struct Key {
    image: ImageId,
    angle: u32,
    stops: String,
    lengths: [u32; 11],
}

/// The most device pixels a side of a picture is stretched to before its mask.
const FINEST: f32 = 4096.0;
/// The most pixels a stretched picture holds, 64 MiB of them.
const MOST: u64 = 4096 * 4096;

/// The pixels a picture of `held` fades in when drawn `drawn` points at `scale`: more where it is drawn larger.
fn fine(held: (u32, u32), drawn: (f32, f32), scale: f32) -> (u32, u32) {
    let side = |held: u32, drawn: f32| held.max((drawn * scale).min(FINEST).ceil() as u32);
    let (wide, high) = (side(held.0, drawn.0), side(held.1, drawn.1));
    // A long strip stretched across would outgrow memory; the GPU stretches it instead.
    if u64::from(wide) * u64::from(high) > MOST {
        return held;
    }
    (wide, high)
}

/// The mask's box in the picture's box: its place, then its size.
pub(super) fn mask_box(settings: &Settings, container: (f32, f32)) -> ((f32, f32), (f32, f32)) {
    let size = settings.background_mask_size.points(container, container);
    let free = (container.0 - size.0, container.1 - size.1);
    (settings.background_mask_position.points(free), size)
}

/// The picture's first frame with the mask as its alpha, clear outside the mask's box.
fn fade(
    image: &RenderImage,
    mask: &Mask,
    (origin, size): ((f32, f32), (f32, f32)),
    place: (f32, f32),
    drawn: (f32, f32),
    scale: f32,
) -> Arc<RenderImage> {
    let pixels = image.size(0);
    let held = (pixels.width.0 as u32, pixels.height.0 as u32);
    let (wide, high) = fine(held, drawn, scale);
    let bytes = image.as_bytes(0).expect("a picture's first frame").to_vec();
    let frame = RgbaImage::from_raw(held.0, held.1, bytes).expect("as many bytes as it holds");
    let frame = if (wide, high) == held {
        frame
    } else {
        imageops::resize(&frame, wide, high, imageops::FilterType::Triangle)
    };
    let (width, height) = (wide as usize, high as usize);
    let mut bytes = frame.into_raw();
    let inside = |at: f32| (0.0..=1.0).contains(&at);
    for y in 0..height {
        let v = (place.1 + (y as f32 + 0.5) / height as f32 * drawn.1 - origin.1) / size.1;
        for x in 0..width {
            let u = (place.0 + (x as f32 + 0.5) / width as f32 * drawn.0 - origin.0) / size.0;
            // A box of no size gives no number, which lies outside it.
            let alpha = if inside(u) && inside(v) {
                mask.alpha(u, v).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let at = (y * width + x) * 4 + 3;
            bytes[at] = (f32::from(bytes[at]) * alpha).round() as u8;
        }
    }
    let faded = RgbaImage::from_raw(wide, high, bytes);
    let frame = Frame::new(faded.expect("as many bytes as before"));
    Arc::new(RenderImage::new(vec![frame]))
}

/// The masked picture for this frame, the kept one while nothing that makes it changed.
pub(super) fn masked(
    image: &Arc<RenderImage>,
    settings: &Settings,
    place: (f32, f32),
    drawn: (f32, f32),
    container: (f32, f32),
    window: &mut Window,
) -> Arc<RenderImage> {
    let mask_box = mask_box(settings, container);
    let (origin, size) = mask_box;
    let scale = window.scale_factor();
    let key = Key {
        image: image.id,
        angle: settings.background_mask_angle.to_bits(),
        stops: settings.background_mask_stops.clone(),
        lengths: [
            place.0,
            place.1,
            drawn.0,
            drawn.1,
            container.0,
            container.1,
            origin.0,
            origin.1,
            size.0,
            size.1,
            scale,
        ]
        .map(f32::to_bits),
    };
    let kept = KEPT.with(|kept| kept.borrow().clone());
    if let Some((_, picture)) = kept.filter(|(held, _)| *held == key) {
        return picture;
    }
    let (angle, stops) = (
        settings.background_mask_angle,
        &settings.background_mask_stops,
    );
    let mask = Mask::new(angle, stops, size).expect("the settings checked the mask");
    let picture = fade(image, &mask, mask_box, place, drawn, scale);
    log::debug!(
        "background: masked {:?} for a box of {container:?}",
        image.size(0)
    );
    forget(window);
    KEPT.with(|kept| kept.replace(Some((key, picture.clone()))));
    picture
}

/// Drops the kept masked picture and its texture, which gpui keeps until told.
pub(super) fn forget(window: &mut Window) {
    if let Some((_, stale)) = KEPT.with(RefCell::take)
        && let Err(error) = window.drop_image(stale)
    {
        log::error!("background: a masked picture's texture did not drop: {error:#}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 2 by 2 picture, every pixel opaque.
    fn picture() -> Arc<RenderImage> {
        let mut pixels = Vec::new();
        for _ in 0..4 {
            pixels.extend_from_slice(&[10, 20, 30, 255]);
        }
        let buffer = RgbaImage::from_raw(2, 2, pixels).expect("four pixels");
        Arc::new(RenderImage::new(vec![Frame::new(buffer)]))
    }

    fn faded(mask_box: ((f32, f32), (f32, f32))) -> Arc<RenderImage> {
        let mask = Mask::new(90.0, "transparent, black", mask_box.1).expect("two stops");
        fade(&picture(), &mask, mask_box, (0.0, 0.0), (2.0, 2.0), 1.0)
    }

    #[test]
    fn a_picture_stretches_no_further_than_memory_bears() {
        assert_eq!(fine((640, 400), (1200.0, 722.0), 2.0), (2400, 1444));
        assert_eq!(fine((640, 400), (300.0, 200.0), 1.0), (640, 400));
        assert_eq!(fine((8, 8), (9000.0, 10.0), 1.0), (4096, 10));
        let strip = (1, 1_000_000);
        assert_eq!(fine(strip, (1024.0, 768.0), 1.0), strip, "a strip stays");
        assert_eq!(fine((8000, 8000), (9000.0, 9000.0), 1.0), (8000, 8000));
    }

    #[test]
    fn a_picture_drawn_larger_fades_by_the_pixels_it_is_drawn_in() {
        let dot = RgbaImage::from_raw(1, 1, vec![10, 20, 30, 255]).expect("a pixel");
        let dot = RenderImage::new(vec![Frame::new(dot)]);
        let mask_box = ((0.0, 0.0), (4.0, 1.0));
        let mask = Mask::new(90.0, "transparent, black", mask_box.1).expect("two stops");
        let faded = fade(&dot, &mask, mask_box, (0.0, 0.0), (4.0, 1.0), 1.0);
        let bytes = faded.as_bytes(0).expect("a frame");
        let alphas: Vec<u8> = bytes.chunks(4).map(|pixel| pixel[3]).collect();
        assert_eq!(alphas, [32, 96, 159, 223]);
    }

    #[test]
    fn the_mask_fades_the_pictures_alpha() {
        let faded = faded(((0.0, 0.0), (2.0, 2.0)));
        let bytes = faded.as_bytes(0).expect("a frame");
        assert_eq!(bytes[3], 64, "the left is a quarter through the stops");
        assert_eq!(bytes[7], 191, "the right is three quarters");
        assert_eq!(bytes[..3], [10, 20, 30], "the colors stay");
    }

    #[test]
    fn a_mask_box_leaves_the_picture_outside_it_clear() {
        let faded = faded(((0.0, 0.0), (1.0, 2.0)));
        let bytes = faded.as_bytes(0).expect("a frame");
        // 90 degrees' cosine is a hair off 0, so half of 255 rounds either way.
        assert!(
            bytes[3].abs_diff(128) <= 1,
            "the left pixel is halfway through the box"
        );
        assert_eq!(bytes[7], 0, "the right lies outside the mask box");
        let none = self::faded(((0.0, 0.0), (0.0, 0.0)));
        let bytes = none.as_bytes(0).expect("a frame");
        assert_eq!([bytes[3], bytes[7]], [0, 0], "a box of no size masks all");
    }

    #[test]
    fn a_mask_box_keeps_its_place_and_size() {
        let settings = Settings::default();
        let box_of = |size: &str, place: &str| {
            let mut settings = settings.clone();
            settings.background_mask_size = size.parse().expect("a size");
            settings.background_mask_position = place.parse().expect("a place");
            mask_box(&settings, (200.0, 100.0))
        };
        assert_eq!(box_of("cover", "center"), ((0.0, 0.0), (200.0, 100.0)));
        assert_eq!(
            box_of("40% 100%", "right top"),
            ((120.0, 0.0), (80.0, 100.0))
        );
        assert_eq!(box_of("50% 50%", "center"), ((50.0, 25.0), (100.0, 50.0)));
    }
}
