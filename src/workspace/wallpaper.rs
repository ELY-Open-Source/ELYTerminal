//! The background picture, where the settings put it.

use std::{
    cell::{Cell, RefCell},
    path::{Path, PathBuf},
    sync::Arc,
    thread::LocalKey,
};

use gpui::{
    AnyElement, App, Bounds, Hsla, ImageAssetLoader, IntoElement, ObjectFit, ParentElement, Pixels,
    Resource, Styled, StyledImage, Window, canvas, div, img, point, size,
};

use crate::settings::{BackgroundScope, Settings};

thread_local! {
    /// The terminal body's bounds, as the last frame laid them.
    static TERMINAL: Cell<Option<Bounds<Pixels>>> = const { Cell::new(None) };
    /// The box the picture paints in, inside Ely's resize margin.
    static GROUND: Cell<Option<Bounds<Pixels>>> = const { Cell::new(None) };
    /// The picture whose failure was logged last, so each frame does not log it again.
    static FAILED: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
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
    let ground = div()
        .absolute()
        .inset_0()
        .opacity(settings.background_opacity)
        .child(div().absolute().inset_0().bg(base))
        .child(measure(&GROUND));
    let (Some(body), Some(within)) = (TERMINAL.get(), GROUND.get()) else {
        return Some(ground.into_any_element());
    };
    let bounds = box_of(settings.background_scope, body, within);
    let path: Arc<Path> = Path::new(&settings.background_image).into();
    let source = Resource::Path(path.clone());
    let Some(loaded) = window.use_asset::<ImageAssetLoader>(&source, cx) else {
        return Some(ground.into_any_element());
    };
    match loaded {
        Ok(image) => Some(
            ground
                .child(
                    img(image)
                        .absolute()
                        .left(bounds.origin.x)
                        .top(bounds.origin.y)
                        .w(bounds.size.width)
                        .h(bounds.size.height)
                        .object_fit(ObjectFit::Cover),
                )
                .into_any_element(),
        ),
        Err(error) => {
            let fresh = FAILED.with(|failed| {
                let mut failed = failed.borrow_mut();
                if failed.as_deref() == Some(&*path) {
                    return false;
                }
                *failed = Some(path.to_path_buf());
                true
            });
            if fresh {
                log::error!("background: {}: {error}", path.display());
            }
            Some(ground.into_any_element())
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::px;

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
}
