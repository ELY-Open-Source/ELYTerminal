use std::rc::Rc;

use gpui::{App, Bounds, Entity, Pixels, SharedString, Window};

use super::{
    edit::{Handle, drawn, hit, inside, resized, snapped},
    shape::{Shape, ShapeKind},
    view::{Frame, Viewport},
};

pub(super) type OnKeys = Rc<dyn Fn(&[SharedString], &mut Window, &mut App)>;
pub(super) type OnDraw = Rc<dyn Fn(ShapeKind, Frame, &mut Window, &mut App)>;
pub(super) type OnMove = Rc<dyn Fn(&[SharedString], (f32, f32), &mut Window, &mut App)>;
pub(super) type OnResize = Rc<dyn Fn(&SharedString, Frame, &mut Window, &mut App)>;
/// A handler for two keys, or a key and its words.
pub(crate) type OnPair = Rc<dyn Fn(&SharedString, &SharedString, &mut Window, &mut App)>;

/// A brush's width in canvas units and its hue among the chart colors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Brush {
    pub size: f32,
    pub hue: usize,
}

/// What a press set going, with its canvas points.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) enum Gesture {
    #[default]
    Idle,
    Marquee {
        from: (f32, f32),
        to: (f32, f32),
    },
    Moving {
        from: (f32, f32),
        to: (f32, f32),
    },
    Resizing {
        key: SharedString,
        handle: Handle,
        from: (f32, f32),
        to: (f32, f32),
    },
    Drawing {
        kind: ShapeKind,
        from: (f32, f32),
        to: (f32, f32),
    },
    Stroke {
        points: Vec<(f32, f32)>,
    },
    Linking {
        from: SharedString,
        to: (f32, f32),
    },
}

/// The layer's own state: the gesture under way, whether Shift is held, the box it covers, the pen's points so far, and the shape being written on.
#[derive(Default)]
pub(super) struct Hand {
    pub(super) gesture: Gesture,
    pub(super) square: bool,
    pub(super) bounds: Bounds<Pixels>,
    pub(super) pen: Vec<(f32, f32)>,
    pub(super) writing: Option<SharedString>,
}

/// Shapes a small nudge still counts as a click by, in view pixels.
pub(super) const REACH: f32 = 4.0;

/// The least a drawn or resized shape spans, in canvas units.
pub(super) const LEAST: f32 = 2.0;

/// A pen path's width, in canvas units.
pub(super) const PEN: f32 = 2.0;

/// The frame the selected shapes span together.
pub(super) fn spanned(shapes: &[Shape], selected: &[SharedString]) -> Option<Frame> {
    shapes
        .iter()
        .filter(|shape| selected.contains(&shape.key))
        .map(|shape| shape.frame)
        .reduce(|whole, frame| whole.union(&frame))
}

/// A path through canvas points, `width` wide: its kind and its frame, the points counted from the frame's corner.
pub(super) fn path(points: &[(f32, f32)], width: f32) -> (ShapeKind, Frame) {
    let frame = points
        .iter()
        .map(|(x, y)| Frame::new(*x, *y, 0.0, 0.0))
        .reduce(|whole, frame| whole.union(&frame))
        .expect("a path has points");
    let points = points
        .iter()
        .map(|(x, y)| (x - frame.x, y - frame.y))
        .collect();
    (ShapeKind::Path { points, width }, frame)
}

pub(super) fn stroke_shape(points: &[(f32, f32)], width: f32, hue: usize) -> Shape {
    let (kind, frame) = path(points, width);
    Shape::new("+stroke", "Stroke", kind, frame).hue(hue)
}

/// The gesture under way, taken so it ends once, with whether Shift was held.
pub(super) fn taken(hand: &Entity<Hand>, cx: &mut App) -> (Gesture, bool) {
    hand.update(cx, |hand, cx| {
        cx.notify();
        (std::mem::take(&mut hand.gesture), hand.square)
    })
}

/// The owner's handlers for what a gesture ends in.
pub(super) struct Handlers {
    pub(super) on_select: Option<OnKeys>,
    pub(super) on_draw: Option<OnDraw>,
    pub(super) on_move: Option<OnMove>,
    pub(super) on_resize: Option<OnResize>,
    pub(super) on_link: Option<OnPair>,
    pub(super) on_text: Option<OnPair>,
}

/// What a gesture ends against: the shapes, the selection, the frames to snap to, and the view.
pub(super) struct Scene {
    pub(super) shapes: Rc<Vec<Shape>>,
    pub(super) selected: Rc<Vec<SharedString>>,
    pub(super) others: Vec<Frame>,
    pub(super) view: Viewport,
}

/// Hands a finished gesture to the owner.
pub(super) fn commit(
    gesture: Gesture,
    square: bool,
    scene: &Scene,
    handlers: &Handlers,
    brush: Brush,
    window: &mut Window,
    cx: &mut App,
) {
    match gesture {
        Gesture::Marquee { from, to } => {
            let keys = inside(&scene.shapes, &Frame::spanning(from, to));
            log::info!("tool layer: marquee takes {keys:?}");
            if let Some(on_select) = &handlers.on_select {
                on_select(&keys, window, cx);
            }
        }
        Gesture::Moving { from, to } => {
            let Some(whole) = spanned(&scene.shapes, &scene.selected) else {
                return;
            };
            let raw = (to.0 - from.0, to.1 - from.1);
            let moved = Frame::new(whole.x + raw.0, whole.y + raw.1, whole.w, whole.h);
            let (nudge, _) = snapped(&moved, &scene.others, REACH / scene.view.zoom);
            let by = (raw.0 + nudge.0, raw.1 + nudge.1);
            if by == (0.0, 0.0) {
                return;
            }
            log::info!("tool layer: move {:?} by {by:?}", scene.selected);
            if let Some(on_move) = &handlers.on_move {
                on_move(&scene.selected, by, window, cx);
            }
        }
        Gesture::Resizing {
            key,
            handle,
            from,
            to,
        } => {
            let Some(shape) = scene.shapes.iter().find(|shape| shape.key == key) else {
                log::error!("tool layer: resized shape {key} left the canvas");
                return;
            };
            let frame = shape.frame;
            let next = resized(frame, handle, (to.0 - from.0, to.1 - from.1), LEAST, square);
            log::info!("tool layer: resize {key} to {next:?}");
            if let Some(on_resize) = &handlers.on_resize {
                on_resize(&key, next, window, cx);
            }
        }
        Gesture::Drawing { kind, from, to } => {
            let (kind, frame) = drawn(kind, from, to, square);
            if frame.w.max(frame.h) < LEAST {
                return;
            }
            log::info!("tool layer: draw {kind:?} at {frame:?}");
            if let Some(on_draw) = &handlers.on_draw {
                on_draw(kind, frame, window, cx);
            }
        }
        Gesture::Linking { from, to } => {
            if !scene.shapes.iter().any(|shape| shape.key == from) {
                log::error!("tool layer: the link's shape {from} left the canvas");
                return;
            }
            let Some(ix) = hit(&scene.shapes, to, REACH / scene.view.zoom) else {
                log::info!("tool layer: the link from {from} lands on nothing");
                return;
            };
            let target = &scene.shapes[ix].key;
            if *target == from {
                return;
            }
            log::info!("tool layer: link {from} to {target}");
            if let Some(on_link) = &handlers.on_link {
                on_link(&from, target, window, cx);
            }
        }
        Gesture::Stroke { points } if points.len() > 1 => {
            let (kind, frame) = path(&points, brush.size);
            log::info!("tool layer: a stroke of {} points", points.len());
            if let Some(on_draw) = &handlers.on_draw {
                on_draw(kind, frame, window, cx);
            }
        }
        _ => {}
    }
}
