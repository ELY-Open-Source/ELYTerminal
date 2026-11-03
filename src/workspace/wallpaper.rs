//! The background picture, where the settings put it.

use std::{
    cell::{Cell, RefCell},
    path::{Path, PathBuf},
    sync::Arc,
};

use gpui::{
    AnyElement, App, Bounds, Hsla, ImageAssetLoader, IntoElement, ObjectFit, ParentElement, Pixels,
    RenderImage, Resource, Size, Styled, StyledImage, Window, canvas, div, img, point, px, size,
};

use crate::settings::{BackgroundScope, PictureRepeat, Settings};

/// The most tiles one picture draws; past it the picture draws once.
const MAX_TILES: usize = 400;

thread_local! {
    /// The terminal body's bounds, as the last frame laid them.
    static TERMINAL: Cell<Option<Bounds<Pixels>>> = const { Cell::new(None) };
    /// The picture whose failure was logged last, so each frame does not log it again.
    static FAILED: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

/// The picture's bounds: the terminal body, or the window below the title bar.
fn box_of(scope: BackgroundScope, body: Bounds<Pixels>, viewport: Size<Pixels>) -> Bounds<Pixels> {
    match scope {
        BackgroundScope::Terminal => body,
        BackgroundScope::Window => Bounds::new(
            point(px(0.0), body.origin.y),
            size(viewport.width, viewport.height - body.origin.y),
        ),
    }
}

/// A canvas that records the terminal body's bounds, so the window's picture starts below the title bar.
pub(super) fn ruler() -> impl IntoElement {
    canvas(
        |bounds, window, _: &mut App| {
            if TERMINAL.get() != Some(bounds) {
                TERMINAL.set(Some(bounds));
                window.request_animation_frame();
            }
        },
        |_, (), _, _| {},
    )
    .absolute()
    .inset_0()
}

/// The window's ground and the picture over it, at the background's opacity; nothing without one.
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
    };
    let Some(body) = TERMINAL.get() else {
        return Some(ground().into_any_element());
    };
    let bounds = box_of(settings.background_scope, body, window.viewport_size());
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
        return Some(ground().into_any_element());
    }
    let container = (f32::from(bounds.size.width), f32::from(bounds.size.height));
    let drawn = settings.background_size.points(intrinsic, container);
    let free = (container.0 - drawn.0, container.1 - drawn.1);
    let place = settings.background_position.points(free);
    let mut boxed = div()
        .absolute()
        .left(bounds.origin.x)
        .top(bounds.origin.y)
        .w(bounds.size.width)
        .h(bounds.size.height);
    for (x, y) in tiles(settings.background_repeat, place, drawn, container) {
        boxed = boxed.child(tile(&image, x, y, drawn));
    }
    Some(ground().child(boxed).into_any_element())
}

/// One tile of the picture, at `x` and `y` in its box.
fn tile(image: &Arc<RenderImage>, x: f32, y: f32, drawn: (f32, f32)) -> impl IntoElement {
    img(image.clone())
        .absolute()
        .left(px(x))
        .top(px(y))
        .w(px(drawn.0))
        .h(px(drawn.1))
        .object_fit(ObjectFit::Fill)
}

/// Where each tile starts, from the picture's place in its box; past `MAX_TILES`, the first one alone.
fn tiles(
    repeat: PictureRepeat,
    place: (f32, f32),
    drawn: (f32, f32),
    container: (f32, f32),
) -> Vec<(f32, f32)> {
    let xs = if matches!(repeat, PictureRepeat::Repeat | PictureRepeat::RepeatX) {
        along(place.0, drawn.0, container.0)
    } else {
        vec![place.0]
    };
    let ys = if matches!(repeat, PictureRepeat::Repeat | PictureRepeat::RepeatY) {
        along(place.1, drawn.1, container.1)
    } else {
        vec![place.1]
    };
    if xs.len() * ys.len() > MAX_TILES {
        return vec![place];
    }
    let mut tiles = Vec::with_capacity(xs.len() * ys.len());
    for y in ys {
        for x in &xs {
            tiles.push((*x, y));
        }
    }
    tiles
}

/// The starts of one repeated tile across `extent`, from `at`.
fn along(at: f32, step: f32, extent: f32) -> Vec<f32> {
    if step <= 0.0 {
        return vec![at];
    }
    let mut first = at.rem_euclid(step);
    if first > 0.0 {
        first -= step;
    }
    let mut starts = Vec::new();
    let mut start = first;
    while start < extent && starts.len() < MAX_TILES {
        starts.push(start);
        start += step;
    }
    starts
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
        let bounds = box_of(
            BackgroundScope::Terminal,
            body(),
            size(px(1200.0), px(700.0)),
        );
        assert_eq!(bounds, body());
    }

    #[test]
    fn the_window_scope_starts_below_the_title_bar_and_spans_the_window() {
        let bounds = box_of(BackgroundScope::Window, body(), size(px(1200.0), px(700.0)));
        assert_eq!(bounds.origin, point(px(0.0), px(38.0)));
        assert_eq!(bounds.size, size(px(1200.0), px(662.0)));
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
    }
}
