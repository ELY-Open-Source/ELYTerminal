use std::rc::Rc;

use gpui::{
    App, Bounds, ElementId, Entity, FontWeight, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window,
    canvas, div, prelude::*,
};

use crate::{
    forms::{Choice, OnValue, float, step, surface},
    primitives::{Icon, IconName, tab_stop},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
};

/// Which entry is open, the link under the keyboard, and where each entry sits.
#[derive(Default)]
struct Open {
    entry: Option<usize>,
    link: usize,
    anchors: Vec<Bounds<Pixels>>,
}

type PickLink = Rc<dyn Fn(usize, usize, &mut Window, &mut App)>;
type Entries = [(SharedString, Vec<Choice>)];

/// The open entry and its marked link, checked against the lists as they are now.
fn marked(open: &Open, entries: &Entries) -> Option<(usize, usize)> {
    let entry = open.entry.filter(|&entry| entry < entries.len())?;
    let links = &entries[entry].1;
    let link = if links.get(open.link).is_some_and(|link| !link.disabled) {
        open.link
    } else {
        step(links, links.len() - 1, 1)
    };
    Some((entry, link))
}

fn show(state: &Entity<Open>, entry: Option<usize>, cx: &mut App) {
    state.update(cx, |open, cx| {
        if open.entry != entry {
            open.link = 0;
        }
        open.entry = entry;
        cx.notify();
    });
}

/// A row of entries; each opens a panel of links. While one is open, hovering another switches to it.
#[derive(IntoElement)]
pub struct NavigationMenu {
    id: ElementId,
    entries: Vec<(SharedString, Vec<Choice>)>,
    on_select: Option<OnValue>,
}

impl NavigationMenu {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            entries: Vec::new(),
            on_select: None,
        }
    }

    /// An entry and its links: each link's label is its title, its note a line of detail.
    pub fn entry(
        mut self,
        label: impl Into<SharedString>,
        links: impl IntoIterator<Item = Choice>,
    ) -> Self {
        let links: Vec<Choice> = links.into_iter().collect();
        assert!(!links.is_empty(), "a navigation menu entry needs links");
        self.entries.push((label.into(), links));
        self
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for NavigationMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let count = self.entries.len();
        let focus = tab_stop((self.id.clone(), "focus").into(), true, window, cx);
        let focused = focus.is_focused(window);
        let state = window.use_keyed_state((self.id.clone(), "open"), cx, |_, _| Open::default());
        if state.read(cx).anchors.len() != count {
            state.update(cx, |open, _| open.anchors = vec![Bounds::default(); count]);
        }
        let current = marked(state.read(cx), &self.entries);
        if state.read(cx).entry.is_some() && (!focused || current.is_none()) {
            log::info!("navigation menu {:?}: closed, focused {focused}", self.id);
            state.update(cx, |open, _| open.entry = None);
        }
        let open = current.filter(|_| focused).map(|(entry, _)| entry);
        let link = current.map_or(0, |(_, link)| link);
        let entries = Rc::new(self.entries);
        let pick: PickLink = {
            let (id, entries, state, on_select) = (
                self.id.clone(),
                entries.clone(),
                state.clone(),
                self.on_select,
            );
            Rc::new(move |entry, link, window, cx| {
                let choice = &entries[entry].1[link];
                if choice.disabled {
                    return;
                }
                let value = choice.value.clone();
                log::info!("navigation menu {id:?}: {value}");
                show(&state, None, cx);
                if let Some(on_select) = &on_select {
                    on_select(&value, window, cx);
                }
            })
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let triggers = entries.iter().enumerate().map(|(ix, (label, _))| {
            let on = open == Some(ix);
            let (press, hover, measure, into) =
                (state.clone(), state.clone(), state.clone(), focus.clone());
            div()
                .id(("entry", ix))
                .relative()
                .flex()
                .items_center()
                .gap_1()
                .h(theme.control_height(ControlSize::Md))
                .px_3()
                .rounded(theme.radius(Radius::Md))
                .border_1()
                .border_color(if focused && on {
                    colors.focus
                } else {
                    gpui::transparent_black()
                })
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(if on { colors.fg } else { colors.fg_muted })
                .when(on, |entry| entry.bg(colors.hover))
                .cursor_pointer()
                .hover(|style| style.text_color(colors.fg))
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    window.prevent_default();
                    window.focus(&into, cx);
                    show(&press, if on { None } else { Some(ix) }, cx);
                })
                .on_hover(move |hovered, _, cx| {
                    let current = hover.read(cx).entry;
                    if *hovered && current.is_some() && current != Some(ix) {
                        show(&hover, Some(ix), cx);
                    }
                })
                .child(label.clone())
                .child(
                    Icon::new(IconName::ChevronDown)
                        .size(IconSize::Xs)
                        .color(colors.fg_subtle),
                )
                .child(
                    canvas(
                        move |bounds, _, cx| {
                            if measure.read(cx).anchors.get(ix) != Some(&bounds) {
                                measure.update(cx, |open, _| open.anchors[ix] = bounds);
                            }
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
                )
        });
        let panel = open.map(|entry| {
            let links = entries[entry].1.iter().enumerate().map(|(ix, choice)| {
                let pick = pick.clone();
                let (fg, muted) = if choice.disabled {
                    (colors.fg_disabled, colors.fg_disabled)
                } else {
                    (colors.fg, colors.fg_muted)
                };
                div()
                    .id(("link", ix))
                    .flex()
                    .gap_3()
                    .p_2()
                    .rounded(theme.radius(Radius::Md))
                    .when(!choice.disabled, |row| {
                        row.when(ix == link, |row| row.bg(colors.hover))
                            .cursor_pointer()
                            .hover(|style| style.bg(colors.hover))
                            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                window.prevent_default();
                                cx.stop_propagation();
                                pick(entry, ix, window, cx);
                            })
                    })
                    .when_some(choice.icon, |row, icon| {
                        row.child(Icon::new(icon).size(IconSize::Md).color(muted))
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(fg)
                                    .child(choice.label.clone()),
                            )
                            .when_some(choice.note.clone(), |words, note| {
                                words.child(
                                    div()
                                        .text_size(theme.text_size(TextSize::Xs))
                                        .text_color(colors.fg_subtle)
                                        .child(note),
                                )
                            }),
                    )
            });
            let close = state.clone();
            let content = surface((self.id.clone(), "panel"), cx)
                .w(theme.nav_panel_width())
                .p_2()
                .grid()
                .grid_cols(2)
                .gap_1()
                .on_mouse_down_out(move |_, _, cx| show(&close, None, cx))
                .children(links);
            let rows = entries[entry].1.len().div_ceil(2) * 2;
            float(
                (self.id.clone(), format!("menu-{entry}")),
                state.read(cx).anchors[entry],
                rows,
                content,
                window,
                cx,
            )
        });
        let (keys, key_entries, enter) = (state.clone(), entries.clone(), pick);
        div()
            .id(self.id)
            .track_focus(&focus)
            .flex()
            .items_center()
            .gap_1()
            .on_key_down(move |event, window, cx| {
                let key = event.keystroke.key.as_str();
                let current = marked(keys.read(cx), &key_entries);
                let entry = current.map(|(entry, _)| entry);
                let next = |by: isize| {
                    (entry.unwrap_or(0) as isize + by).rem_euclid(count as isize) as usize
                };
                match (key, current) {
                    ("right", Some(_)) => show(&keys, Some(next(1)), cx),
                    ("left", Some(_)) => show(&keys, Some(next(-1)), cx),
                    ("down" | "enter" | "space", None) => show(&keys, Some(0), cx),
                    ("down" | "up", Some((at, link))) => {
                        let by = if key == "down" { 1 } else { -1 };
                        let to = step(&key_entries[at].1, link, by);
                        keys.update(cx, |open, cx| {
                            open.link = to;
                            cx.notify();
                        });
                    }
                    ("enter" | "space", Some((at, link))) => enter(at, link, window, cx),
                    ("escape", Some(_)) => show(&keys, None, cx),
                    _ => return,
                }
                cx.stop_propagation();
            })
            .children(triggers)
            .children(panel)
    }
}
