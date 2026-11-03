use std::rc::Rc;

use gpui::{
    AnyElement, App, Axis, DefiniteLength, Div, DragMoveEvent, ElementId, EmptyView, EntityId,
    FontWeight, IntoElement, MouseButton, MouseDownEvent, ParentElement, Pixels, RenderOnce,
    SharedString, Styled, Window, div, prelude::*,
};
use smallvec::SmallVec;

use super::chrome::{OnClose, WindowControls, controls_style, drag_region, route_close};
use crate::{
    layout::{on_axis, resize_handle},
    theme::{ActiveTheme, ControlSize, Platform, TextSize},
    typography::Ellipsis,
};

type OnResize = Rc<dyn Fn(Pixels, &mut Window, &mut App)>;

/// A width the owner keeps, which a drag on one border moves.
struct Resize {
    width: Pixels,
    min: Pixels,
    max: Pixels,
    on_resize: OnResize,
}

/// The border a column's width is dragged by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

#[derive(Clone, Copy)]
struct BorderDrag {
    owner: EntityId,
    side: Side,
}

/// Where a press on a border landed; once it moves sideways, the width it sets off from and the owner's then.
#[derive(Clone, Copy)]
struct Press {
    at: Pixels,
    start: Option<(Pixels, Pixels)>,
}

/// One column of a `ColumnShell`: a header that drags the window, then its body.
pub struct ShellColumn {
    id: ElementId,
    width: Option<DefiniteLength>,
    resize: Option<Resize>,
    leading: SmallVec<[AnyElement; 2]>,
    title: Option<SharedString>,
    actions: SmallVec<[AnyElement; 2]>,
    body: SmallVec<[AnyElement; 2]>,
}

impl ShellColumn {
    /// Fills the width the other columns leave.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            width: None,
            resize: None,
            leading: SmallVec::new(),
            title: None,
            actions: SmallVec::new(),
            body: SmallVec::new(),
        }
    }

    /// A set width instead of the rest; it holds its header's fixed parts and yields when the shell runs short.
    pub fn width(mut self, width: impl Into<DefiniteLength>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// A set width the owner keeps: a drag on the border toward the filling column moves it within `min` and `max`, and the owner hears each new width.
    pub fn resizable(
        mut self,
        width: Pixels,
        min: Pixels,
        max: Pixels,
        on_resize: impl Fn(Pixels, &mut Window, &mut App) + 'static,
    ) -> Self {
        assert!(
            min <= max,
            "shell column {:?}: limits {min:?} to {max:?} run backwards",
            self.id
        );
        self.width = Some(width.into());
        self.resize = Some(Resize {
            width,
            min,
            max,
            on_resize: Rc::new(on_resize),
        });
        self
    }

    /// Header content before the title.
    pub fn leading(mut self, item: impl IntoElement) -> Self {
        self.leading.push(item.into_any_element());
        self
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Header content at the end.
    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }
}

impl ParentElement for ShellColumn {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

/// Columns to the window's top, each under a header; controls lead the first or end the last.
#[derive(IntoElement)]
pub struct ColumnShell {
    id: ElementId,
    platform: Option<Platform>,
    on_close: Option<OnClose>,
    columns: Vec<ShellColumn>,
    glass: bool,
}

impl ColumnShell {
    /// On macOS the system draws the traffic lights over the first column.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            platform: None,
            on_close: None,
            columns: Vec::new(),
            glass: false,
        }
    }

    pub fn column(mut self, column: ShellColumn) -> Self {
        self.columns.push(column);
        self
    }

    /// Draws this platform's controls instead of the system's.
    pub fn platform(mut self, platform: Platform) -> Self {
        self.platform = Some(platform);
        self
    }

    /// Runs instead of closing; without a forced platform, system closes run it too.
    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }

    /// Grounds the columns in the palette's glass, so a blurred window shows through them.
    pub fn glass(mut self) -> Self {
        self.glass = true;
        self
    }
}

impl RenderOnce for ColumnShell {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let count = self.columns.len();
        assert!(count > 0, "column shell {:?} needs a column", self.id);
        if self.platform.is_none() {
            route_close(&self.id, self.on_close.clone(), window, cx);
        }
        let (system, style) = controls_style(self.platform, cx);
        let controls = || {
            let controls = WindowControls::new((self.id.clone(), "controls")).platform(style);
            let controls = match self.on_close.clone() {
                Some(close) => controls.on_close(move |window, cx| close(window, cx)),
                None => controls,
            };
            div()
                .flex_none()
                .debug_selector(|| "column-shell-controls".into())
                .child(controls)
        };
        let room = system && !window.is_fullscreen();
        // Each border lies toward the column that fills, so a drag on it moves one width.
        let fill = self
            .columns
            .iter()
            .position(|column| column.width.is_none());
        let mut columns = Vec::with_capacity(count);
        for (ix, column) in self.columns.into_iter().enumerate() {
            let (first, last) = (ix == 0, ix + 1 == count);
            let side = match fill {
                Some(fill) if ix > fill => Some(Side::Left),
                Some(fill) if ix == fill => None,
                _ => (!last).then_some(Side::Right),
            };
            let lead = (!system && first && style == Platform::Mac).then(controls);
            let trail = (!system && last && style != Platform::Mac).then(controls);
            let flush = trail.is_some() && style == Platform::Windows;
            let header = header(column.id.clone(), window, cx)
                .when(first && room, |header| {
                    header.pl(cx.theme().traffic_light_inset())
                })
                .when(!(first && room), |header| header.pl_3())
                .when(!flush, |header| header.pr_3())
                .children(lead)
                .child(
                    items(
                        column.id.clone(),
                        column.leading,
                        title(column.id.clone(), column.title, window, cx),
                        column.actions,
                    )
                    .min_w(cx.theme().control_height(ControlSize::Lg))
                    .debug_selector(move || format!("column-shell-items-{ix}")),
                )
                .children(trail);
            let colors = &cx.theme().colors;
            let box_ = div()
                .relative()
                .flex()
                .flex_col()
                .h_full()
                // A basis, not a width, so the header's fixed parts set the least width.
                .map(|box_| match column.width {
                    Some(width) => box_.flex_basis(width),
                    None => box_.flex_1(),
                })
                // A resizing column's handle draws its border.
                .map(|box_| match (side, column.resize.is_some()) {
                    (Some(Side::Left), false) => box_.border_l_1(),
                    (Some(Side::Right), false) => box_.border_r_1(),
                    _ => box_,
                })
                .border_color(colors.border)
                .debug_selector(move || format!("column-shell-{ix}"))
                .child(header)
                .child(
                    // Absolute, so only the header sets the least width.
                    div().flex_1().min_h_0().relative().child(
                        div()
                            .absolute()
                            .inset_0()
                            .flex()
                            .flex_col()
                            .children(column.body),
                    ),
                );
            columns.push(match column.resize {
                Some(resize) => {
                    let side = side.unwrap_or_else(|| {
                        panic!(
                            "column shell {:?}: column {ix} resizes, but no border lies toward a filling column",
                            self.id
                        )
                    });
                    resizing(box_, column.id, ix, side, resize, window, cx)
                }
                None => box_,
            });
        }
        let theme = cx.theme();
        let ground = if self.glass {
            theme.colors.glass
        } else {
            theme.colors.bg
        };
        div()
            .flex()
            .h_full()
            .debug_selector(|| "column-shell".into())
            .bg(ground)
            .text_color(theme.colors.fg)
            .children(columns)
    }
}

/// `box_` with a handle on its `side` border, whose drag moves its width.
fn resizing(
    box_: Div,
    id: ElementId,
    ix: usize,
    side: Side,
    resize: Resize,
    window: &mut Window,
    cx: &mut App,
) -> Div {
    let Resize {
        width,
        min,
        max,
        on_resize,
    } = resize;
    let pressed = window.use_keyed_state((id.clone(), "press"), cx, |_, _| None::<Press>);
    let owner = pressed.entity_id();
    let drag = BorderDrag { owner, side };
    let at = pressed.clone();
    let handle = resize_handle((id, "border"), Axis::Horizontal, cx)
        .absolute()
        .top_0()
        .map(|handle| match side {
            Side::Left => handle.left_0().justify_start(),
            Side::Right => handle.right_0().justify_end(),
        })
        // Over a header, so a press drags the border, not the window.
        .occlude()
        .debug_selector(move || format!("column-shell-border-{ix}"))
        .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
            at.update(cx, |at, _| {
                *at = Some(Press {
                    at: event.position.x,
                    start: None,
                })
            })
        })
        .on_drag(drag, |_, _, _, cx| cx.new(|_| EmptyView));
    box_.on_drag_move(move |event: &DragMoveEvent<BorderDrag>, window, cx| {
        let BorderDrag {
            owner: dragged,
            side,
        } = *event.drag(cx);
        if dragged != owner {
            return;
        }
        let Some(press) = *pressed.read(cx) else {
            log::error!("column shell: a drag on column {ix} with no press behind it");
            return;
        };
        let moved = event.event.position.x - press.at;
        let (start, kept) = match press.start {
            Some(start) => start,
            // A drag that has not moved sideways yet changes nothing.
            None if moved == Pixels::ZERO => return,
            // A header's parts may hold a column wider, a short shell squeeze it narrower.
            None => {
                let start = (event.bounds.size.width.max(width), width);
                pressed.update(cx, |held, _| {
                    *held = Some(Press {
                        start: Some(start),
                        ..press
                    })
                });
                start
            }
        };
        let next = match side {
            _ if moved == Pixels::ZERO => kept,
            Side::Left => (start - moved).clamp(min, max),
            Side::Right => (start + moved).clamp(min, max),
        };
        if next != width {
            on_resize(next, window, cx);
        }
    })
    .child(handle)
}

/// A header strip at the title bar's height that drags the window.
fn header(id: ElementId, window: &mut Window, cx: &mut App) -> gpui::Stateful<gpui::Div> {
    drag_region((id, "header"), window, cx)
        .flex()
        .flex_none()
        .items_center()
        .gap_2()
        .h(cx.theme().titlebar_height())
}

/// The host's header items: one control wide at least, scrolling when short.
fn items(
    id: ElementId,
    leading: SmallVec<[AnyElement; 2]>,
    title: gpui::Stateful<gpui::Div>,
    actions: SmallVec<[AnyElement; 2]>,
) -> gpui::Div {
    let group = || div().flex_none().flex().items_center().gap_2();
    div().flex_1().min_w_0().h_full().relative().child(
        on_axis(div().id((id, "items")))
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .gap_2()
            .overflow_x_scroll()
            // Windows takes a drag region under controls as caption.
            .occlude()
            .child(group().children(leading))
            .child(title)
            .child(group().children(actions)),
    )
}

/// Title or spacer: drags the window, adds no width.
fn title(
    id: ElementId,
    title: Option<SharedString>,
    window: &mut Window,
    cx: &mut App,
) -> gpui::Stateful<gpui::Div> {
    let region = drag_region((id, "title"), window, cx);
    let theme = cx.theme();
    region.flex_1().min_w_0().h_full().relative().child(
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .text_size(theme.text_size(TextSize::Sm))
            .font_weight(FontWeight::MEDIUM)
            .text_color(theme.colors.fg_muted)
            .children(title.map(Ellipsis::new)),
    )
}
