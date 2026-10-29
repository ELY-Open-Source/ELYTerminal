//! Placements as the view draws them: under, between, over text.

use std::{collections::HashMap, sync::Arc};

use gpui::{Bounds, RenderImage, point, size};
use image::{Frame, RgbaImage};

use super::{Shared, store::Image};

/// Where a placement draws, by its z-index, as kitty layers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    /// Under every cell's ground.
    Under,
    /// Over the grounds, under the text.
    Between,
    Over,
}

impl Layer {
    fn of(z: i32) -> Self {
        match z {
            z if z < i32::MIN / 2 => Self::Under,
            z if z < 0 => Self::Between,
            _ => Self::Over,
        }
    }
}

/// A placement to paint, in cells from the grid's top-left.
#[derive(Clone)]
pub struct Shown {
    pub layer: Layer,
    pub picture: Arc<RenderImage>,
    /// The whole image, scaled as the placement shows its source.
    pub whole: Bounds<f32>,
    /// The part shown: the placement on the screen.
    pub clip: Bounds<f32>,
}

/// Textures by image generation, built when first shown.
#[derive(Default)]
pub struct Pictures {
    built: HashMap<u64, Arc<RenderImage>>,
}

impl Pictures {
    /// What shows, scrolled up `offset`, and textures whose image went.
    pub fn shown(
        &mut self,
        shared: &Shared,
        offset: usize,
        (columns, lines): (usize, usize),
    ) -> (Vec<Shown>, Vec<Arc<RenderImage>>) {
        let screen = &shared.screens[usize::from(shared.alt)];
        let cell = shared.cell;
        let (width, height) = (cell.0.max(1) as f32, cell.1.max(1) as f32);
        let grid = Bounds::new(point(0., 0.), size(columns as f32, lines as f32));
        let mut placed: Vec<_> = screen
            .placements
            .iter()
            .filter_map(|kept| Some((kept, screen.store.get(kept.image)?)))
            .collect();
        placed.sort_by_key(|(kept, image)| {
            let z = kept.display.z;
            (Layer::of(z), z, image.id, kept.serial)
        });
        let mut shown = Vec::new();
        for (kept, image) in placed {
            let [x, y, w, h] = kept.source(image);
            let (wide, tall) = kept.size(image, cell);
            if w <= 0. || h <= 0. || wide == 0 || tall == 0 {
                continue;
            }
            let (ox, oy) = kept.offset(cell);
            let row = (kept.line + offset as i32) as f32;
            let corner = point(
                kept.column as f32 + ox as f32 / width,
                row + oy as f32 / height,
            );
            let area = size(wide as f32 / width, tall as f32 / height);
            let (kx, ky) = (area.width / w, area.height / h);
            let whole = Bounds::new(
                point(corner.x - x * kx, corner.y - y * ky),
                size(image.width as f32 * kx, image.height as f32 * ky),
            );
            let clip = Bounds::new(corner, area).intersect(&grid);
            if clip.size.width <= 0. || clip.size.height <= 0. {
                continue;
            }
            let picture = self
                .built
                .entry(image.generation)
                .or_insert_with(|| Arc::new(texture(image)))
                .clone();
            let layer = Layer::of(kept.display.z);
            shown.push(Shown {
                layer,
                picture,
                whole,
                clip,
            });
        }
        let alive: Vec<u64> = shared
            .screens
            .iter()
            .flat_map(|screen| screen.store.images().map(|image| image.generation))
            .collect();
        let mut stale = Vec::new();
        self.built.retain(|generation, picture| {
            let keep = alive.contains(generation);
            if !keep {
                stale.push(Arc::clone(picture));
            }
            keep
        });
        (shown, stale)
    }
}

/// The image as gpui takes it, BGRA.
fn texture(image: &Image) -> RenderImage {
    let mut pixels = image.pixels.clone();
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    let buffer = RgbaImage::from_raw(image.width, image.height, pixels)
        .expect("an image's pixels fill its size");
    RenderImage::new([Frame::new(buffer)])
}
