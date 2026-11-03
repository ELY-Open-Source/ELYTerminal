use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, canvas, div,
};

use super::{
    edit::Tool,
    gesture::Brush,
    links::Link,
    palette::ToolPalette,
    panel::OnEdit,
    plane::InfiniteCanvas,
    shape::{Shape, ShapeKind},
    tools::ToolLayer,
    view::{Frame, Viewport},
    zoom::ZoomControls,
};
use crate::{
    layout::seeded::use_seeded,
    theme::{ActiveTheme, Elevation, Radius},
};

/// What lies on a whiteboard: its shapes, in paint order, and the links between them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Board {
    pub shapes: Vec<Shape>,
    pub links: Vec<Link>,
}

/// The tools a whiteboard offers, in the palette's order.
const TOOLS: [Tool; 10] = [
    Tool::Select,
    Tool::Hand,
    Tool::Note,
    Tool::Rect,
    Tool::Ellipse,
    Tool::Arrow,
    Tool::Connector,
    Tool::Text,
    Tool::Pen,
    Tool::Brush,
];

/// The whiteboard's own state: its tool, view and selection, the size of its box, and how many things it has made.
struct Desk {
    tool: Tool,
    view: Viewport,
    selected: Vec<SharedString>,
    size: (f32, f32),
    made: usize,
}

type Apply = Rc<dyn Fn(&dyn Fn(&mut Board, &mut Desk), &mut Window, &mut App)>;

/// A board to think on: notes, shapes, text, pen and brush strokes, and links on an endless plane, with its tools floating over the top left and its zoom at the bottom right. It keeps its own tool, view and selection, applies each edit, and hands the owner the whole board after it; a new board from the owner replaces its own. Delete or Backspace removes the selection and its links. It fills its box, which the host sizes.
#[derive(IntoElement)]
pub struct Whiteboard {
    id: ElementId,
    board: Board,
    on_change: Option<OnEdit<Board>>,
}

impl Whiteboard {
    pub fn new(id: impl Into<ElementId>, board: Board) -> Self {
        Self {
            id: id.into(),
            board,
            on_change: None,
        }
    }

    pub fn on_change(mut self, handler: impl Fn(Board, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

/// A key no shape of `board` holds yet.
fn fresh(board: &Board, made: &mut usize) -> SharedString {
    loop {
        *made += 1;
        let key = SharedString::from(format!("board-{made}"));
        if !board.shapes.iter().any(|shape| shape.key == key)
            && !board.links.iter().any(|link| link.key == key)
        {
            return key;
        }
    }
}

impl RenderOnce for Whiteboard {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let content = use_seeded((id.clone(), "board"), self.board, window, cx);
        let desk = window.use_keyed_state((id.clone(), "desk"), cx, |_, _| Desk {
            tool: Tool::Select,
            view: Viewport::new(0.0, 0.0, 1.0),
            selected: Vec::new(),
            size: (0.0, 0.0),
            made: 0,
        });
        let board = content.read(cx).value.clone();
        let (tool, view, selected, size) = {
            let desk = desk.read(cx);
            (desk.tool, desk.view, desk.selected.clone(), desk.size)
        };
        let apply: Apply = {
            let (content, desk, on_change) = (content.clone(), desk.clone(), self.on_change);
            Rc::new(move |edit, window, cx| {
                let mut board = content.read(cx).value.clone();
                desk.update(cx, |desk, cx| {
                    edit(&mut board, desk);
                    cx.notify();
                });
                content.update(cx, |content, cx| {
                    content.value = board.clone();
                    cx.notify();
                });
                log::info!(
                    "whiteboard: {} shapes, {} links",
                    board.shapes.len(),
                    board.links.len()
                );
                if let Some(on_change) = &on_change {
                    on_change(board, window, cx);
                }
            })
        };
        let set = {
            let desk = desk.clone();
            move |edit: &dyn Fn(&mut Desk), cx: &mut App| {
                desk.update(cx, |desk, cx| {
                    edit(desk);
                    cx.notify();
                })
            }
        };
        let [drawn, moved, sized, linked, written, removed] = [(); 6].map(|_| apply.clone());
        let [chosen, viewed, picked, zoomed, fitted] = [(); 5].map(|_| set.clone());
        let whole = board
            .shapes
            .iter()
            .map(|shape| shape.frame)
            .reduce(|whole, frame| whole.union(&frame));
        let layer = ToolLayer::new((id.clone(), "tools"), tool, view, board.shapes.clone())
            .selected(selected)
            .brush(Brush { size: 4.0, hue: 0 })
            .on_select(move |keys, _, cx| {
                let keys = keys.to_vec();
                chosen(&|desk| desk.selected = keys.clone(), cx)
            })
            .on_draw(move |kind, frame, window, cx| {
                drawn(
                    &|board, desk| {
                        let key = fresh(board, &mut desk.made);
                        let (name, hue) = match &kind {
                            ShapeKind::Note(_) => ("Note", 2),
                            ShapeKind::Text(_) => ("Text", 0),
                            ShapeKind::Path { .. } => ("Stroke", 0),
                            _ => ("Shape", desk.made % 8),
                        };
                        board
                            .shapes
                            .push(Shape::new(key.clone(), name, kind.clone(), frame).hue(hue));
                        desk.selected = vec![key];
                    },
                    window,
                    cx,
                )
            })
            .on_move(move |keys, (dx, dy), window, cx| {
                let keys = keys.to_vec();
                moved(
                    &|board, _| {
                        for shape in board
                            .shapes
                            .iter_mut()
                            .filter(|shape| keys.contains(&shape.key))
                        {
                            shape.frame = Frame::new(
                                shape.frame.x + dx,
                                shape.frame.y + dy,
                                shape.frame.w,
                                shape.frame.h,
                            );
                        }
                    },
                    window,
                    cx,
                )
            })
            .on_resize(move |key, frame, window, cx| {
                let key = key.clone();
                sized(
                    &|board, _| {
                        let Some(shape) = board.shapes.iter_mut().find(|shape| shape.key == key)
                        else {
                            log::error!("whiteboard: resized shape {key} left the board");
                            return;
                        };
                        shape.frame = frame;
                    },
                    window,
                    cx,
                )
            })
            .on_link(move |from, to, window, cx| {
                let (from, to) = (from.clone(), to.clone());
                linked(
                    &|board, desk| {
                        let key = fresh(board, &mut desk.made);
                        board.links.push(Link::new(key, from.clone(), to.clone()));
                    },
                    window,
                    cx,
                )
            })
            .on_delete(move |keys, window, cx| {
                let gone = keys.to_vec();
                removed(
                    &|board, desk| {
                        desk.selected.clear();
                        board.shapes.retain(|shape| !gone.contains(&shape.key));
                        board
                            .links
                            .retain(|link| !gone.contains(&link.from) && !gone.contains(&link.to));
                    },
                    window,
                    cx,
                )
            })
            .on_text(move |key, text, window, cx| {
                let (key, text) = (key.clone(), text.clone());
                written(
                    &|board, _| {
                        let Some(shape) = board.shapes.iter_mut().find(|shape| shape.key == key)
                        else {
                            log::error!("whiteboard: written shape {key} left the board");
                            return;
                        };
                        *shape = shape.written(text.clone());
                    },
                    window,
                    cx,
                )
            });
        let theme = cx.theme();
        let floating = |element: gpui::AnyElement| {
            div()
                .absolute()
                .rounded(theme.radius(Radius::Lg))
                .bg(theme.colors.surface)
                .shadow(theme.elevation(Elevation::Floating))
                .child(element)
        };
        let measured = desk.clone();
        div()
            .id(id.clone())
            .relative()
            .size_full()
            .overflow_hidden()
            .child(
                canvas(
                    move |bounds, window, cx| {
                        let now = (f32::from(bounds.size.width), f32::from(bounds.size.height));
                        if measured.read(cx).size != now {
                            measured.update(cx, |desk, _| desk.size = now);
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
            .child(
                InfiniteCanvas::new((id.clone(), "plane"), view)
                    .shapes(board.shapes)
                    .links(board.links)
                    .layer(layer)
                    .on_viewport(move |next, _, cx| viewed(&|desk| desk.view = next, cx)),
            )
            .child(
                floating(
                    ToolPalette::new((id.clone(), "palette"), tool)
                        .tools(TOOLS)
                        .on_change(move |next, _, cx| picked(&|desk| desk.tool = next, cx))
                        .into_any_element(),
                )
                .top_3()
                .left_3(),
            )
            .child(
                floating(
                    ZoomControls::new((id, "zoom"), view.zoom)
                        .on_zoom(move |zoom, _, cx| {
                            let middle = (size.0 / 2.0, size.1 / 2.0);
                            zoomed(
                                &|desk| desk.view = desk.view.zoomed(zoom / desk.view.zoom, middle),
                                cx,
                            )
                        })
                        .on_fit(move |_, cx| {
                            if let Some(whole) = whole {
                                fitted(
                                    &|desk| desk.view = Viewport::fitting(whole, desk.size, 48.0),
                                    cx,
                                )
                            }
                        })
                        .into_any_element(),
                )
                .bottom_3()
                .right_3(),
            )
    }
}
