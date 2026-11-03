use std::{collections::HashMap, rc::Rc};

use gpui::{
    AnyElement, App, AppContext, Bounds, Div, DragMoveEvent, ElementId, EmptyView, EntityId,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, Point, RenderOnce,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, div,
};

use crate::{
    primitives::{Measure, tab_stop},
    theme::{ActiveTheme, Radius},
};

type OnSelect = Rc<dyn Fn(&[SharedString], &mut Window, &mut App)>;

/// Where the items sat when last drawn, the item the keyboard is on, where the last press on empty space fell, and the band while a drag draws it, from the box's corner.
#[derive(Default)]
struct Area {
    items: HashMap<SharedString, Bounds<Pixels>>,
    cursor: usize,
    press: Option<Point<Pixels>>,
    band: Option<(Point<Pixels>, Point<Pixels>)>,
}

/// Where a key takes the cursor among `count` items: on through the items' order, or back; the ends hold.
fn stepped(key: &str, at: usize, count: usize) -> Option<usize> {
    match key {
        "right" | "down" => Some((at + 1).min(count - 1)),
        "left" | "up" => Some(at.saturating_sub(1)),
        "home" => Some(0),
        "end" => Some(count - 1),
        _ => None,
    }
}

struct Band {
    owner: EntityId,
    adding: Vec<SharedString>,
}

/// The rectangle between two corners.
fn spanned(from: Point<Pixels>, to: Point<Pixels>) -> Bounds<Pixels> {
    Bounds::from_corners(
        gpui::point(from.x.min(to.x), from.y.min(to.y)),
        gpui::point(from.x.max(to.x), from.y.max(to.y)),
    )
}

/// The keys, in the given order, of the items a band touches, after those it adds to.
fn banded(
    order: &[SharedString],
    items: &HashMap<SharedString, Bounds<Pixels>>,
    band: Bounds<Pixels>,
    adding: &[SharedString],
) -> Vec<SharedString> {
    order
        .iter()
        .filter(|key| {
            adding.contains(key) || items.get(*key).is_some_and(|item| item.intersects(&band))
        })
        .cloned()
        .collect()
}

/// The selection after a press on `key`: it alone, or with Command or Shift, added or dropped.
fn pressed(selected: &[SharedString], key: &SharedString, adding: bool) -> Vec<SharedString> {
    match (adding, selected.contains(key)) {
        (false, _) => vec![key.clone()],
        (true, true) => selected
            .iter()
            .filter(|each| *each != key)
            .cloned()
            .collect(),
        (true, false) => selected.iter().chain([key]).cloned().collect(),
    }
}

/// Items in a box, each under a key. A drag on the box's empty space draws a band that selects what it touches, Shift or Command adding to what was selected, and a press there clears; a press on an item selects it alone, and Command or Shift adds or drops it. The box is one Tab stop: the arrows walk a cursor through the items, ringed while the box has focus, Space adds or drops the one it is on, Escape clears and Command-A takes all. The owner lays the items out through the box's style and hears each new selection.
#[derive(IntoElement)]
pub struct SelectionArea {
    id: ElementId,
    base: Div,
    items: Vec<(SharedString, AnyElement)>,
    selected: Vec<SharedString>,
    on_change: OnSelect,
}

impl SelectionArea {
    pub fn new(
        id: impl Into<ElementId>,
        on_change: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            base: div(),
            items: Vec::new(),
            selected: Vec::new(),
            on_change: Rc::new(on_change),
        }
    }

    pub fn item(mut self, key: impl Into<SharedString>, item: impl IntoElement) -> Self {
        self.items.push((key.into(), item.into_any_element()));
        self
    }

    pub fn selected(mut self, keys: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.selected = keys.into_iter().map(Into::into).collect();
        self
    }
}

impl Styled for SelectionArea {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for SelectionArea {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let on_change = self.on_change;
        let area = window.use_keyed_state((id.clone(), "area"), cx, |_, _| Area::default());
        let owner = area.entity_id();
        let focus = tab_stop((id.clone(), "focus").into(), true, window, cx);
        let focused = focus.is_focused(window);
        let order: Rc<[SharedString]> = self.items.iter().map(|(key, _)| key.clone()).collect();
        let count = order.len();
        let cursor = area.read(cx).cursor.min(count.saturating_sub(1));
        let selected: Rc<[SharedString]> = self.selected.into();
        let theme = cx.theme();
        let (accent, ring, wash, radius) = (
            theme.colors.accent,
            theme.colors.focus,
            theme.colors.selection,
            theme.radius(Radius::Md),
        );
        let tell: OnSelect = Rc::new(move |keys, window, cx| {
            log::info!("selection area: {} selected", keys.len());
            on_change(keys, window, cx)
        });
        let items: Vec<_> = self
            .items
            .into_iter()
            .enumerate()
            .map(|(ix, (key, item))| {
                let on = selected.contains(&key);
                let (seen, press, now, held, keys_to, at) = (
                    area.clone(),
                    tell.clone(),
                    selected.clone(),
                    key.clone(),
                    focus.clone(),
                    area.clone(),
                );
                div()
                    .id((id.clone(), format!("item-{key}")))
                    .rounded(radius)
                    .border_1()
                    .border_color(if focused && ix == cursor {
                        ring
                    } else {
                        gpui::transparent_black()
                    })
                    .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                        cx.stop_propagation();
                        window.focus(&keys_to, cx);
                        at.update(cx, |area, _| area.cursor = ix);
                        let adding = event.modifiers.platform || event.modifiers.shift;
                        press(&pressed(&now, &held, adding), window, cx)
                    })
                    .child(
                        div()
                            .rounded(radius)
                            .border_1()
                            .border_color(if on {
                                accent
                            } else {
                                gpui::transparent_black()
                            })
                            .child(
                                Measure::new(
                                    (id.clone(), format!("measure-{key}")),
                                    move |bounds, _, cx| {
                                        let key = key.clone();
                                        seen.update(cx, |area, _| {
                                            area.items.insert(key, bounds);
                                        })
                                    },
                                )
                                .child(item),
                            ),
                    )
            })
            .collect();
        let band = area.read(cx).band.map(|(from, to)| {
            let lit = spanned(from, to);
            div()
                .debug_selector(|| "selection-band".into())
                .absolute()
                .left(lit.origin.x)
                .top(lit.origin.y)
                .w(lit.size.width)
                .h(lit.size.height)
                .border_1()
                .border_color(accent)
                .bg(wash)
        });
        let (pressed_at, moved, ended, gone, walked) =
            (area.clone(), area.clone(), area.clone(), area.clone(), area);
        let (keys, keyed, emptied) = (tell.clone(), tell.clone(), tell);
        let (all, adding, now) = (order.clone(), selected.to_vec(), selected.clone());
        self.base
            .id(id.clone())
            .relative()
            .track_focus(&focus)
            .on_drag(Band { owner, adding }, |_, _, _, cx| cx.new(|_| EmptyView))
            .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                pressed_at.update(cx, |area, _| area.press = Some(event.position));
            })
            .on_drag_move(move |event: &DragMoveEvent<Band>, window, cx| {
                let drag = event.drag(cx);
                if drag.owner != owner {
                    return;
                }
                let origin = event.bounds.origin;
                let at = event.event.position - origin;
                let Some(press) = moved.read(cx).press else {
                    log::error!("selection area: a band with no press behind it");
                    return;
                };
                let start = press - origin;
                let adding = match event.event.modifiers.platform || event.event.modifiers.shift {
                    true => drag.adding.clone(),
                    false => Vec::new(),
                };
                let lit = spanned(start + origin, at + origin);
                let chosen = banded(&order, &moved.read(cx).items, lit, &adding);
                moved.update(cx, |area, cx| {
                    area.band = Some((start, at));
                    cx.notify();
                });
                keys(&chosen, window, cx);
            })
            .on_mouse_up(MouseButton::Left, move |event, window, cx| {
                let clicked = ended.update(cx, |area, cx| {
                    let clicked = area.band.is_none() && area.press.is_some();
                    area.band = None;
                    area.press = None;
                    cx.notify();
                    clicked
                });
                if clicked && !event.modifiers.platform && !event.modifiers.shift {
                    emptied(&[], window, cx);
                }
            })
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                gone.update(cx, |area, cx| {
                    area.band = None;
                    area.press = None;
                    cx.notify();
                })
            })
            .on_key_down(move |event, window, cx| {
                if count == 0 {
                    return;
                }
                let keystroke = &event.keystroke;
                let key = keystroke.key.as_str();
                if let Some(to) =
                    stepped(key, cursor, count).filter(|_| !keystroke.modifiers.modified())
                {
                    cx.stop_propagation();
                    walked.update(cx, |area, cx| {
                        area.cursor = to;
                        cx.notify();
                    });
                    return;
                }
                match (key, keystroke.modifiers.platform) {
                    ("space", false) => keyed(&pressed(&now, &all[cursor], true), window, cx),
                    ("escape", false) => keyed(&[], window, cx),
                    ("a", true) => keyed(&all, window, cx),
                    _ => return,
                }
                cx.stop_propagation();
            })
            .children(items)
            .children(band)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use gpui::{Bounds, SharedString, point, px, size};

    use super::{banded, pressed, stepped};

    fn keys(list: &[&str]) -> Vec<SharedString> {
        list.iter()
            .map(|key| SharedString::from(key.to_string()))
            .collect()
    }

    #[test]
    fn a_band_takes_what_it_touches_in_order_after_what_it_adds_to() {
        let order = keys(&["a", "b", "c"]);
        let tile = |x: f32| Bounds::new(point(px(x), px(0.0)), size(px(40.0), px(40.0)));
        let items = HashMap::from([
            (order[0].clone(), tile(0.0)),
            (order[1].clone(), tile(50.0)),
            (order[2].clone(), tile(100.0)),
        ]);
        let band = Bounds::new(point(px(45.0), px(10.0)), size(px(60.0), px(5.0)));
        assert_eq!(banded(&order, &items, band, &[]), keys(&["b", "c"]));
        assert_eq!(
            banded(&order, &items, band, &keys(&["a"])),
            keys(&["a", "b", "c"])
        );
    }

    #[test]
    fn keys_walk_the_cursor_and_hold_at_the_ends() {
        assert_eq!(stepped("right", 0, 3), Some(1));
        assert_eq!(stepped("down", 2, 3), Some(2), "the end holds");
        assert_eq!(stepped("left", 0, 3), Some(0));
        assert_eq!(stepped("end", 0, 3), Some(2));
        assert_eq!(stepped("space", 1, 3), None);
    }

    #[test]
    fn a_press_selects_alone_or_adds_and_drops() {
        let now = keys(&["a"]);
        let b = SharedString::from("b");
        assert_eq!(pressed(&now, &b, false), keys(&["b"]));
        assert_eq!(pressed(&now, &b, true), keys(&["a", "b"]));
        assert_eq!(pressed(&keys(&["a", "b"]), &b, true), keys(&["a"]));
    }
}
