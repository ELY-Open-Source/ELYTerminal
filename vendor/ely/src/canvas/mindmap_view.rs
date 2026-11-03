use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, RenderOnce, SharedString, Styled, Window, canvas, div, prelude::*,
};

use super::{
    gesture::OnPair,
    mindmap::{Placed, Topic, laid, stepped},
    paint::{finish, in_view, wire},
    plane::{InfiniteCanvas, OnViewport},
    view::Viewport,
    writing,
};
use crate::{
    forms::Editing,
    primitives::{FocusNext, tab_stop},
    theme::{ActiveTheme, Radius, TextSize},
    typography::Ellipsis,
};

type OnTopic = Rc<dyn Fn(Option<&SharedString>, &mut Window, &mut App)>;
type OnKey = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// Topics branching from a root on an endless plane, half to each side. A press selects a topic; arrows walk, Tab asks for a child, Enter for a sibling, Delete or Backspace to remove one, and F2 or a double press rewrites it; Escape lets the selection go, so Tab moves on. While a topic is being rewritten its field takes every key; Tab keeps the words and hands focus back to the map. The owner keeps the map and answers each ask with `Topic::adding`, `removing` and `renaming`.
#[derive(IntoElement)]
pub struct MindMap {
    id: ElementId,
    root: Topic,
    viewport: Viewport,
    selected: Option<SharedString>,
    on_select: Option<OnTopic>,
    on_add: Option<OnKey>,
    on_remove: Option<OnKey>,
    on_rename: Option<OnPair>,
    on_viewport: Option<OnViewport>,
}

impl MindMap {
    pub fn new(id: impl Into<ElementId>, root: Topic, viewport: Viewport) -> Self {
        Self {
            id: id.into(),
            root,
            viewport,
            selected: None,
            on_select: None,
            on_add: None,
            on_remove: None,
            on_rename: None,
            on_viewport: None,
        }
    }

    pub fn selected(mut self, key: Option<impl Into<SharedString>>) -> Self {
        self.selected = key.map(Into::into);
        self
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(Option<&SharedString>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Gets the key of the topic a new one goes under.
    pub fn on_add(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_add = Some(Rc::new(handler));
        self
    }

    pub fn on_remove(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_remove = Some(Rc::new(handler));
        self
    }

    pub fn on_rename(
        mut self,
        handler: impl Fn(&SharedString, &SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_rename = Some(Rc::new(handler));
        self
    }

    pub fn on_viewport(
        mut self,
        handler: impl Fn(Viewport, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_viewport = Some(Rc::new(handler));
        self
    }
}

/// Where a branch leaves `parent` and meets `child`, in canvas units.
fn branch(parent: &Placed, child: &Placed) -> ((f32, f32), (f32, f32)) {
    let (p, c) = (parent.frame, child.frame);
    if child.left {
        ((p.x, p.center().1), (c.right(), c.center().1))
    } else {
        ((p.right(), p.center().1), (c.x, c.center().1))
    }
}

impl RenderOnce for MindMap {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let view = self.viewport;
        let placed = Rc::new(laid(&self.root));
        let focus = tab_stop((id.clone(), "focus").into(), true, window, cx);
        let editing =
            window.use_keyed_state((id.clone(), "editing"), cx, |_, _| Editing::default());
        let selected = self.selected.clone().filter(|key| {
            let placed = placed.iter().any(|topic| topic.key == *key);
            if !placed {
                log::error!("mind map {id:?}: no topic {key}; none selected");
            }
            placed
        });
        let rename = {
            let (editing, placed, on_rename, home) = (
                editing.clone(),
                placed.clone(),
                self.on_rename.clone(),
                focus.clone(),
            );
            move |key: &SharedString, window: &mut Window, cx: &mut App| {
                let topic = placed
                    .iter()
                    .find(|topic| topic.key == *key)
                    .expect("a placed topic");
                let words = topic.text.to_string();
                writing::begin(
                    &editing,
                    key.clone(),
                    words,
                    on_rename.clone(),
                    home.clone(),
                    window,
                    cx,
                );
            }
        };
        let rename = Rc::new(rename);
        let (on_select, on_add, on_remove) = (self.on_select, self.on_add, self.on_remove);
        let theme = cx.theme();
        let rem = window.rem_size();
        let text = theme.text_size(TextSize::Sm).to_pixels(rem) * view.zoom;
        let (colors, radius) = (
            theme.colors.clone(),
            theme.radius(Radius::Lg).to_pixels(rem) * view.zoom,
        );
        let stroke = theme.canvas().stroke.to_pixels(rem);
        let topics: Vec<AnyElement> = placed
            .iter()
            .map(|topic| {
                let frame = in_view(&view, &topic.frame);
                let chosen = selected.as_ref() == Some(&topic.key);
                let root = topic.parent.is_none();
                let (key, pick, write, hold) = (
                    topic.key.clone(),
                    on_select.clone(),
                    rename.clone(),
                    focus.clone(),
                );
                div()
                    .id((id.clone(), format!("topic-{}", topic.key)))
                    .absolute()
                    .left(Pixels::from(frame.x))
                    .top(Pixels::from(frame.y))
                    .w(Pixels::from(frame.w))
                    .h(Pixels::from(frame.h))
                    .flex()
                    .items_center()
                    .justify_center()
                    .px(text * 0.75)
                    .rounded(radius)
                    .border_1()
                    .border_color(if chosen {
                        colors.accent
                    } else if root {
                        colors.border_strong
                    } else {
                        colors.border
                    })
                    .bg(colors.surface)
                    .text_size(text)
                    .line_height(text * 1.3)
                    .text_color(colors.fg)
                    .when(root, |topic| topic.font_weight(FontWeight::SEMIBOLD))
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                        cx.stop_propagation();
                        window.focus(&hold, cx);
                        if let Some(pick) = &pick {
                            pick(Some(&key), window, cx);
                        }
                        if event.click_count >= 2 {
                            window.prevent_default();
                            write(&key, window, cx);
                        }
                    })
                    .child(Ellipsis::new(topic.text.clone()))
                    .into_any_element()
            })
            .collect();
        let branches = {
            let (placed, ink) = (placed.clone(), colors.border_strong);
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    for child in placed.iter() {
                        let Some(parent) = child.parent.as_ref() else {
                            continue;
                        };
                        let parent = placed
                            .iter()
                            .find(|topic| topic.key == *parent)
                            .expect("a placed parent");
                        let (from, to) = branch(parent, child);
                        let (from, to) = (view.to_view(from), view.to_view(to));
                        finish(
                            wire(from, to, (to.0 - from.0) / 2.0, bounds.origin, stroke),
                            ink,
                            window,
                        );
                    }
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full()
        };
        let field = selected
            .as_ref()
            .and_then(|key| placed.iter().find(|topic| topic.key == *key))
            .and_then(|topic| writing::field(&editing, in_view(&view, &topic.frame), cx));
        let layer = div()
            .absolute()
            .inset_0()
            .child(branches)
            .children(topics)
            .children(field);
        let plane = InfiniteCanvas::new((id.clone(), "plane"), view).layer(layer);
        let plane = match self.on_viewport {
            Some(on_viewport) => {
                plane.on_viewport(move |next, window, cx| on_viewport(next, window, cx))
            }
            None => plane,
        };
        let (keys, tabbed, parent_of) = (placed.clone(), selected.clone(), self.root.clone());
        let (adding, writing_tab, writing_keys) =
            (on_add.clone(), editing.clone(), editing.clone());
        let back = focus.clone();
        div()
            .id(id)
            .track_focus(&focus)
            .relative()
            .size_full()
            .when(selected.is_some(), |map| {
                map.capture_action(move |_: &FocusNext, window, cx| {
                    let (Some(key), Some(on_add)) = (&tabbed, &adding) else {
                        return;
                    };
                    if writing_tab.read(cx).field().is_some() {
                        cx.stop_propagation();
                        writing_tab.update(cx, |editing, cx| editing.finish(false, window, cx));
                        window.focus(&back, cx);
                        return;
                    }
                    cx.stop_propagation();
                    log::info!("mind map: a child under {key}");
                    on_add(key, window, cx);
                })
            })
            .on_key_down(move |event, window, cx| {
                let Some(at) = &selected else {
                    return;
                };
                if writing_keys.read(cx).field().is_some() {
                    return;
                }
                let key = event.keystroke.key.as_str();
                match key {
                    "up" | "down" | "left" | "right" => {
                        cx.stop_propagation();
                        if let (Some(next), Some(pick)) = (stepped(&keys, at, key), &on_select) {
                            pick(Some(&next), window, cx);
                        }
                    }
                    "enter" => {
                        cx.stop_propagation();
                        let under = parent_of.parent_of(at).unwrap_or_else(|| at.clone());
                        log::info!("mind map: a topic under {under}");
                        if let Some(on_add) = &on_add {
                            on_add(&under, window, cx);
                        }
                    }
                    "backspace" | "delete" if parent_of.key != *at => {
                        cx.stop_propagation();
                        log::info!("mind map: remove {at}");
                        if let Some(on_remove) = &on_remove {
                            on_remove(at, window, cx);
                        }
                    }
                    "f2" => {
                        cx.stop_propagation();
                        rename(at, window, cx);
                    }
                    "escape" => {
                        cx.stop_propagation();
                        if let Some(pick) = &on_select {
                            pick(None, window, cx);
                        }
                    }
                    _ => {}
                }
            })
            .child(plane)
    }
}
