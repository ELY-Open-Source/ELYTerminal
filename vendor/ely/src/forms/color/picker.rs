use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, Context, DragMoveEvent, ElementId, EmptyView, Entity, EntityId, Hsla,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, Point, RenderOnce,
    StatefulInteractiveElement, Styled, Subscription, Window, black, canvas, div,
    linear_color_stop, linear_gradient, prelude::*, relative, white,
};

use super::{
    super::{InputEvent, TextInput},
    ColorSwatch, EyeDropper, Hsva,
    fields::{Commit, Format, SetFormat, format_select, typed_fields},
    hex, parse_hex,
    swatch::checker,
};
use crate::theme::{ActiveTheme, ControlSize, Elevation, Radius};

type OnColor = Rc<dyn Fn(Hsla, &mut Window, &mut App)>;

/// Which part of the picker a drag or click moves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Area {
    Plane,
    Hue,
    Alpha,
}

/// `color` after pointing at `at` inside an area's `bounds`.
pub(crate) fn pointed(color: Hsva, area: Area, bounds: Bounds<Pixels>, at: Point<Pixels>) -> Hsva {
    let x = ((at.x - bounds.left()) / bounds.size.width).clamp(0.0, 1.0);
    let y = ((at.y - bounds.top()) / bounds.size.height).clamp(0.0, 1.0);
    match area {
        Area::Plane => Hsva {
            s: x,
            v: 1.0 - y,
            ..color
        },
        Area::Hue => Hsva {
            h: x * 360.0,
            ..color
        },
        Area::Alpha => Hsva { a: x, ..color },
    }
}

struct Grip {
    owner: EntityId,
    area: Area,
}

/// The picker's own color, which keeps a hue through grays, its hex field, and where its areas sit.
struct Picking {
    color: Hsva,
    committed: Hsla,
    field: Entity<TextInput>,
    format: Format,
    opaque: bool,
    on_change: Option<OnColor>,
    areas: [Bounds<Pixels>; 3],
    _events: Subscription,
}

impl Picking {
    fn commit(&mut self, color: Hsva, window: &mut Window, cx: &mut Context<Self>) {
        let color = if self.opaque {
            Hsva { a: 1.0, ..color }
        } else {
            color
        };
        let value: Hsla = color.to_rgba().into();
        self.color = color;
        self.committed = value;
        log::debug!("color picker: {}", hex(color.to_rgba()));
        if let Some(on_change) = self.on_change.clone() {
            on_change(value, window, cx);
        }
        cx.notify();
    }
}

fn picking(id: &ElementId, value: Hsla, window: &mut Window, cx: &mut App) -> Entity<Picking> {
    window.use_keyed_state(
        (id.clone(), "picking"),
        cx,
        |window, cx: &mut Context<Picking>| {
            let field = cx.new(|cx| TextInput::new(window, cx).max_len(9));
            let events = cx.subscribe_in(&field, window, |picking, field, event, window, cx| {
                if !matches!(event, InputEvent::Submit | InputEvent::Blur) {
                    return;
                }
                let text = field.read(cx).text().to_string();
                match parse_hex(&text) {
                    Ok(rgba) => {
                        let color = Hsva::from_rgba(rgba, picking.color.h);
                        picking.commit(color, window, cx);
                    }
                    Err(problem) => {
                        log::info!("color picker: {problem}; kept the last color");
                        let shown = hex(picking.color.to_rgba());
                        field.update(cx, |field, cx| field.set_text(shown, cx));
                    }
                }
            });
            Picking {
                color: Hsva::from_rgba(value.to_rgb(), 0.0),
                committed: value,
                field,
                format: Format::default(),
                opaque: false,
                on_change: None,
                areas: [Bounds::default(); 3],
                _events: events,
            }
        },
    )
}

/// A color by saturation and value, hue and alpha, or typed as hex.
#[derive(IntoElement)]
pub struct ColorPicker {
    id: ElementId,
    value: Hsla,
    alpha: bool,
    on_change: Option<OnColor>,
}

impl ColorPicker {
    pub fn new(id: impl Into<ElementId>, value: impl Into<Hsla>) -> Self {
        Self {
            id: id.into(),
            value: value.into(),
            alpha: true,
            on_change: None,
        }
    }

    /// Hides the alpha rail; colors stay opaque.
    pub fn opaque(mut self) -> Self {
        self.alpha = false;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(Hsla, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

/// An area that follows drags and jumps on press, recording where it sits.
fn area(
    id: ElementId,
    which: Area,
    state: &Entity<Picking>,
    owner: EntityId,
) -> gpui::Stateful<gpui::Div> {
    let (moved, pressed, measured) = (state.clone(), state.clone(), state.clone());
    div()
        .id(id)
        .relative()
        .w_full()
        .cursor_crosshair()
        .on_drag(Grip { owner, area: which }, |_, _, _, cx| {
            cx.new(|_| EmptyView)
        })
        .on_drag_move(move |event: &DragMoveEvent<Grip>, window, cx| {
            let grip = event.drag(cx);
            if grip.owner != owner || grip.area != which {
                return;
            }
            let (bounds, at) = (event.bounds, event.event.position);
            moved.update(cx, |picking, cx| {
                let next = pointed(picking.color, which, bounds, at);
                picking.commit(next, window, cx);
            });
        })
        .on_mouse_down(MouseButton::Left, move |event, window, cx| {
            let at = event.position;
            pressed.update(cx, |picking, cx| {
                let next = pointed(picking.color, which, picking.areas[which as usize], at);
                picking.commit(next, window, cx);
            });
        })
        .child(
            canvas(
                move |bounds, _, cx| {
                    if measured.read(cx).areas[which as usize] != bounds {
                        measured.update(cx, |picking, _| picking.areas[which as usize] = bounds);
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
}

impl RenderOnce for ColorPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = picking(&self.id, self.value, window, cx);
        let value = if self.alpha {
            self.value
        } else {
            self.value.alpha(1.0)
        };
        state.update(cx, |picking, _| {
            picking.on_change = self.on_change.clone();
            picking.opaque = !self.alpha;
            if picking.committed != value {
                picking.color = Hsva::from_rgba(value.to_rgb(), picking.color.h);
                picking.committed = value;
            }
        });
        let (color, field, format) = {
            let picking = state.read(cx);
            (picking.color, picking.field.clone(), picking.format)
        };
        let shown = hex(color.to_rgba());
        if !field.read(cx).focus().is_focused(window) && field.read(cx).text() != shown {
            field.update(cx, |field, cx| field.set_text(shown, cx));
        }
        let owner = window
            .use_keyed_state((self.id.clone(), "grip"), cx, |_, _| ())
            .entity_id();
        let theme = cx.theme();
        let (knob, rail, radius) = (
            theme.slider_thumb(),
            theme.slider_thumb() * 0.75,
            theme.radius(Radius::Md),
        );
        let current: Hsla = color.to_rgba().into();
        let solid = current.alpha(1.0);
        let hue_at = |degrees: f32| -> Hsla {
            Hsva {
                h: degrees,
                s: 1.0,
                v: 1.0,
                a: 1.0,
            }
            .to_rgba()
            .into()
        };
        let thumb = |fill: Hsla, x: f32| {
            div()
                .absolute()
                .size(knob)
                .left(relative(x))
                .ml(knob * -0.5)
                .rounded_full()
                .border_2()
                .border_color(theme.colors.on_media)
                .bg(fill)
                .shadow(theme.elevation(Elevation::Raised))
        };
        let plane = area(
            (self.id.clone(), "plane").into(),
            Area::Plane,
            &state,
            owner,
        )
        .h(theme.color_plane())
        .rounded(radius)
        .bg(hue_at(color.h))
        .child(
            div()
                .absolute()
                .size_full()
                .rounded(radius)
                .bg(linear_gradient(
                    90.0,
                    linear_color_stop(white(), 0.0),
                    linear_color_stop(white().alpha(0.0), 1.0),
                )),
        )
        .child(
            div()
                .absolute()
                .size_full()
                .rounded(radius)
                .bg(linear_gradient(
                    180.0,
                    linear_color_stop(black().alpha(0.0), 0.0),
                    linear_color_stop(black(), 1.0),
                )),
        )
        .child(
            thumb(solid, color.s)
                .top(relative(1.0 - color.v))
                .mt(knob * -0.5),
        );
        let bands = (0..6).map(|band| {
            div()
                .flex_1()
                .h_full()
                .bg(linear_gradient(
                    90.0,
                    linear_color_stop(hue_at(band as f32 * 60.0), 0.0),
                    linear_color_stop(hue_at((band + 1) as f32 * 60.0), 1.0),
                ))
                .when(band == 0, |first| first.rounded_l(theme.radius(Radius::Sm)))
                .when(band == 5, |last| last.rounded_r(theme.radius(Radius::Sm)))
        });
        let hue = area((self.id.clone(), "hue").into(), Area::Hue, &state, owner)
            .h(rail)
            .flex()
            .children(bands)
            .child(
                thumb(hue_at(color.h), color.h / 360.0)
                    .top(relative(0.5))
                    .mt(knob * -0.5),
            );
        let alpha = self.alpha.then(|| {
            area(
                (self.id.clone(), "alpha").into(),
                Area::Alpha,
                &state,
                owner,
            )
            .h(rail)
            .child(checker(theme.radius(Radius::Sm), cx))
            .child(
                div()
                    .absolute()
                    .size_full()
                    .rounded(theme.radius(Radius::Sm))
                    .bg(linear_gradient(
                        90.0,
                        linear_color_stop(solid.alpha(0.0), 0.0),
                        linear_color_stop(solid, 1.0),
                    )),
            )
            .child(thumb(current, color.a).top(relative(0.5)).mt(knob * -0.5))
        });
        let dropped = state.clone();
        let (typed, formatted) = (state.clone(), state.clone());
        let commit: Commit = Rc::new(move |color, window, cx| {
            typed.update(cx, |picking, cx| picking.commit(color, window, cx))
        });
        let set_format: SetFormat = Rc::new(move |format, _, cx| {
            formatted.update(cx, |picking, cx| {
                picking.format = format;
                cx.notify();
            })
        });
        let row = div()
            .flex()
            .items_center()
            .gap_2()
            .child(ColorSwatch::new((self.id.clone(), "preview"), current).size(ControlSize::Md))
            .child(
                div()
                    .flex_1()
                    .child(format_select(&self.id, format, set_format)),
            )
            .child(EyeDropper::new((self.id.clone(), "dropper")).on_pick(
                move |picked, window, cx| {
                    dropped.update(cx, |picking, cx| {
                        let color = Hsva::from_rgba(picked.to_rgb(), picking.color.h);
                        picking.commit(color, window, cx);
                    })
                },
            ));
        let fields = typed_fields(&self.id, format, color, &field, commit, cx);
        let rails: Vec<AnyElement> = [
            Some(hue.into_any_element()),
            alpha.map(IntoElement::into_any_element),
        ]
        .into_iter()
        .flatten()
        .collect();
        div()
            .id(self.id)
            .flex()
            .flex_col()
            .gap_3()
            .px(knob * 0.5)
            .pt(knob * 0.5)
            .child(plane)
            .children(rails)
            .child(row)
            .child(fields)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, point, px, size};

    use super::{Area, Hsva, pointed};

    #[test]
    fn pointing_sets_the_part_under_the_pointer() {
        let bounds = Bounds::new(point(px(10.0), px(10.0)), size(px(100.0), px(50.0)));
        let gray = Hsva {
            h: 200.0,
            s: 0.0,
            v: 0.5,
            a: 1.0,
        };
        let plane = pointed(gray, Area::Plane, bounds, point(px(60.0), px(20.0)));
        assert_eq!((plane.h, plane.s, plane.v), (200.0, 0.5, 0.8));
        let hue = pointed(gray, Area::Hue, bounds, point(px(200.0), px(0.0)));
        assert_eq!(hue.h, 360.0);
        let alpha = pointed(gray, Area::Alpha, bounds, point(px(35.0), px(30.0)));
        assert_eq!(alpha.a, 0.25);
    }
}
