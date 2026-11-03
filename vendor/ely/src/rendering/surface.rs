use std::{rc::Rc, sync::Arc};

use gpui::{
    App, Bounds, Corners, DevicePixels, ElementId, IntoElement, ParentElement, Pixels, RenderImage,
    RenderOnce, Size, Styled, Window, canvas, prelude::*,
};
use web_time::Instant;

use crate::primitives::{checked_ratio, framed};

type Draw = Rc<dyn Fn(Size<DevicePixels>, &mut App) -> RenderImage>;

/// The frame on screen and what it was drawn for.
struct Drawn {
    size: Size<DevicePixels>,
    revision: u64,
    frame: Arc<RenderImage>,
}

/// Pixels the host draws, one frame at the box's size in device pixels, in the shape `ratio` gives it. It draws again when that size or the host's revision changes, and frees the old frame from the atlas. A frame is BGRA, as gpui's `RenderImage` holds it.
#[derive(IntoElement)]
pub struct RenderSurface {
    id: ElementId,
    ratio: f32,
    revision: u64,
    draw: Draw,
}

impl RenderSurface {
    /// `ratio` is the box's width over its height; `draw` makes a frame of exactly the size it is given.
    pub fn new(
        id: impl Into<ElementId>,
        ratio: f32,
        draw: impl Fn(Size<DevicePixels>, &mut App) -> RenderImage + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            ratio: checked_ratio(ratio),
            revision: 0,
            draw: Rc::new(draw),
        }
    }

    /// Bumped by the host whenever the content changes.
    pub fn revision(mut self, revision: u64) -> Self {
        self.revision = revision;
        self
    }
}

impl RenderOnce for RenderSurface {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Freed from every atlas on leave.
        let shown = window.use_keyed_state((self.id.clone(), "frame"), cx, |_, cx| {
            cx.on_release(|shown: &mut Option<Drawn>, cx| {
                if let Some(drawn) = shown.take() {
                    cx.drop_image(drawn.frame, None);
                }
            })
            .detach();
            None
        });
        let (id, revision, draw) = (self.id.clone(), self.revision, self.draw);
        let painted = self.id.clone();
        framed(self.ratio, cx)
            .id(self.id)
            .debug_selector(|| "render-surface".into())
            .child(
                canvas(
                    move |bounds, window, cx| {
                        let size = bounds.size.to_device_pixels(window.scale_factor());
                        if size.width.0 == 0 || size.height.0 == 0 {
                            return None;
                        }
                        let stale = shown
                            .read(cx)
                            .as_ref()
                            .is_none_or(|drawn| drawn.size != size || drawn.revision != revision);
                        if stale {
                            let frame = drawn(&id, size, revision, &draw, cx);
                            let old = shown.update(cx, |shown, _| {
                                shown.replace(Drawn {
                                    size,
                                    revision,
                                    frame,
                                })
                            });
                            if let Some(old) = old
                                && let Err(error) = window.drop_image(old.frame)
                            {
                                log::error!("render surface {id}: the atlas kept a frame: {error}");
                            }
                        }
                        shown.read(cx).as_ref().map(|drawn| drawn.frame.clone())
                    },
                    move |bounds: Bounds<Pixels>, frame, window, _| {
                        if let Some(frame) = frame
                            && let Err(error) = window.paint_image(
                                bounds,
                                bounds,
                                Corners::default(),
                                frame,
                                0,
                                false,
                            )
                        {
                            log::error!(
                                "render surface {painted}: the frame did not paint: {error}"
                            );
                        }
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
    }
}

/// The host's frame for `size`, which must match it.
fn drawn(
    id: &ElementId,
    size: Size<DevicePixels>,
    revision: u64,
    draw: &Draw,
    cx: &mut App,
) -> Arc<RenderImage> {
    let start = Instant::now();
    let frame = draw(size, cx);
    assert_eq!(
        frame.frame_count(),
        1,
        "render surface {id}: a frame is one picture"
    );
    assert_eq!(
        frame.size(0),
        size,
        "render surface {id}: the host drew the wrong size"
    );
    log::debug!(
        "render surface {id}: drew {}x{} at revision {revision} in {:?}",
        size.width.0,
        size.height.0,
        start.elapsed()
    );
    Arc::new(frame)
}
