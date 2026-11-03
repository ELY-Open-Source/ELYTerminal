use std::rc::Rc;

use gpui::{
    App, AppContext as _, Bounds, ElementId, EmptyView, Entity, EntityId, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Pixels, Point, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, canvas, div, prelude::*,
};

use super::{
    edit::{Tool, drawn, resized, snapped},
    gesture::{
        Brush, Gesture, Hand, Handlers, LEAST, OnDraw, OnKeys, OnMove, OnPair, OnResize, PEN,
        REACH, Scene, commit, path, spanned, stroke_shape, taken,
    },
    marks::{SelectionBox, SnapIndicator, TransformHandles, placed},
    paint::{in_view, paint_shape},
    press::{Press, pressed as pressed_with},
    shape::{Corner, Shape, ShapeKind},
    view::{Frame, Viewport},
    writing,
};
use crate::{forms::Editing, primitives::tab_stop, theme::ActiveTheme};

/// A drag on a tool layer, and whose layer it is.
struct Tracing {
    owner: EntityId,
}

/// A layer over an InfiniteCanvas where its tools work. Select presses a shape, Shift adds to the selection, a drag moves it and snaps its edges to others, a handle resizes it, and a drag on empty space draws a SelectionBox. The shape tools draw by a drag, Shift keeping them square; the pen adds a point per press and a double press ends the path; text drops a line where pressed; the brush strokes a path; a note drops where pressed; the connector links the shape pressed to the one released on. A double press with Select writes on a shape: a text's or a note's words, or any other's label. A press focuses the layer, where Delete or Backspace asks to delete the selection and Escape lets it go. A drag may end anywhere, in the layer or out of it. Everything asks the owner; the hand leaves the canvas to pan.
#[derive(IntoElement)]
pub struct ToolLayer {
    id: ElementId,
    tool: Tool,
    viewport: Viewport,
    shapes: Vec<Shape>,
    selected: Vec<SharedString>,
    brush: Brush,
    on_select: Option<OnKeys>,
    on_draw: Option<OnDraw>,
    on_move: Option<OnMove>,
    on_resize: Option<OnResize>,
    on_link: Option<OnPair>,
    on_text: Option<OnPair>,
    on_delete: Option<OnKeys>,
}

impl ToolLayer {
    pub fn new(
        id: impl Into<ElementId>,
        tool: Tool,
        viewport: Viewport,
        shapes: impl IntoIterator<Item = Shape>,
    ) -> Self {
        Self {
            id: id.into(),
            tool,
            viewport,
            shapes: shapes.into_iter().collect(),
            selected: Vec::new(),
            brush: Brush { size: 4.0, hue: 0 },
            on_select: None,
            on_draw: None,
            on_move: None,
            on_resize: None,
            on_link: None,
            on_text: None,
            on_delete: None,
        }
    }

    pub fn selected(mut self, keys: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.selected = keys.into_iter().map(Into::into).collect();
        self
    }

    pub fn brush(mut self, brush: Brush) -> Self {
        self.brush = brush;
        self
    }

    /// Gets the keys selected after each press or marquee.
    pub fn on_select(
        mut self,
        handler: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Gets a new shape's kind and frame, for the owner to key and keep.
    pub fn on_draw(
        mut self,
        handler: impl Fn(ShapeKind, Frame, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_draw = Some(Rc::new(handler));
        self
    }

    /// Gets the selection's keys and how far it moved, snaps included.
    pub fn on_move(
        mut self,
        handler: impl Fn(&[SharedString], (f32, f32), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_move = Some(Rc::new(handler));
        self
    }

    /// Gets a resized shape's key and new frame.
    pub fn on_resize(
        mut self,
        handler: impl Fn(&SharedString, Frame, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_resize = Some(Rc::new(handler));
        self
    }

    /// Gets the keys of two shapes the connector joined, from first.
    pub fn on_link(
        mut self,
        handler: impl Fn(&SharedString, &SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_link = Some(Rc::new(handler));
        self
    }

    /// Gets the selected keys when Delete or Backspace is pressed on the layer.
    pub fn on_delete(
        mut self,
        handler: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_delete = Some(Rc::new(handler));
        self
    }

    /// Gets a shape's key and the words written on it after a double press.
    pub fn on_text(
        mut self,
        handler: impl Fn(&SharedString, &SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_text = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ToolLayer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let hand: Entity<Hand> =
            window.use_keyed_state((self.id.clone(), "hand"), cx, |_, _| Hand::default());
        let editing =
            window.use_keyed_state((self.id.clone(), "writing"), cx, |_, _| Editing::default());
        if self.tool != Tool::Pen && !hand.read(cx).pen.is_empty() {
            let points = hand.update(cx, |hand, _| std::mem::take(&mut hand.pen));
            end_path(points, self.on_draw.clone(), window, cx);
        }
        if self.tool == Tool::Hand {
            return div().into_any_element();
        }
        let (view, tool) = (self.viewport, self.tool);
        let shapes = Rc::new(self.shapes);
        let selected = Rc::new(self.selected);
        let others: Vec<Frame> = shapes
            .iter()
            .filter(|shape| !selected.contains(&shape.key))
            .map(|shape| shape.frame)
            .collect();
        let whole = spanned(&shapes, &selected);
        let state = hand.read(cx);
        let gesture = state.gesture.clone();
        let square = state.square;
        let pen = state.pen.clone();
        let theme = cx.theme();
        let (palette, stroke) = (
            theme.colors.clone(),
            theme.canvas().stroke.to_pixels(window.rem_size()),
        );
        let side = f32::from(theme.canvas().handle.to_pixels(window.rem_size()));
        let (nudge, guides) = match &gesture {
            Gesture::Moving { from, to } => {
                let moved = whole.map(|frame| {
                    Frame::new(
                        frame.x + to.0 - from.0,
                        frame.y + to.1 - from.1,
                        frame.w,
                        frame.h,
                    )
                });
                moved.map_or(((0.0, 0.0), Vec::new()), |moved| {
                    snapped(&moved, &others, REACH / view.zoom)
                })
            }
            _ => ((0.0, 0.0), Vec::new()),
        };
        let preview: Vec<Shape> = match &gesture {
            Gesture::Drawing { kind, from, to } => {
                let (kind, frame) = drawn(kind.clone(), *from, *to, square);
                vec![Shape::new("+draft", "Draft", kind, frame)]
            }
            Gesture::Moving { from, to } => shapes
                .iter()
                .filter(|shape| selected.contains(&shape.key))
                .map(|shape| {
                    let mut moved = shape.clone();
                    moved.frame.x += to.0 - from.0 + nudge.0;
                    moved.frame.y += to.1 - from.1 + nudge.1;
                    moved
                })
                .collect(),
            Gesture::Resizing {
                key,
                handle,
                from,
                to,
            } => shapes
                .iter()
                .filter(|shape| shape.key == *key)
                .map(|shape| {
                    let mut sized = shape.clone();
                    sized.frame = resized(
                        shape.frame,
                        *handle,
                        (to.0 - from.0, to.1 - from.1),
                        LEAST,
                        square,
                    );
                    sized
                })
                .collect(),
            Gesture::Stroke { points } if points.len() > 1 => {
                vec![stroke_shape(points, self.brush.size, self.brush.hue)]
            }
            Gesture::Linking { from, to } => match shapes.iter().find(|shape| shape.key == *from) {
                Some(start) => {
                    let start = start.frame.center();
                    let (kind, frame) =
                        drawn(ShapeKind::Arrow(Corner::BottomRight), start, *to, false);
                    vec![Shape::new("+link", "Link", kind, frame)]
                }
                None => {
                    log::error!("tool layer: the link's shape {from} left the canvas");
                    Vec::new()
                }
            },
            _ if tool == Tool::Pen && pen.len() > 1 => {
                vec![stroke_shape(&pen, PEN, self.brush.hue)]
            }
            _ => Vec::new(),
        };
        let marquee = match &gesture {
            Gesture::Marquee { from, to } => Some(SelectionBox::new(in_view(
                &view,
                &Frame::spanning(*from, *to),
            ))),
            _ => None,
        };
        let chosen = (tool == Tool::Select && matches!(gesture, Gesture::Idle))
            .then_some(whole)
            .flatten()
            .map(|frame| in_view(&view, &frame));
        let handles = chosen
            .filter(|_| selected.len() == 1)
            .map(TransformHandles::new);
        let outline = chosen
            .filter(|_| selected.len() > 1)
            .map(|frame| placed(&frame).border_1().border_color(palette.accent));
        let snap = (!guides.is_empty()).then(|| SnapIndicator::new(guides.clone(), view));
        let writing_on = hand.read(cx).writing.clone();
        let field = writing_on
            .as_ref()
            .and_then(|key| shapes.iter().find(|shape| shape.key == *key))
            .and_then(|shape| writing::field(&editing, in_view(&view, &shape.frame), cx));
        if field.is_none() && writing_on.is_some() && editing.read(cx).field().is_some() {
            let closing = editing.clone();
            log::info!("tool layer: the shape written on left the canvas");
            window.defer(cx, move |window, cx| {
                closing.update(cx, |editing, cx| editing.finish(true, window, cx))
            });
        }
        let point_of = move |at: Point<Pixels>, bounds: Bounds<Pixels>| {
            let local = at - bounds.origin;
            view.to_canvas((f32::from(local.x), f32::from(local.y)))
        };
        let owner = hand.entity_id();
        let focus = tab_stop((self.id.clone(), "focus").into(), true, window, cx);
        let (ring, held_focus) = (
            if focus.is_focused(window) {
                palette.focus
            } else {
                gpui::transparent_black()
            },
            focus.clone(),
        );
        let (gone, on_delete, let_go) = (
            selected.clone(),
            self.on_delete.clone(),
            self.on_select.clone(),
        );
        let (pressed, moved, dropped, released) =
            (hand.clone(), hand.clone(), hand.clone(), hand.clone());
        let handlers = Handlers {
            on_select: self.on_select,
            on_draw: self.on_draw,
            on_move: self.on_move,
            on_resize: self.on_resize,
            on_link: self.on_link,
            on_text: self.on_text,
        };
        let scene = Rc::new(Scene {
            shapes,
            selected,
            others,
            view,
        });
        let handlers = Rc::new(handlers);
        let select_on_up = handlers.on_select.clone();
        let (press_scene, press_handlers, press_editing) =
            (scene.clone(), handlers.clone(), editing.clone());
        let brush = self.brush;
        let (out_hand, out_scene, out_handlers) = (hand.clone(), scene.clone(), handlers.clone());
        let measured = hand.clone();
        div()
            .id(self.id)
            .track_focus(&focus)
            .absolute()
            .inset_0()
            .child(div().absolute().inset_0().border_1().border_color(ring))
            .when(tool != Tool::Select, |layer| layer.cursor_crosshair())
            .on_key_down(
                move |event, window, cx| match event.keystroke.key.as_str() {
                    "backspace" | "delete" if !gone.is_empty() => {
                        cx.stop_propagation();
                        log::info!("tool layer: delete {gone:?}");
                        if let Some(on_delete) = &on_delete {
                            on_delete(&gone, window, cx);
                        }
                    }
                    "escape" if !gone.is_empty() => {
                        cx.stop_propagation();
                        log::info!("tool layer: nothing selected");
                        if let Some(on_select) = &let_go {
                            on_select(&[], window, cx);
                        }
                    }
                    _ => {}
                },
            )
            .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                cx.stop_propagation();
                window.focus(&held_focus, cx);
                let bounds = pressed.read(cx).bounds;
                let at = point_of(event.position, bounds);
                let local = event.position - bounds.origin;
                let view_at = (f32::from(local.x), f32::from(local.y));
                let press = Press {
                    tool,
                    at,
                    view_at,
                    count: event.click_count,
                    shift: event.modifiers.shift,
                    side,
                    back: held_focus.clone(),
                };
                let gesture = pressed_with(
                    &press,
                    &press_scene,
                    &press_handlers,
                    &pressed,
                    &press_editing,
                    window,
                    cx,
                );
                pressed.update(cx, |hand, cx| {
                    hand.gesture = gesture;
                    hand.square = event.modifiers.shift;
                    cx.notify();
                });
            })
            .on_drag(Tracing { owner }, |_, _, _, cx| cx.new(|_| EmptyView))
            .on_drag_move::<Tracing>(move |event, _, cx| {
                if event.drag(cx).owner != owner {
                    return;
                }
                let bounds = moved.read(cx).bounds;
                let at = point_of(event.event.position, bounds);
                moved.update(cx, |hand, cx| {
                    hand.square = event.event.modifiers.shift;
                    match &mut hand.gesture {
                        Gesture::Marquee { to, .. }
                        | Gesture::Moving { to, .. }
                        | Gesture::Resizing { to, .. }
                        | Gesture::Drawing { to, .. }
                        | Gesture::Linking { to, .. } => *to = at,
                        Gesture::Stroke { points } => points.push(at),
                        Gesture::Idle => {}
                    }
                    cx.notify();
                });
            })
            .on_drop(move |_: &Tracing, window, cx| {
                let (gesture, square) = taken(&dropped, cx);
                commit(gesture, square, &scene, &handlers, brush, window, cx);
            })
            .on_mouse_up_out(MouseButton::Left, move |_, window, cx| {
                let (gesture, square) = taken(&out_hand, cx);
                commit(
                    gesture,
                    square,
                    &out_scene,
                    &out_handlers,
                    brush,
                    window,
                    cx,
                );
            })
            .on_mouse_up(MouseButton::Left, move |_, window, cx| {
                if cx.has_active_drag() {
                    return;
                }
                let gesture = released.update(cx, |hand, cx| {
                    cx.notify();
                    std::mem::take(&mut hand.gesture)
                });
                if matches!(gesture, Gesture::Marquee { .. }) {
                    log::info!("tool layer: nothing selected");
                    if let Some(on_select) = &select_on_up {
                        on_select(&[], window, cx);
                    }
                }
            })
            .child(
                canvas(
                    move |bounds, window, cx| {
                        if measured.read(cx).bounds != bounds {
                            measured.update(cx, |hand, _| hand.bounds = bounds);
                            window.request_animation_frame();
                        }
                    },
                    move |bounds, _, window, _| {
                        for shape in &preview {
                            paint_shape(shape, &view, bounds.origin, &palette, stroke, window);
                        }
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .children(marquee)
            .children(handles)
            .children(outline)
            .children(field)
            .children(snap)
            .into_any_element()
    }
}

/// Ends a pen path the tool left: two points or more go to the owner after this frame, a lone point is dropped.
fn end_path(points: Vec<(f32, f32)>, on_draw: Option<OnDraw>, window: &mut Window, cx: &mut App) {
    if points.len() < 2 {
        log::info!("tool layer: the pen left a lone point, dropped");
        return;
    }
    window.defer(cx, move |window, cx| {
        let (kind, frame) = path(&points, PEN);
        log::info!("tool layer: the pen left a path of {} points", points.len());
        if let Some(on_draw) = &on_draw {
            on_draw(kind, frame, window, cx);
        }
    });
}
