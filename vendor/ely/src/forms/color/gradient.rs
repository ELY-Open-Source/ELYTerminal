use std::rc::Rc;

use gpui::{
    App, Bounds, DragMoveEvent, ElementId, EmptyView, Entity, EntityId, Hsla, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Pixels, RenderOnce, StatefulInteractiveElement,
    Styled, Window, canvas, div, linear_color_stop, linear_gradient, prelude::*, relative,
};

use super::{ColorPicker, swatch::checker};
use crate::{
    layout::seeded::{Seeded, use_seeded},
    primitives::tab_stop,
    theme::{ActiveTheme, ControlSize, Mix, Radius},
};

/// A color at a place along a gradient, from 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientStop {
    pub at: f32,
    pub color: Hsla,
}

/// The color at `t` along stops sorted by place.
pub(crate) fn color_at(stops: &[GradientStop], t: f32) -> Hsla {
    let first = stops.first().expect("a gradient has stops");
    if t <= first.at {
        return first.color;
    }
    for pair in stops.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        if t <= to.at {
            let span = (to.at - from.at).max(f32::EPSILON);
            return from.color.mix(&to.color, (t - from.at) / span);
        }
    }
    stops.last().expect("a gradient has stops").color
}

/// `stops` with `ix` moved to `t`, still sorted, and where `ix` landed.
pub(crate) fn moved(stops: &[GradientStop], ix: usize, t: f32) -> (Vec<GradientStop>, usize) {
    let mut next = stops.to_vec();
    let mut stop = next.remove(ix);
    stop.at = t.clamp(0.0, 1.0);
    let landed = next.partition_point(|other| other.at <= stop.at);
    next.insert(landed, stop);
    (next, landed)
}

/// `stops` with a new stop at `t`, colored as the gradient is there.
pub(crate) fn added(stops: &[GradientStop], t: f32) -> (Vec<GradientStop>, usize) {
    let stop = GradientStop {
        at: t.clamp(0.0, 1.0),
        color: color_at(stops, t),
    };
    let mut next = stops.to_vec();
    let landed = next.partition_point(|other| other.at <= stop.at);
    next.insert(landed, stop);
    (next, landed)
}

/// A stop handle being dragged; the grabbed stop is the chosen one.
struct Handle {
    owner: EntityId,
}

type OnStops = Rc<dyn Fn(&[GradientStop], &mut Window, &mut App)>;
type SetStops = Rc<dyn Fn(Vec<GradientStop>, usize, &mut Window, &mut App)>;
type Live = Entity<Seeded<Vec<GradientStop>>>;

/// The stops and the chosen index as they stand when a handler runs.
fn now(live: &Live, chosen: &Entity<usize>, cx: &App) -> (Vec<GradientStop>, usize) {
    let stops = live.read(cx).value.clone();
    let at = (*chosen.read(cx)).min(stops.len() - 1);
    (stops, at)
}

/// A gradient bar with stops beneath. Drag a stop, click the bar to add one, Delete removes it.
#[derive(IntoElement)]
pub struct GradientEditor {
    id: ElementId,
    stops: Vec<GradientStop>,
    on_change: Option<OnStops>,
}

impl GradientEditor {
    pub fn new(id: impl Into<ElementId>, stops: impl IntoIterator<Item = GradientStop>) -> Self {
        let stops: Vec<GradientStop> = stops.into_iter().collect();
        assert!(stops.len() >= 2, "a gradient needs two stops");
        assert!(
            stops.windows(2).all(|pair| pair[0].at <= pair[1].at)
                && stops.iter().all(|stop| (0.0..=1.0).contains(&stop.at)),
            "gradient stops must be sorted within 0 to 1"
        );
        Self {
            id: id.into(),
            stops,
            on_change: None,
        }
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(&[GradientStop], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for GradientEditor {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let live = use_seeded((self.id.clone(), "stops"), self.stops, window, cx);
        let chosen = window.use_keyed_state((self.id.clone(), "chosen"), cx, |_, _| 0usize);
        let (stops, at) = now(&live, &chosen, cx);
        let count = stops.len();
        let bar = window.use_keyed_state((self.id.clone(), "bar"), cx, |_, _| {
            Bounds::<Pixels>::default()
        });
        let owner = window
            .use_keyed_state((self.id.clone(), "drag"), cx, |_, _| ())
            .entity_id();
        let focus = tab_stop((self.id.clone(), "focus").into(), true, window, cx);
        let focused = focus.is_focused(window);
        let commit: SetStops = {
            let (id, live, chosen, on_change) = (
                self.id.clone(),
                live.clone(),
                chosen.clone(),
                self.on_change,
            );
            Rc::new(move |next, ix, window, cx| {
                log::info!(
                    "gradient editor {id:?}: {} stops, stop {ix} chosen",
                    next.len()
                );
                chosen.update(cx, |chosen, cx| {
                    *chosen = ix;
                    cx.notify();
                });
                live.update(cx, |live, cx| {
                    live.value = next.clone();
                    cx.notify();
                });
                if let Some(on_change) = &on_change {
                    on_change(&next, window, cx);
                }
            })
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let (knob, small) = (theme.slider_thumb(), theme.radius(Radius::Sm));
        let mut spans: Vec<(f32, f32, Hsla, Hsla)> = Vec::new();
        let (first, last) = (stops[0], stops[count - 1]);
        if first.at > 0.0 {
            spans.push((0.0, first.at, first.color, first.color));
        }
        spans.extend(
            stops
                .windows(2)
                .map(|pair| (pair[0].at, pair[1].at, pair[0].color, pair[1].color)),
        );
        if last.at < 1.0 {
            spans.push((last.at, 1.0, last.color, last.color));
        }
        let edge = spans.len() - 1;
        let bands = spans.into_iter().enumerate().map(|(ix, (from, to, a, b))| {
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(relative(from))
                .w(relative(to - from))
                .bg(linear_gradient(
                    90.0,
                    linear_color_stop(a, 0.0),
                    linear_color_stop(b, 1.0),
                ))
                .when(ix == 0, |band| band.rounded_l(small))
                .when(ix == edge, |band| band.rounded_r(small))
        });
        let (measure, press) = (bar.clone(), bar.clone());
        let (add_live, add_chosen, add) = (live.clone(), chosen.clone(), commit.clone());
        let strip = div()
            .id((self.id.clone(), "bar"))
            .relative()
            .w_full()
            .h(theme.control_height(ControlSize::Md))
            .cursor_crosshair()
            .child(checker(small, cx))
            .children(bands)
            .child(
                canvas(
                    move |bounds, _, cx| {
                        if *measure.read(cx) != bounds {
                            measure.update(cx, |bar, _| *bar = bounds);
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                let bounds = *press.read(cx);
                let t = (event.position.x - bounds.left()) / bounds.size.width;
                let (stops, _) = now(&add_live, &add_chosen, cx);
                let (next, ix) = added(&stops, t);
                add(next, ix, window, cx);
            });
        let handles = stops.iter().enumerate().map(|(ix, stop)| {
            let pick = chosen.clone();
            div()
                .id(("stop", ix))
                .absolute()
                .top_0()
                .left(relative(stop.at))
                .ml(knob * -0.5)
                .size(knob)
                .rounded(small)
                .border_2()
                .border_color(if ix == at {
                    if focused { colors.focus } else { colors.accent }
                } else {
                    colors.border_strong
                })
                .bg(stop.color)
                .cursor_grab()
                .on_drag(Handle { owner }, |_, _, _, cx| cx.new(|_| EmptyView))
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    pick.update(cx, |chosen, cx| {
                        *chosen = ix;
                        cx.notify();
                    })
                })
        });
        let (drag_live, drag_chosen, drag) = (live.clone(), chosen.clone(), commit.clone());
        let (key_live, key_chosen, keys) = (live.clone(), chosen.clone(), commit.clone());
        let rail = div()
            .id((self.id.clone(), "stops"))
            .track_focus(&focus)
            .relative()
            .w_full()
            .h(knob)
            .on_drag_move(move |event: &DragMoveEvent<Handle>, window, cx| {
                if event.drag(cx).owner != owner {
                    return;
                }
                let bounds = event.bounds;
                let t = (event.event.position.x - bounds.left()) / bounds.size.width;
                let (stops, at) = now(&drag_live, &drag_chosen, cx);
                let (next, ix) = moved(&stops, at, t);
                drag(next, ix, window, cx);
            })
            .on_key_down(move |event, window, cx| {
                let (stops, at) = now(&key_live, &key_chosen, cx);
                let (next, ix) = match event.keystroke.key.as_str() {
                    "left" => moved(&stops, at, stops[at].at - 0.01),
                    "right" => moved(&stops, at, stops[at].at + 0.01),
                    "delete" | "backspace" if stops.len() > 2 => {
                        let mut next = stops;
                        next.remove(at);
                        (next, at.saturating_sub(1))
                    }
                    _ => return,
                };
                cx.stop_propagation();
                keys(next, ix, window, cx);
            })
            .children(handles);
        let (edit_live, edit_chosen, edit) = (live, chosen, commit);
        div()
            .id(self.id.clone())
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .px(knob * 0.5)
                    .child(strip)
                    .child(rail),
            )
            .child(
                ColorPicker::new((self.id, "color"), stops[at].color).on_change(
                    move |color, window, cx| {
                        let (mut next, at) = now(&edit_live, &edit_chosen, cx);
                        next[at].color = color;
                        edit(next, at, window, cx);
                    },
                ),
            )
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Hsla, black, white};

    use super::{GradientStop, added, color_at, moved};

    fn stops() -> Vec<GradientStop> {
        vec![
            GradientStop {
                at: 0.0,
                color: black(),
            },
            GradientStop {
                at: 1.0,
                color: white(),
            },
        ]
    }

    #[test]
    fn colors_blend_between_stops() {
        let middle: Hsla = color_at(&stops(), 0.5);
        assert!((middle.l - 0.5).abs() < 0.05);
        assert_eq!(color_at(&stops(), -1.0), black());
    }

    #[test]
    fn adding_and_moving_keep_stops_sorted() {
        let (three, ix) = added(&stops(), 0.25);
        assert_eq!((three.len(), ix, three[1].at), (3, 1, 0.25));
        let (swapped, landed) = moved(&three, 0, 0.5);
        assert_eq!(landed, 1);
        assert_eq!(
            swapped.iter().map(|stop| stop.at).collect::<Vec<_>>(),
            [0.25, 0.5, 1.0]
        );
    }
}
