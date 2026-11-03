use std::rc::Rc;

use gpui::{
    App, Bounds, CursorStyle, DragMoveEvent, ElementId, EmptyView, EntityId, ImageSource,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, Point, RenderOnce,
    StatefulInteractiveElement, Styled, Window, canvas, div, prelude::*, relative,
    transparent_black,
};

use super::crop::{Crop, Grip, dragged, grip_at};
use crate::{
    layout::seeded::use_seeded,
    primitives::{FocusRing, Image, checked_ratio, framed, tab_stop},
    theme::ActiveTheme,
};

pub(super) type OnCrop = Rc<dyn Fn(Crop, &mut Window, &mut App)>;

/// Least side of a crop, as a share of its picture.
const LEAST: f32 = 0.05;
/// What an arrow moves the crop, and Shift with it.
const NUDGE: (f32, f32) = (0.01, 0.1);

/// A crop's drag, marked with its cropper.
struct Hold(EntityId);

/// The stage's bounds, what the press holds, and where the pointer was last.
#[derive(Default)]
struct Stage {
    bounds: Bounds<Pixels>,
    held: Option<Grip>,
    from: Point<Pixels>,
}

/// A picture and a box to crop it by, the picture's shape from the host, inset by half a handle so handles on its edge stay whole. Drag the box to move it and its handles to size it; with an aspect, only its corners size it and it keeps that shape. The arrows move it a hundredth of the picture, a tenth with Shift. The host keeps the crop; `Crop::fitted` gives a box of a new aspect.
#[derive(IntoElement)]
pub struct ImageCropper {
    id: ElementId,
    source: ImageSource,
    ratio: f32,
    crop: Crop,
    aspect: Option<f32>,
    on_change: Option<OnCrop>,
}

impl ImageCropper {
    /// `ratio` is the picture's width over its height.
    pub fn new(
        id: impl Into<ElementId>,
        source: impl Into<ImageSource>,
        ratio: f32,
        crop: Crop,
    ) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
            ratio: checked_ratio(ratio),
            crop: crop.checked(),
            aspect: None,
            on_change: None,
        }
    }

    /// The crop's width over its height, in pixels; none leaves it free.
    pub fn aspect(mut self, aspect: Option<f32>) -> Self {
        self.aspect = aspect.map(checked_ratio);
        self
    }

    pub fn on_change(mut self, handler: impl Fn(Crop, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ImageCropper {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let crop = use_seeded((self.id.clone(), "crop"), self.crop, window, cx);
        let stage = window.use_keyed_state((self.id.clone(), "stage"), cx, |_, _| Stage::default());
        let editable = self.on_change.is_some();
        let focus = tab_stop((self.id.clone(), "focus").into(), editable, window, cx);
        let now = crop.read(cx).value;
        let shape = self.aspect.map(|aspect| aspect / self.ratio);
        let owner = crop.entity_id();
        let theme = cx.theme();
        let (veil, line) = (
            theme.colors.media_backdrop.alpha(0.55),
            theme.colors.on_media,
        );
        let side = theme.media().crop_handle;
        let hit = side.to_pixels(window.rem_size());
        let change = {
            let (crop, on_change) = (crop.clone(), self.on_change.clone());
            move |next: Crop, window: &mut Window, cx: &mut App| {
                crop.update(cx, |crop, cx| {
                    crop.value = next;
                    cx.notify();
                });
                if let Some(on_change) = &on_change {
                    on_change(next, window, cx);
                }
            }
        };
        let pane = |left: f32, top: f32, w: f32, h: f32| {
            div()
                .absolute()
                .left(relative(left))
                .top(relative(top))
                .w(relative(w))
                .h(relative(h))
                .bg(veil)
        };
        let (x2, y2) = (now.x + now.w, now.y + now.h);
        let rule = |across: bool, at: f32| {
            let rule = div().absolute().border_color(line.alpha(0.35));
            if across {
                rule.left_0().right_0().top(relative(at)).border_t_1()
            } else {
                rule.top_0().bottom_0().left(relative(at)).border_l_1()
            }
        };
        let (fall, rise) = (
            CursorStyle::ResizeUpLeftDownRight,
            CursorStyle::ResizeUpRightDownLeft,
        );
        let corners = [
            (0.0, 0.0, fall),
            (1.0, 0.0, rise),
            (0.0, 1.0, rise),
            (1.0, 1.0, fall),
        ];
        let (tall, wide) = (CursorStyle::ResizeUpDown, CursorStyle::ResizeLeftRight);
        let edges = [
            (0.5, 0.0, tall),
            (0.5, 1.0, tall),
            (0.0, 0.5, wide),
            (1.0, 0.5, wide),
        ];
        let marks = corners
            .into_iter()
            .chain(edges.into_iter().filter(|_| shape.is_none()))
            .map(|(x, y, cursor)| {
                div()
                    .absolute()
                    .left(relative(x))
                    .top(relative(y))
                    .size_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(div().flex_none().size(side).bg(line).cursor(cursor))
            });
        let body = div()
            .absolute()
            .left(relative(now.x))
            .top(relative(now.y))
            .w(relative(now.w))
            .h(relative(now.h))
            .border_1()
            .border_color(line)
            .children([1.0 / 3.0, 2.0 / 3.0].map(|at| rule(true, at)))
            .children([1.0 / 3.0, 2.0 / 3.0].map(|at| rule(false, at)))
            .when(editable, |body| body.children(marks));
        let (measure, pressed, moving, keyed) =
            (stage.clone(), stage.clone(), stage.clone(), crop.clone());
        let (drag_change, key_change) = (change.clone(), change);
        let pressed_crop = crop.clone();
        let picture = framed(self.ratio, cx)
            .debug_selector(|| "image-cropper".into())
            .child(Image::new((self.id.clone(), "picture"), self.source).size_full())
            .child(
                canvas(
                    move |bounds, window, cx| {
                        if measure.read(cx).bounds != bounds {
                            measure.update(cx, |stage, cx| {
                                stage.bounds = bounds;
                                cx.notify();
                            });
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .child(pane(0.0, 0.0, 1.0, now.y))
            .child(pane(0.0, y2, 1.0, 1.0 - y2))
            .child(pane(0.0, now.y, now.x, now.h))
            .child(pane(x2, now.y, 1.0 - x2, now.h));
        let lid = div()
            .absolute()
            .top(side / 2.0)
            .left(side / 2.0)
            .right(side / 2.0)
            .bottom(side / 2.0)
            .child(body);
        div()
            .id(self.id.clone())
            .relative()
            .p(side / 2.0)
            .border_1()
            .border_color(transparent_black())
            .child(picture)
            .child(lid)
            .when(editable, |area| {
                area.track_focus(&focus)
                    .focus_ring(cx)
                    .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                        let now = pressed_crop.read(cx).value;
                        pressed.update(cx, |stage, _| {
                            let (w, h) = (
                                f32::from(stage.bounds.size.width),
                                f32::from(stage.bounds.size.height),
                            );
                            let at = event.position - stage.bounds.origin;
                            let spot = (f32::from(at.x) / w, f32::from(at.y) / h);
                            let reach = (f32::from(hit) / w, f32::from(hit) / h);
                            stage.held = grip_at(now, spot, reach, shape.is_some());
                            stage.from = event.position;
                        })
                    })
                    .on_drag(Hold(owner), |_, _, _, cx| cx.new(|_| EmptyView))
                    .on_drag_move(move |event: &DragMoveEvent<Hold>, window, cx| {
                        if event.drag(cx).0 != owner {
                            return;
                        }
                        let at = event.event.position;
                        let (bounds, held, from) = moving.update(cx, |stage, _| {
                            let from = std::mem::replace(&mut stage.from, at);
                            (stage.bounds, stage.held, from)
                        });
                        let Some(grip) = held else {
                            return;
                        };
                        let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
                        let by = (f32::from(at.x - from.x) / w, f32::from(at.y - from.y) / h);
                        let before = keyed.read(cx).value;
                        let next = dragged(before, grip, by, LEAST, shape);
                        if next != before {
                            drag_change(next, window, cx);
                        }
                    })
                    .on_key_down(move |event, window, cx| {
                        let step = if event.keystroke.modifiers.shift {
                            NUDGE.1
                        } else {
                            NUDGE.0
                        };
                        let by = match event.keystroke.key.as_str() {
                            "left" => (-step, 0.0),
                            "right" => (step, 0.0),
                            "up" => (0.0, -step),
                            "down" => (0.0, step),
                            _ => return,
                        };
                        cx.stop_propagation();
                        let next = dragged(now, Grip::default(), by, LEAST, shape);
                        log::info!("image cropper: moved to {:.2}, {:.2}", next.x, next.y);
                        key_change(next, window, cx);
                    })
            })
    }
}
