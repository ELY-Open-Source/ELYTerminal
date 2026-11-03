use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};

use gpui::{App, Asset, RenderImage};
use image::{
    Frame, RgbaImage,
    imageops::{rotate90, rotate180, rotate270},
};

/// A decoded picture and how many quarter turns clockwise to give it; two are alike when they turn the same picture as far.
#[derive(Clone)]
pub(crate) struct Turn(pub Arc<RenderImage>, pub u8);

impl Hash for Turn {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.id.hash(state);
        (self.1 % 4).hash(state);
    }
}

/// Turns a decoded picture off the main thread; gpui keeps each turn once made.
pub(crate) enum Turned {}

impl Asset for Turned {
    type Source = Turn;
    type Output = Arc<RenderImage>;

    fn load(
        source: Self::Source,
        _: &mut App,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let Turn(image, quarters) = source;
        async move { Arc::new(turn(&image, quarters)) }
    }
}

/// Every frame of `image` turned `quarters` quarter turns clockwise, each keeping its delay.
pub(crate) fn turn(image: &RenderImage, quarters: u8) -> RenderImage {
    let frames: Vec<Frame> = (0..image.frame_count())
        .map(|ix| {
            let size = image.size(ix);
            let bytes = image.as_bytes(ix).expect("a frame in range").to_vec();
            let buffer = RgbaImage::from_raw(size.width.0 as u32, size.height.0 as u32, bytes)
                .expect("a frame's bytes fill its size");
            let turned = match quarters % 4 {
                1 => rotate90(&buffer),
                2 => rotate180(&buffer),
                3 => rotate270(&buffer),
                _ => buffer,
            };
            Frame::from_parts(turned, 0, 0, image.delay(ix))
        })
        .collect();
    RenderImage::new(frames)
}

#[cfg(test)]
mod tests {
    use gpui::RenderImage;
    use image::{Frame, RgbaImage};

    use super::turn;

    #[test]
    fn a_quarter_turn_swaps_the_sides_and_carries_each_pixel_round() {
        let mut wide = RgbaImage::new(2, 1);
        wide.get_pixel_mut(0, 0).0 = [1, 0, 0, 255];
        wide.get_pixel_mut(1, 0).0 = [2, 0, 0, 255];
        let image = RenderImage::new(vec![Frame::new(wide)]);
        let turned = turn(&image, 1);
        let size = turned.size(0);
        assert_eq!((size.width.0, size.height.0), (1, 2));
        let bytes = turned.as_bytes(0).expect("a frame");
        assert_eq!(
            (bytes[0], bytes[4]),
            (1, 2),
            "the left pixel turns to the top"
        );
        let back = turn(&turned, 3);
        assert_eq!(
            back.as_bytes(0),
            image.as_bytes(0),
            "three more quarters bring it home"
        );
    }
}
