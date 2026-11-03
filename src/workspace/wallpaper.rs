//! The background picture, where the settings put it.

use std::{
    cell::{Cell, RefCell},
    path::{Path, PathBuf},
    sync::Arc,
};

use gpui::{
    AnyElement, App, Bounds, Hsla, ImageAssetLoader, IntoElement, ObjectFit, ParentElement, Pixels,
    Resource, Size, Styled, StyledImage, Window, canvas, div, img, point, px, size,
};

use crate::settings::{BackgroundScope, Settings};

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
    let ground = div()
        .absolute()
        .inset_0()
        .opacity(settings.background_opacity)
        .child(div().absolute().inset_0().bg(base));
    let Some(body) = TERMINAL.get() else {
        return Some(ground.into_any_element());
    };
    let bounds = box_of(settings.background_scope, body, window.viewport_size());
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
}
