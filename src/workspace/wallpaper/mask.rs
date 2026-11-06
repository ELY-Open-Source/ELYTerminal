//! The picture's mask: its alpha in the picture's straight BGRA bytes.

use std::{cell::RefCell, collections::HashMap, sync::Arc};

use gpui::RenderImage;
use image::{Frame, RgbaImage};

use crate::settings::{Mask, Settings};

/// The most masked copies kept, each a whole picture's pixels.
const MASKED: usize = 4;

thread_local! {
    /// The masked pictures by their image, mask and box, so each frame does not mask again.
    static MASKED_IMAGES: RefCell<HashMap<MaskKey, Arc<RenderImage>>> = RefCell::new(HashMap::new());
}

/// What makes a masked copy: the image, the mask, and the box it covers.
#[derive(Clone, PartialEq, Eq, Hash)]
struct MaskKey {
    image: u64,
    angle: u32,
    stops: String,
    box_of: [u32; 6],
    mask_box: [u32; 4],
}

/// The mask's box in the picture's box: its size, then its place.
pub(super) fn mask_box(settings: &Settings, container: (f32, f32)) -> ((f32, f32), (f32, f32)) {
    let size = settings.background_mask_size.points(container, container);
    let free = (container.0 - size.0, container.1 - size.1);
    (settings.background_mask_position.points(free), size)
}

/// The picture with its mask as alpha, its straight BGRA bytes scaled; a copy kept by image, mask and box.
pub(super) fn masked(
    image: &Arc<RenderImage>,
    mask: &Mask,
    stops: &str,
    mask_box: ((f32, f32), (f32, f32)),
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
    let (origin, size) = mask_box;
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
        mask_box: [
            origin.0.to_bits(),
            origin.1.to_bits(),
            size.0.to_bits(),
            size.1.to_bits(),
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
            let center = (
                place.0 + (x as f32 + 0.5) / width as f32 * drawn.0,
                place.1 + (y as f32 + 0.5) / height as f32 * drawn.1,
            );
            let alpha = if size.0 > 0.0 && size.1 > 0.0 {
                let u = (center.0 - origin.0) / size.0;
                let v = (center.1 - origin.1) / size.1;
                if (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v) {
                    mask.alpha(u, v).clamp(0.0, 1.0)
                } else {
                    0.0
                }
            } else {
                0.0
            };
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

    #[test]
    fn the_mask_fades_the_pictures_alpha() {
        let image = picture();
        let mask = Mask::new(90.0, "transparent, black").expect("two stops");
        let faded = masked(
            &image,
            &mask,
            "transparent, black",
            ((0.0, 0.0), (2.0, 2.0)),
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
            ((0.0, 0.0), (2.0, 2.0)),
            (0.0, 0.0),
            (2.0, 2.0),
            (2.0, 2.0),
        )
        .expect("the same copy");
        assert!(Arc::ptr_eq(&faded, &kept), "a frame masks once");
    }

    #[test]
    fn a_mask_box_leaves_the_picture_outside_it_clear() {
        let image = picture();
        let mask = Mask::new(90.0, "transparent, black").expect("two stops");
        let faded = masked(
            &image,
            &mask,
            "transparent, black",
            ((0.0, 0.0), (1.0, 2.0)),
            (0.0, 0.0),
            (2.0, 2.0),
            (2.0, 2.0),
        )
        .expect("a masked copy");
        let bytes = faded.as_bytes(0).expect("a frame");
        assert_eq!(bytes[3], 128, "the left pixel is halfway through the box");
        assert_eq!(bytes[7], 0, "the right lies outside the mask box");
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
