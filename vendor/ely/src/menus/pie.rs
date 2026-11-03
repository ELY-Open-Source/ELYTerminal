use std::{f32::consts::TAU, rc::Rc};

use gpui::{
    Animation, AnimationExt, AnyElement, App, Bounds, ElementId, Entity, FontWeight,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, Point, RenderOnce,
    SharedString, Styled, Window, anchored, canvas, div, point,
};
use smallvec::SmallVec;

use crate::{
    forms::{Pick, Run},
    motion,
    primitives::{Icon, IconName, give_back, raise, take_focus},
    theme::{ActiveTheme, Elevation, IconSize, TextSize},
};

/// One slice of a pie menu.
#[derive(Clone)]
pub struct PieItem {
    icon: IconName,
    label: SharedString,
    on_click: Option<Run>,
}

impl PieItem {
    pub fn new(icon: IconName, label: impl Into<SharedString>) -> Self {
        Self {
            icon,
            label: label.into(),
            on_click: None,
        }
    }

    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

/// Where the ring opened, where its center landed, and the slice the pointer points at.
#[derive(Default)]
struct Pie {
    at: Option<Point<Pixels>>,
    center: Point<Pixels>,
    marked: Option<usize>,
}

/// The slice at `pointer`: item 0 sits at the top and the rest follow clockwise; the hub picks none.
fn slice(
    center: Point<Pixels>,
    pointer: Point<Pixels>,
    hub: Pixels,
    count: usize,
) -> Option<usize> {
    let (dx, dy) = (
        f32::from(pointer.x - center.x),
        f32::from(pointer.y - center.y),
    );
    if dx.hypot(dy) < f32::from(hub) / 2.0 {
        return None;
    }
    let wedge = TAU / count as f32;
    let turn = (dy.atan2(dx) + TAU / 4.0 + wedge / 2.0).rem_euclid(TAU);
    Some((turn / wedge) as usize % count)
}

/// Its children, and a ring of choices that opens at the pointer on a right click. The pointer's direction marks one; a click runs it.
#[derive(IntoElement)]
pub struct PieMenu {
    id: ElementId,
    items: Vec<PieItem>,
    children: SmallVec<[AnyElement; 2]>,
}

impl PieMenu {
    pub fn new(id: impl Into<ElementId>, items: impl IntoIterator<Item = PieItem>) -> Self {
        let items: Vec<PieItem> = items.into_iter().collect();
        assert!(items.len() >= 2, "a pie menu needs two slices or more");
        Self {
            id: id.into(),
            items,
            children: SmallVec::new(),
        }
    }
}

impl ParentElement for PieMenu {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

fn show(state: &Entity<Pie>, at: Option<Point<Pixels>>, cx: &mut App) {
    state.update(cx, |pie, cx| {
        *pie = Pie {
            at,
            ..Pie::default()
        };
        cx.notify();
    });
}

impl RenderOnce for PieMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state((self.id.clone(), "pie"), cx, |_, _| Pie::default());
        let opener = state.clone();
        let region = div()
            .id(self.id.clone())
            .on_mouse_down(MouseButton::Right, move |event, window, cx| {
                window.prevent_default();
                log::info!("pie menu: at {:?}", event.position);
                show(&opener, Some(event.position), cx);
            })
            .children(self.children);
        let Some(at) = state.read(cx).at else {
            return region;
        };
        let takeover = take_focus((self.id.clone(), "takeover"), window, cx);
        let focus = takeover.read(cx).focus.clone();
        let close: Run = {
            let (state, id) = (state.clone(), self.id.clone());
            Rc::new(move |window, cx| {
                log::info!("pie menu {id:?}: closed");
                show(&state, None, cx);
                give_back(&takeover, window, cx);
            })
        };
        if !focus.is_focused(window) {
            close(window, cx);
            return region;
        }
        let items = Rc::new(self.items);
        let count = items.len();
        let run: Pick = {
            let (items, close) = (items.clone(), close.clone());
            Rc::new(move |ix, window, cx| {
                let item = &items[ix];
                log::info!("pie menu: {}", item.label);
                close(window, cx);
                if let Some(on_click) = &item.on_click {
                    on_click(window, cx);
                }
            })
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let rem = window.rem_size();
        let (radius, bubble) = theme.pie();
        let (radius, bubble) = (radius.to_pixels(rem), bubble.to_pixels(rem));
        let hub = bubble * 1.75;
        let span = (radius + bubble) * 2.0;
        let marked = state.read(cx).marked.filter(|&ix| ix < count);
        let bloom =
            Animation::new(motion::duration(motion::BASE, cx)).with_easing(motion::ease_out_cubic);
        let bubbles = items.iter().enumerate().map(|(ix, item)| {
            let angle = ix as f32 * TAU / count as f32 - TAU / 4.0;
            let on = marked == Some(ix);
            let (fg, bg) = if on {
                (colors.on_accent, colors.accent)
            } else {
                (colors.fg_muted, colors.overlay)
            };
            div()
                .absolute()
                .flex()
                .items_center()
                .justify_center()
                .size(bubble)
                .rounded_full()
                .bg(bg)
                .border_1()
                .border_color(if on { colors.accent } else { colors.border })
                .shadow(theme.elevation(Elevation::Floating))
                .child(Icon::new(item.icon).size(IconSize::Md).color(fg))
                .with_animation(
                    (self.id.clone(), format!("bubble-{ix}")),
                    bloom.clone(),
                    move |bubble_el, t| {
                        let late = (ix as f32 * 0.06).min(0.4);
                        let t = ((t - late) / (1.0 - late)).clamp(0.0, 1.0);
                        let reach = radius * motion::spring(t);
                        let middle = span / 2.0;
                        bubble_el
                            .left(middle + reach * angle.cos() - bubble / 2.0)
                            .top(middle + reach * angle.sin() - bubble / 2.0)
                            .opacity(t)
                    },
                )
        });
        let label = marked.map_or(SharedString::from("Cancel"), |ix| items[ix].label.clone());
        let measure = state.clone();
        let ring = div()
            .relative()
            .size(span)
            .child(
                div()
                    .absolute()
                    .left(radius + bubble - hub / 2.0)
                    .top(radius + bubble - hub / 2.0)
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(hub)
                    .rounded_full()
                    .bg(colors.overlay)
                    .border_1()
                    .border_color(colors.border)
                    .shadow(theme.elevation(Elevation::Modal))
                    .text_size(theme.text_size(TextSize::Xs))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if marked.is_some() {
                        colors.fg
                    } else {
                        colors.fg_subtle
                    })
                    .child(label),
            )
            .children(bubbles)
            .child(
                canvas(
                    move |bounds: Bounds<Pixels>, window, cx| {
                        let center = bounds.center();
                        if measure.read(cx).center != center {
                            measure.update(cx, |pie, cx| {
                                pie.center = center;
                                cx.notify();
                                window.request_animation_frame();
                            });
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
        let (aim, pick, keys, lift) = (state.clone(), state.clone(), state.clone(), state.clone());
        let (run_click, run_key) = (run.clone(), run);
        let (close_click, close_right, escape) = (close.clone(), close.clone(), close);
        let viewport = window.viewport_size();
        let catcher = div()
            .id((self.id.clone(), "catcher"))
            .track_focus(&focus)
            .w(viewport.width)
            .h(viewport.height)
            .occlude()
            .on_mouse_move(move |event, _, cx| {
                let pie = aim.read(cx);
                let to = slice(pie.center, event.position, hub, count);
                if pie.marked != to {
                    aim.update(cx, |pie, cx| {
                        pie.marked = to;
                        cx.notify();
                    });
                }
            })
            .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                window.prevent_default();
                let pie = pick.read(cx);
                match slice(pie.center, event.position, hub, count) {
                    Some(ix) => run_click(ix, window, cx),
                    None => close_click(window, cx),
                }
            })
            .on_mouse_down(MouseButton::Right, move |_, window, cx| {
                window.prevent_default();
                close_right(window, cx)
            })
            .on_key_down(move |event, window, cx| {
                let stroke = &event.keystroke;
                let by: isize = match stroke.key.as_str() {
                    "right" | "down" => 1,
                    "left" | "up" => -1,
                    "escape" => {
                        cx.stop_propagation();
                        return escape(window, cx);
                    }
                    "enter" | "space" if !stroke.modifiers.modified() => {
                        return cx.stop_propagation();
                    }
                    _ => return,
                };
                cx.stop_propagation();
                keys.update(cx, |pie, cx| {
                    let from = pie
                        .marked
                        .filter(|&at| at < count)
                        .map_or(if by > 0 { -1 } else { 0 }, |at| at as isize);
                    pie.marked = Some((from + by).rem_euclid(count as isize) as usize);
                    cx.notify();
                });
            })
            .on_key_up(move |event, window, cx| {
                let stroke = &event.keystroke;
                if matches!(stroke.key.as_str(), "enter" | "space") && !stroke.modifiers.modified()
                {
                    cx.stop_propagation();
                    if let Some(ix) = lift.read(cx).marked.filter(|&ix| ix < count) {
                        run_key(ix, window, cx);
                    }
                }
            });
        region.child(
            raise(
                (self.id.clone(), "raised"),
                anchored()
                    .position(point(Pixels::ZERO, Pixels::ZERO))
                    .child(
                        catcher.child(
                            anchored()
                                .position(at - point(span / 2.0, span / 2.0))
                                .snap_to_window()
                                .child(ring),
                        ),
                    ),
            )
            .with_priority(1),
        )
    }
}
