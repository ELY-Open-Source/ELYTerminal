use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};

use gpui::{
    AnyElement, App, Asset, ImageSource, ImgResourceLoader, IntoElement, ObjectFit, ParentElement,
    Pixels, RenderImage, Styled, StyledImage, Window, div, img,
};
use image::{Frame, RgbaImage};

use super::geo::Laid;
use crate::{
    primitives::{Icon, IconName},
    theme::ActiveTheme,
};

/// A decoded tile; two are alike when they hold the same picture.
#[derive(Clone)]
pub(crate) struct Rim(pub Arc<RenderImage>);

impl Hash for Rim {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.id.hash(state);
    }
}

/// A tile with a one-pixel rim copied from its own edge, made off the main thread. gpui packs pictures in its atlas without padding and scales them with a linear filter, so a bare tile's edge takes color from its atlas neighbor.
pub(crate) enum Rimmed {}

impl Asset for Rimmed {
    type Source = Rim;
    type Output = Arc<RenderImage>;

    fn load(
        source: Self::Source,
        _: &mut App,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let Rim(image) = source;
        async move { Arc::new(rimmed(&image)) }
    }
}

/// Every frame of `image` one pixel larger each side, the rim repeating the edge beside it.
pub(crate) fn rimmed(image: &RenderImage) -> RenderImage {
    let frames: Vec<Frame> = (0..image.frame_count())
        .map(|ix| {
            let size = image.size(ix);
            let (w, h) = (size.width.0 as u32, size.height.0 as u32);
            let bytes = image.as_bytes(ix).expect("a frame in range").to_vec();
            let inner = RgbaImage::from_raw(w, h, bytes).expect("a frame's bytes fill its size");
            let outer = RgbaImage::from_fn(w + 2, h + 2, |x, y| {
                *inner.get_pixel(
                    x.saturating_sub(1).min(w - 1),
                    y.saturating_sub(1).min(h - 1),
                )
            });
            Frame::from_parts(outer, 0, 0, image.delay(ix))
        })
        .collect();
    RenderImage::new(frames)
}

/// A tile's picture in its box: its rim lies one pixel past each side, clipped, so the box shows its own pixels alone. Nothing while it loads; a broken-picture mark, and gpui's log line, when it cannot.
pub(crate) fn laid_tile(
    laid: &Laid,
    source: ImageSource,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let slot = div()
        .absolute()
        .left(Pixels::from(laid.x))
        .top(Pixels::from(laid.y))
        .w(Pixels::from(laid.w))
        .h(Pixels::from(laid.h))
        .overflow_hidden();
    let decoded = match source {
        ImageSource::Render(image) => Some(Ok(image)),
        ImageSource::Resource(resource) => window.use_asset::<ImgResourceLoader>(&resource, cx),
        _ => panic!(
            "map tile {:?}: a web address, a file or a picture drawn in code",
            laid.tile
        ),
    };
    let image = match decoded {
        None => return slot.into_any_element(),
        Some(Err(_)) => {
            let mark = cx.theme().colors.fg_subtle;
            return slot
                .flex()
                .items_center()
                .justify_center()
                .child(Icon::new(IconName::ImageOff).color(mark))
                .into_any_element();
        }
        Some(Ok(image)) => image,
    };
    let Some(rimmed) = window.use_asset::<Rimmed>(&Rim(image.clone()), cx) else {
        return slot.into_any_element();
    };
    let size = image.size(0);
    let (dx, dy) = (laid.w / size.width.0 as f32, laid.h / size.height.0 as f32);
    slot.child(
        img(ImageSource::Render(rimmed))
            .absolute()
            .left(Pixels::from(-dx))
            .top(Pixels::from(-dy))
            .w(Pixels::from(laid.w + dx * 2.0))
            .h(Pixels::from(laid.h + dy * 2.0))
            .object_fit(ObjectFit::Fill),
    )
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use gpui::RenderImage;
    use image::{Frame, RgbaImage};

    use super::rimmed;

    #[test]
    fn a_rim_repeats_the_edge_beside_it() {
        let mut wide = RgbaImage::new(2, 1);
        wide.get_pixel_mut(0, 0).0 = [1, 0, 0, 255];
        wide.get_pixel_mut(1, 0).0 = [2, 0, 0, 255];
        let rimmed = rimmed(&RenderImage::new(vec![Frame::new(wide)]));
        let size = rimmed.size(0);
        assert_eq!((size.width.0, size.height.0), (4, 3));
        let bytes = rimmed.as_bytes(0).expect("a frame");
        let first = |row: usize| {
            bytes[row * 16..row * 16 + 16]
                .iter()
                .step_by(4)
                .copied()
                .collect::<Vec<_>>()
        };
        assert_eq!(first(0), [1, 1, 2, 2], "the top rim repeats the row below");
        assert_eq!(first(1), [1, 1, 2, 2], "each side repeats its edge pixel");
        assert_eq!(first(2), [1, 1, 2, 2]);
    }
}
