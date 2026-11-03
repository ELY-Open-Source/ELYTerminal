use std::rc::Rc;

use gpui::{
    App, Bounds, ClickEvent, ElementId, Entity, InteractiveElement, IntoElement, ParentElement,
    Pixels, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, canvas, div,
    prelude::*,
};

use super::{
    Choice,
    options::{OnValues, float, option_row, surface},
    select::{field_button, field_text},
};
use crate::{
    primitives::{Icon, IconName, tab_stop},
    theme::{ActiveTheme, ControlSize, IconSize},
};

/// A node of a cascade: a value, its label, and the nodes under it.
#[derive(Clone, Debug, PartialEq)]
pub struct Cascade {
    pub value: SharedString,
    pub label: SharedString,
    pub children: Vec<Cascade>,
}

impl Cascade {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            children: Vec::new(),
        }
    }

    pub fn child(mut self, node: Cascade) -> Self {
        self.children.push(node);
        self
    }
}

/// The columns a trail opens: the roots, then the children of each step with any.
fn columns<'a>(roots: &'a [Cascade], trail: &[usize]) -> Vec<&'a [Cascade]> {
    let mut out = vec![roots];
    for &ix in trail {
        let node = &out[out.len() - 1][ix];
        if node.children.is_empty() {
            break;
        }
        out.push(&node.children);
    }
    out
}

/// Row indices along `path`, as far as it matches.
fn trail_of(roots: &[Cascade], path: &[SharedString]) -> Vec<usize> {
    let mut trail = Vec::new();
    let mut level = roots;
    for value in path {
        let Some(ix) = level.iter().position(|node| node.value == *value) else {
            break;
        };
        trail.push(ix);
        level = &level[ix].children;
    }
    trail
}

/// `trail` cut back to rows that still exist; never empty.
fn fit_trail(roots: &[Cascade], trail: &[usize]) -> Vec<usize> {
    let mut fitted = Vec::new();
    let mut level = roots;
    for &ix in trail {
        let Some(node) = level.get(ix) else {
            break;
        };
        fitted.push(ix);
        level = &node.children;
    }
    if fitted.is_empty() {
        fitted.push(0);
    }
    fitted
}

/// The nodes a trail passes through.
fn nodes<'a>(roots: &'a [Cascade], trail: &[usize]) -> Vec<&'a Cascade> {
    let mut level = roots;
    trail
        .iter()
        .map(|&ix| {
            let node = &level[ix];
            level = &node.children;
            node
        })
        .collect()
}

#[derive(Default)]
struct Trail {
    open: bool,
    trail: Vec<usize>,
    anchor: Bounds<Pixels>,
}

fn show(state: &Entity<Trail>, open: bool, trail: Vec<usize>, cx: &mut App) {
    state.update(cx, |state, cx| {
        state.open = open;
        state.trail = trail;
        cx.notify();
    });
}

type PickTrail = Rc<dyn Fn(Vec<usize>, &mut Window, &mut App)>;

/// A select over a tree: each column opens the next, and a leaf picks the path.
#[derive(IntoElement)]
pub struct Cascader {
    id: ElementId,
    roots: Vec<Cascade>,
    selected: Vec<SharedString>,
    placeholder: SharedString,
    size: ControlSize,
    on_change: Option<OnValues>,
}

impl Cascader {
    pub fn new(id: impl Into<ElementId>, roots: impl IntoIterator<Item = Cascade>) -> Self {
        Self {
            id: id.into(),
            roots: roots.into_iter().collect(),
            selected: Vec::new(),
            placeholder: SharedString::from("Choose…"),
            size: ControlSize::default(),
            on_change: None,
        }
    }

    /// The chosen path, root first.
    pub fn selected(mut self, path: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.selected = path.into_iter().map(Into::into).collect();
        self
    }

    pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
        self.placeholder = text.into();
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Cascader {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        assert!(
            !self.roots.is_empty(),
            "cascader {:?} has no roots",
            self.id
        );
        let focus = tab_stop((self.id.clone(), "focus").into(), true, window, cx);
        let focused = focus.is_focused(window);
        let state = window.use_keyed_state((self.id.clone(), "trail"), cx, |_, _| Trail::default());
        if state.read(cx).open && !focused {
            state.update(cx, |state, _| state.open = false);
        }
        let roots = Rc::new(self.roots);
        let start = {
            let trail = trail_of(&roots, &self.selected);
            if trail.is_empty() { vec![0] } else { trail }
        };
        let kept = state.read(cx).trail.clone();
        let fitted = fit_trail(&roots, &kept);
        if fitted != kept {
            state.update(cx, |state, _| state.trail = fitted);
        }
        let (open, trail, anchor) = {
            let state = state.read(cx);
            (state.open, state.trail.clone(), state.anchor)
        };
        let pick: PickTrail = {
            let (id, roots, state, on_change) = (
                self.id.clone(),
                roots.clone(),
                state.clone(),
                self.on_change,
            );
            Rc::new(move |trail, window, cx| {
                let path: Vec<SharedString> = nodes(&roots, &trail)
                    .iter()
                    .map(|node| node.value.clone())
                    .collect();
                log::info!("cascader {id:?}: {path:?}");
                show(&state, false, trail, cx);
                if let Some(on_change) = &on_change {
                    on_change(&path, window, cx);
                }
            })
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let shown: Vec<SharedString> = nodes(&roots, &trail_of(&roots, &self.selected))
            .iter()
            .map(|node| node.label.clone())
            .collect();
        let (click, keys, measure) = (state.clone(), state.clone(), state.clone());
        let (enter, key_roots, key_start) = (pick.clone(), roots.clone(), start.clone());
        let lists = open.then(|| {
            let columns = columns(&roots, &trail);
            let deepest = columns.iter().map(|column| column.len()).max().unwrap_or(0);
            let lists = columns.iter().enumerate().map(|(level, column)| {
                let rows = column.iter().enumerate().map(|(ix, node)| {
                    let (hover, pick) = (state.clone(), pick.clone());
                    let at: Vec<usize> = trail[..level].iter().copied().chain([ix]).collect();
                    let leaf = node.children.is_empty();
                    let choice = Choice::new(node.value.clone(), node.label.clone());
                    option_row(
                        ("row", ix),
                        &choice,
                        trail.get(level) == Some(&ix),
                        None,
                        cx,
                    )
                    .when(!leaf, |row| {
                        row.child(
                            Icon::new(IconName::ChevronRight)
                                .size(IconSize::Xs)
                                .color(colors.fg_subtle),
                        )
                    })
                    .on_hover({
                        let at = at.clone();
                        move |hovered, _, cx| {
                            if *hovered && hover.read(cx).trail != at {
                                show(&hover, true, at.clone(), cx);
                            }
                        }
                    })
                    .when(leaf, |row| {
                        row.on_click(move |_, window, cx| pick(at.clone(), window, cx))
                    })
                });
                div()
                    .id(("column", level))
                    .w(theme.tooltip_max_width() * 0.75)
                    .max_h(theme.list_max_height())
                    .overflow_y_scroll()
                    .p_1()
                    .flex()
                    .flex_col()
                    .when(level > 0, |list| {
                        list.border_l_1().border_color(colors.border)
                    })
                    .children(rows)
            });
            let close = state.clone();
            float(
                self.id.clone(),
                anchor,
                deepest,
                surface((self.id.clone(), "columns"), cx)
                    .flex()
                    .on_mouse_down_out(move |_, _, cx| {
                        let trail = close.read(cx).trail.clone();
                        show(&close, false, trail, cx)
                    })
                    .children(lists),
                window,
                cx,
            )
        });
        field_button(self.id, &focus, self.size, false, window, cx)
            .on_click(move |event, _, cx| {
                if matches!(event, ClickEvent::Mouse(_)) {
                    show(&click, !open, start.clone(), cx)
                }
            })
            .on_key_down(move |event, window, cx| {
                let key = event.keystroke.key.as_str();
                let mut trail = keys.read(cx).trail.clone();
                if !open {
                    if matches!(key, "down" | "up" | "enter" | "space") {
                        cx.stop_propagation();
                        show(&keys, true, key_start.clone(), cx);
                    }
                    return;
                }
                let columns = columns(&key_roots, &trail);
                let level = trail.len() - 1;
                let (at, count) = (trail[level], columns[level].len());
                let node = &columns[level][at];
                match key {
                    "down" => trail[level] = (at + 1) % count,
                    "up" => trail[level] = (at + count - 1) % count,
                    "right" if !node.children.is_empty() => trail.push(0),
                    "left" if level > 0 => {
                        trail.pop();
                    }
                    "enter" | "space" if node.children.is_empty() => {
                        cx.stop_propagation();
                        return enter(trail, window, cx);
                    }
                    "enter" | "space" => trail.push(0),
                    "escape" => {
                        cx.stop_propagation();
                        return show(&keys, false, trail, cx);
                    }
                    _ => return,
                }
                cx.stop_propagation();
                show(&keys, true, trail, cx);
            })
            .child(field_text(
                (!shown.is_empty()).then(|| SharedString::from(shown.join(" / "))),
                self.placeholder,
                false,
                cx,
            ))
            .child(
                Icon::new(IconName::ChevronsUpDown)
                    .size(IconSize::Xs)
                    .color(colors.fg_subtle),
            )
            .child(
                canvas(
                    move |bounds, _, cx| {
                        if measure.read(cx).anchor != bounds {
                            measure.update(cx, |state, _| state.anchor = bounds);
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .children(lists)
    }
}

#[cfg(test)]
mod tests {
    use gpui::SharedString;

    use super::{Cascade, columns, fit_trail, nodes, trail_of};

    fn world() -> Vec<Cascade> {
        vec![
            Cascade::new("asia", "Asia").child(
                Cascade::new("jp", "Japan")
                    .child(Cascade::new("tyo", "Tokyo"))
                    .child(Cascade::new("osa", "Osaka")),
            ),
            Cascade::new("eu", "Europe").child(Cascade::new("pt", "Portugal")),
        ]
    }

    #[test]
    fn a_trail_opens_one_column_per_parent() {
        let roots = world();
        assert_eq!(columns(&roots, &[0]).len(), 2);
        assert_eq!(columns(&roots, &[0, 0]).len(), 3);
        assert_eq!(columns(&roots, &[0, 0, 1]).len(), 3);
    }

    #[test]
    fn a_trail_is_cut_back_to_rows_that_exist() {
        let roots = world();
        assert_eq!(fit_trail(&roots, &[0, 0, 1]), [0, 0, 1]);
        assert_eq!(fit_trail(&roots, &[1, 5]), [1]);
        assert_eq!(fit_trail(&roots, &[9]), [0]);
        assert_eq!(fit_trail(&roots, &[]), [0]);
    }

    #[test]
    fn paths_and_trails_round_trip() {
        let roots = world();
        let path: Vec<SharedString> = vec!["asia".into(), "jp".into(), "osa".into()];
        let trail = trail_of(&roots, &path);
        assert_eq!(trail, [0, 0, 1]);
        let labels: Vec<_> = nodes(&roots, &trail)
            .iter()
            .map(|node| node.label.clone())
            .collect();
        assert_eq!(labels, ["Asia", "Japan", "Osaka"]);
        assert_eq!(trail_of(&roots, &["eu".into(), "xx".into()]), [1]);
    }
}
