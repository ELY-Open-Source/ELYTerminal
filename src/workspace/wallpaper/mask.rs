//! The picture's mask: its alpha in the picture's straight BGRA bytes.

use std::{cell::RefCell, sync::Arc};

use gpui::{ImageId, RenderImage, Window};
use image::{Frame, RgbaImage};

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
    lengths: [u32; 10],
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
) -> Arc<RenderImage> {
    let pixels = image.size(0);
    let (width, height) = (pixels.width.0 as usize, pixels.height.0 as usize);
    let mut bytes = image.as_bytes(0).expect("a picture's first frame").to_vec();
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
    let faded = RgbaImage::from_raw(width as u32, height as u32, bytes);
    let frame = Frame::new(faded.expect("as many bytes as the picture held"));
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
        ]
        .map(f32::to_bits),
    };
    let kept = KEPT.with(|kept| kept.borrow().clone());
    if let Some((_, picture)) = kept.filter(|(held, _)| *held == key) {
        return picture;
    }
    let mask = Mask::new(
        settings.background_mask_angle,
        &settings.background_mask_stops,
    )
    .expect("the settings checked the mask");
    let picture = fade(image, &mask, mask_box, place, drawn);
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
        let mask = Mask::new(90.0, "transparent, black").expect("two stops");
        fade(&picture(), &mask, mask_box, (0.0, 0.0), (2.0, 2.0))
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
        assert_eq!(bytes[3], 128, "the left pixel is halfway through the box");
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
