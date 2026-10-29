//! Placements as the view draws them: under, between, over text.

use std::{collections::HashMap, sync::Arc};

use gpui::{Bounds, RenderImage, point, size};
use image::{Frame, RgbaImage};

use super::{
    Shared,
    place::{Kind, Placement, Screen},
    store::Image,
    unicode::Run,
};

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

/// Textures by what each image shows, built when first shown.
#[derive(Default)]
pub struct Pictures {
    built: HashMap<u64, Arc<RenderImage>>,
}

impl Pictures {
    /// What shows, scrolled up `offset`, and textures whose image went.
    pub fn shown(
        &mut self,
        shared: &Shared,
        runs: &[Run],
        offset: usize,
        (columns, lines): (usize, usize),
    ) -> (Vec<Shown>, Vec<Arc<RenderImage>>) {
        let screen = &shared.screens[usize::from(shared.alt)];
        let cell = shared.cell;
        let grid = Bounds::new(point(0., 0.), size(columns as f32, lines as f32));
        let mut drawn = Vec::new();
        // A virtual placement's top-left: its placeholders' least column and row.
        let mut corners: HashMap<u64, (f32, f32)> = HashMap::new();
        for run in runs {
            let Some(target) = target(screen, run) else {
                continue;
            };
            let Some(image) = screen.store.get(target.image) else {
                continue;
            };
            let corner = corners.entry(target.serial).or_insert((f32::MAX, f32::MAX));
            *corner = (
                corner.0.min(run.column as f32),
                corner.1.min(run.row as f32),
            );
            let cells = size(run.width as f32, 1.);
            let clip = Bounds::new(point(run.column as f32, run.row as f32), cells);
            drawn.push((
                target,
                image,
                fitted(target, image, cell, run),
                clip.intersect(&grid),
            ));
        }
        for kept in &screen.placements {
            let (Some(image), Some((root, columns, lines))) =
                (screen.store.get(kept.image), screen.root(kept))
            else {
                continue;
            };
            let corner = match root.kind {
                _ if kept.kind == Kind::Virtual => continue,
                Kind::Grid => (root.column as f32, (root.line + offset as i32) as f32),
                Kind::Virtual => match corners.get(&root.serial) {
                    Some(&corner) => corner,
                    None => continue,
                },
                Kind::Relative { .. } => unreachable!("a root is no relative placement"),
            };
            let corner = (corner.0 + columns as f32, corner.1 + lines as f32);
            if let Some((whole, clip)) = placed(kept, image, cell, corner) {
                drawn.push((kept, image, whole, clip.intersect(&grid)));
            }
        }
        drawn.sort_by_key(|(kept, image, ..)| {
            let z = kept.display.z;
            (Layer::of(z), z, image.id, kept.serial)
        });
        let mut shown = Vec::new();
        for (kept, image, whole, clip) in drawn {
            if clip.size.width <= 0. || clip.size.height <= 0. {
                continue;
            }
            let picture = self
                .built
                .entry(image.version)
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
            .flat_map(|screen| screen.store.images().map(|image| image.version))
            .collect();
        let mut stale = Vec::new();
        self.built.retain(|version, picture| {
            let keep = alive.contains(version);
            if !keep {
                stale.push(Arc::clone(picture));
            }
            keep
        });
        (shown, stale)
    }

    /// Every texture built, for a session hidden or closed.
    pub fn release(&mut self) -> Vec<Arc<RenderImage>> {
        self.built.drain().map(|(_, picture)| picture).collect()
    }
}

/// The whole image and the cells it fills, from `corner`.
fn placed(
    kept: &Placement,
    image: &Image,
    cell: (u32, u32),
    corner: (f32, f32),
) -> Option<(Bounds<f32>, Bounds<f32>)> {
    let (width, height) = (cell.0.max(1) as f32, cell.1.max(1) as f32);
    let [x, y, w, h] = kept.source(image);
    let (wide, tall) = kept.size(image, cell);
    if w <= 0. || h <= 0. || wide == 0 || tall == 0 {
        return None;
    }
    let (ox, oy) = kept.offset(cell);
    let corner = point(corner.0 + ox as f32 / width, corner.1 + oy as f32 / height);
    let area = size(wide as f32 / width, tall as f32 / height);
    let (kx, ky) = (area.width / w, area.height / h);
    let whole = Bounds::new(
        point(corner.x - x * kx, corner.y - y * ky),
        size(image.width as f32 * kx, image.height as f32 * ky),
    );
    Some((whole, Bounds::new(corner, area)))
}

/// A run's placement: its id's, else the oldest virtual one.
fn target<'a>(screen: &'a Screen, run: &Run) -> Option<&'a Placement> {
    let mut of = screen
        .placements
        .iter()
        .filter(|kept| kept.image == run.image);
    match run.placement {
        0 => of
            .filter(|kept| kept.kind == Kind::Virtual)
            .min_by_key(|kept| kept.serial),
        id => of.find(|kept| kept.id == id),
    }
}

/// The whole image `run` shows, fitted as Ghostty fits.
fn fitted(target: &Placement, image: &Image, cell: (u32, u32), run: &Run) -> Bounds<f32> {
    let (width, height) = (cell.0.max(1) as f32, cell.1.max(1) as f32);
    let (w, h) = (image.width as f32, image.height as f32);
    let given = |cells: u32, length: f32, side: f32| match cells {
        0 => (length / side).ceil(),
        cells => cells as f32,
    };
    let (columns, rows) = target.display.cells;
    let (wide, tall) = (
        given(columns, w, width) * width,
        given(rows, h, height) * height,
    );
    // Whole in its cells, centred, its aspect kept.
    let scale = if w * tall > h * wide {
        wide / w
    } else {
        tall / h
    };
    let (fw, fh) = (w * scale, h * scale);
    let x = run.column as f32 - run.at.1 as f32 + (wide - fw) / 2. / width;
    let y = run.row as f32 - run.at.0 as f32 + (tall - fh) / 2. / height;
    Bounds::new(point(x, y), size(fw / width, fh / height))
}

/// The image as gpui takes it, BGRA.
fn texture(image: &Image) -> RenderImage {
    let mut pixels = image.shown().to_vec();
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    let buffer = RgbaImage::from_raw(image.width, image.height, pixels)
        .expect("an image's pixels fill its size");
    RenderImage::new([Frame::new(buffer)])
}
