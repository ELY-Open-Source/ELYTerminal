use std::rc::Rc;

use gpui::{
    Anchor, Animation, AnimationExt, AnyElement, App, Bounds, ElementId, Entity,
    InteractiveElement, IntoElement, ParentElement, Pixels, Point, SharedString, Styled, Task,
    Window, anchored, div, prelude::*,
};

use super::{
    draw::{self, measure},
    model::{Kind, Menu},
};
use crate::{
    forms::{Down, Enter, Left, Right, Run, TextInput, Up, float},
    motion,
    primitives::{Icon, IconName, Takeover, give_back, raise, take_focus},
    theme::{ActiveTheme, IconSize},
};

/// The menu open at `depth`, following the marked rows above it; `None` once those rows changed.
pub(super) fn at_depth<'a>(menu: &'a Menu, levels: &[Level], depth: usize) -> Option<&'a Menu> {
    levels.get(..depth)?.iter().try_fold(menu, |menu, level| {
        match level
            .at
            .and_then(|ix| menu.item_at(ix, level.column))
            .map(|item| &item.kind)
        {
            Some(Kind::Sub(sub)) => Some(sub),
            _ => None,
        }
    })
}

/// Fits open levels to the menu as it is now: a mark on a row gone or disabled clears, and levels under a row that no longer opens them close.
fn settle(menu: &Menu, levels: &mut Vec<Level>) {
    let mut here = menu;
    for depth in 0..levels.len() {
        let (column, name) = (levels[depth].column, levels[depth].name.clone());
        // A mark holds its item, not just its place.
        let at = levels[depth].at.filter(|&ix| {
            here.item_at(ix, column)
                .is_some_and(|item| !item.disabled && item.label == name)
        });
        levels[depth].at = at;
        match at
            .and_then(|ix| here.item_at(ix, column))
            .map(|item| &item.kind)
        {
            Some(Kind::Sub(sub)) if depth + 1 < levels.len() => here = sub,
            _ => {
                levels.truncate(depth + 1);
                return;
            }
        }
    }
}

/// An open level: marked row, button, its label, and boxes.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Level {
    pub(super) at: Option<usize>,
    pub(super) column: usize,
    pub(super) name: SharedString,
    pub(super) row: Bounds<Pixels>,
    pub(super) panel: Bounds<Pixels>,
}

/// Where an open menu hangs: under its anchor box, or at a point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Spot {
    Under,
    At(Point<Pixels>),
}

/// A host's menu: where it hangs, its open levels root first, the host's box, the box it hangs under, and the focus to hand back.
#[derive(Default)]
pub(crate) struct Open {
    spot: Option<Spot>,
    pub(super) levels: Vec<Level>,
    host: Bounds<Pixels>,
    anchor: Bounds<Pixels>,
    takeover: Option<Entity<Takeover>>,
    /// The pointer's last place over a row.
    pub(super) pointer: Option<Point<Pixels>>,
    /// The row under the pointer: depth, index and column.
    pub(super) over: Option<(usize, (usize, usize))>,
    /// A delayed mark; dropping it cancels.
    pub(super) pending: Option<Task<()>>,
}

impl Open {
    pub(crate) fn is_open(&self) -> bool {
        self.spot.is_some()
    }

    /// Hangs the menu under `anchor`, kept current as its owner moves.
    pub(crate) fn set_anchor(state: &Entity<Open>, anchor: Bounds<Pixels>, cx: &mut App) {
        if state.read(cx).anchor != anchor {
            state.update(cx, |open, cx| {
                open.anchor = anchor;
                cx.notify();
            });
        }
    }

    /// Opens at `spot`, with the first row marked when `marked`.
    pub(crate) fn show(state: &Entity<Open>, menu: &Menu, spot: Spot, marked: bool, cx: &mut App) {
        state.update(cx, |open, _| open.spot = Some(spot));
        Self::begin(state, menu, menu.step(None, 1).filter(|_| marked), cx);
    }

    /// Starts over on new rows with the first marked; the menu stays where it hangs.
    pub(crate) fn restart(state: &Entity<Open>, menu: &Menu, cx: &mut App) {
        Self::begin(state, menu, menu.step(None, 1), cx);
    }

    /// Fresh levels marking `first`; the old hover is dropped.
    fn begin(state: &Entity<Open>, menu: &Menu, first: Option<usize>, cx: &mut App) {
        state.update(cx, |open, cx| {
            open.levels = vec![first_of(menu, first)];
            (open.pointer, open.over, open.pending) = (None, None, None);
            cx.notify();
        });
    }

    /// Closes and hands focus back to whatever held it before the menu opened.
    pub(crate) fn close(state: &Entity<Open>, window: &mut Window, cx: &mut App) {
        let takeover = state.update(cx, |open, cx| {
            open.spot = None;
            open.levels.clear();
            (open.pointer, open.over, open.pending) = (None, None, None);
            cx.notify();
            open.takeover.take()
        });
        if let Some(takeover) = takeover {
            give_back(&takeover, window, cx);
        }
    }
}

/// A level marking row `at` at its first enabled button.
fn first_of(menu: &Menu, at: Option<usize>) -> Level {
    let Some(ix) = at else {
        return Level::default();
    };
    let column = menu.first(ix).expect("a stepped row runs");
    Level {
        at,
        column,
        name: menu
            .item_at(ix, column)
            .expect("its first spot")
            .label
            .clone(),
        ..Level::default()
    }
}

/// Marks a spot at `depth`; `open_sub` opens its submenu.
pub(super) fn mark(
    state: &Entity<Open>,
    menu: &Menu,
    depth: usize,
    (at, column): (usize, usize),
    open_sub: bool,
    cx: &mut App,
) {
    state.update(cx, |open, cx| {
        if open.levels.len() <= depth {
            return log::debug!("menu: level {depth} closed before its mark");
        }
        let Some(item) =
            at_depth(menu, &open.levels, depth).and_then(|here| here.item_at(at, column))
        else {
            return;
        };
        let (sub, name) = (matches!(item.kind, Kind::Sub(_)), item.label.clone());
        open.levels.truncate(depth + 1);
        let level = &mut open.levels[depth];
        if level.at != Some(at) {
            level.row = Bounds::default();
        }
        (level.at, level.column, level.name) = (Some(at), column, name);
        if open_sub && sub {
            open.levels.push(Level::default());
        }
        cx.notify();
    });
}

/// Runs a spot; a submenu opens, others close first.
pub(super) fn choose(
    state: &Entity<Open>,
    menu: &Menu,
    depth: usize,
    (at, column): (usize, usize),
    close: &Run,
    window: &mut Window,
    cx: &mut App,
) {
    let levels = &state.read(cx).levels;
    let Some(item) = at_depth(menu, levels, depth)
        .filter(|_| levels.len() > depth)
        .and_then(|here| here.item_at(at, column))
        .cloned()
    else {
        log::debug!(
            "menu: a press from an older frame found row {at}:{column} of level {depth} gone"
        );
        return;
    };
    if item.disabled {
        return;
    }
    state.update(cx, |open, _| open.pending = None);
    if let Kind::Sub(sub) = &item.kind {
        let first = first_of(sub, sub.step(None, 1));
        mark(state, menu, depth, (at, column), false, cx);
        state.update(cx, |open, cx| {
            open.levels.push(first);
            cx.notify();
        });
        return;
    }
    log::info!("menu: {}", item.label);
    close(window, cx);
    if let Some(on_click) = &item.on_click {
        on_click(window, cx);
    }
}

/// Arrows, Enter, Space and Escape drive the deepest level.
fn keys(
    menu: &Menu,
    state: &Entity<Open>,
    key: &str,
    close: &Run,
    window: &mut Window,
    cx: &mut App,
) -> bool {
    let levels = state.read(cx).levels.clone();
    let Some(depth) = levels.len().checked_sub(1) else {
        return false;
    };
    let Some(here) = at_depth(menu, &levels, depth) else {
        return false;
    };
    state.update(cx, |open, _| open.pending = None);
    let (at, column) = (levels[depth].at, levels[depth].column);
    let sub = at
        .and_then(|at| here.item_at(at, column))
        .is_some_and(|item| matches!(item.kind, Kind::Sub(_)));
    let across = |by| at.and_then(|at| here.across(at, column, by).map(|to| (at, to)));
    match (key, at) {
        ("down" | "up", _) => {
            let by = if key == "down" { 1 } else { -1 };
            if let Some(to) = here.step(at, by) {
                let column = here.first(to).expect("a stepped row runs");
                mark(state, menu, depth, (to, column), false, cx);
            }
        }
        ("left" | "right", Some(_)) if across(1).is_some() => {
            let to = across(if key == "right" { 1 } else { -1 }).expect("a row with a button");
            mark(state, menu, depth, to, false, cx);
        }
        ("right", Some(at)) if sub => choose(state, menu, depth, (at, column), close, window, cx),
        ("enter" | "space", Some(at)) => {
            choose(state, menu, depth, (at, column), close, window, cx)
        }
        ("left" | "escape", _) if depth > 0 => state.update(cx, |open, cx| {
            open.levels.pop();
            cx.notify();
        }),
        ("escape", _) => close(window, cx),
        _ => return false,
    }
    true
}

/// Whether the deepest level marks a button row.
fn on_buttons(menu: &Menu, state: &Entity<Open>, cx: &App) -> bool {
    let levels = &state.read(cx).levels;
    let Some(depth) = levels.len().checked_sub(1) else {
        return false;
    };
    let here = at_depth(menu, levels, depth);
    here.zip(levels[depth].at)
        .is_some_and(|(here, ix)| here.is_buttons(ix))
}

/// Measures the host's box: presses on it are the host's, and when `hangs_under` the menu hangs under it.
pub(crate) fn measure_host(state: Entity<Open>, hangs_under: bool) -> impl IntoElement {
    measure(state, move |open, bounds| {
        let changed = open.host != bounds || (hangs_under && open.anchor != bounds);
        open.host = bounds;
        if hangs_under {
            open.anchor = bounds;
        }
        changed
    })
}

/// The open menu of host `id`, placed at its spot. It holds focus while open and gives it back on every close.
pub(crate) fn hang(
    id: &ElementId,
    menu: &Menu,
    state: &Entity<Open>,
    field: Option<&Entity<TextInput>>,
    window: &mut Window,
    cx: &mut App,
) -> Option<AnyElement> {
    let spot = state.read(cx).spot?;
    let takeover = take_focus((id.clone(), "takeover"), window, cx);
    let focus = takeover.read(cx).focus.clone();
    if state.read(cx).takeover.is_none() {
        state.update(cx, |open, _| open.takeover = Some(takeover));
    }
    let close: Run = {
        let (state, id) = (state.clone(), id.clone());
        Rc::new(move |window, cx| {
            log::info!("menu {id:?}: closed");
            Open::close(&state, window, cx);
        })
    };
    if !focus.contains_focused(window, cx) {
        log::info!("menu {id:?}: focus left");
        close(window, cx);
        return None;
    }
    let typing = field.map(|field| field.read(cx).focus().clone());
    if let Some(typing) = &typing
        && focus.is_focused(window)
    {
        window.focus(typing, cx);
    }
    let mut levels = state.read(cx).levels.clone();
    settle(menu, &mut levels);
    if levels != state.read(cx).levels {
        log::info!("menu {id:?}: rows changed while open");
        state.update(cx, |open, _| open.levels = levels);
    }
    let depth = state.read(cx).levels.len();
    let enter =
        Animation::new(motion::duration(motion::FAST, cx)).with_easing(motion::ease_out_cubic);
    let (keyed, key_menu, key_close) = (state.clone(), menu.clone(), close.clone());
    let (lifted, lift_menu, lift_close) = (state.clone(), menu.clone(), close.clone());
    let (out, out_close) = (state.clone(), close.clone());
    let spaced = typing.is_none();
    let confirm = move |key: &str| key == "enter" || (spaced && key == "space");
    let head = field.map(|field| {
        let theme = cx.theme();
        div()
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .pb_1()
            .mb_1()
            .border_b_1()
            .border_color(theme.colors.border)
            .child(
                Icon::new(IconName::Search)
                    .size(IconSize::Sm)
                    .color(theme.colors.fg_subtle),
            )
            .child(div().flex_1().min_w_0().child(field.clone()))
            .into_any_element()
    });
    let (up_menu, up_state, up_close) = (menu.clone(), state.clone(), close.clone());
    let (down_menu, down_state, down_close) = (menu.clone(), state.clone(), close.clone());
    let (left_menu, left_state, left_close) = (menu.clone(), state.clone(), close.clone());
    let (right_menu, right_state, right_close) = (menu.clone(), state.clone(), close.clone());
    let root = draw::panel(id, menu, state, 0, &close, head, cx)
        .track_focus(&focus)
        .when(!spaced, |root| {
            root.capture_action(move |_: &Up, window, cx| {
                cx.stop_propagation();
                keys(&up_menu, &up_state, "up", &up_close, window, cx);
            })
            .capture_action(move |_: &Down, window, cx| {
                cx.stop_propagation();
                keys(&down_menu, &down_state, "down", &down_close, window, cx);
            })
            .capture_action(|_: &Enter, _, cx| cx.stop_propagation())
            // A button row takes the arrows from the filter.
            .capture_action(move |_: &Left, window, cx| {
                if on_buttons(&left_menu, &left_state, cx) {
                    cx.stop_propagation();
                    keys(&left_menu, &left_state, "left", &left_close, window, cx);
                }
            })
            .capture_action(move |_: &Right, window, cx| {
                if on_buttons(&right_menu, &right_state, cx) {
                    cx.stop_propagation();
                    keys(&right_menu, &right_state, "right", &right_close, window, cx);
                }
            })
        })
        .on_key_down(move |event, window, cx| {
            let stroke = &event.keystroke;
            let key = stroke.key.as_str();
            if matches!(key, "enter" | "space") {
                if confirm(key) && !stroke.modifiers.modified() {
                    cx.stop_propagation();
                }
                return;
            }
            if keys(&key_menu, &keyed, key, &key_close, window, cx) {
                cx.stop_propagation();
            }
        })
        .on_key_up(move |event, window, cx| {
            let stroke = &event.keystroke;
            let key = stroke.key.as_str();
            if confirm(key)
                && !stroke.modifiers.modified()
                && keys(&lift_menu, &lifted, key, &lift_close, window, cx)
            {
                cx.stop_propagation();
            }
        })
        .on_mouse_down_out(move |event, window, cx| {
            let open = out.read(cx);
            let on_host = open.host.contains(&event.position);
            if !on_host
                && !open
                    .levels
                    .iter()
                    .any(|level| level.panel.contains(&event.position))
            {
                out_close(window, cx);
            }
        });
    let placed = match spot {
        Spot::Under => float(
            (id.clone(), "panel"),
            state.read(cx).anchor,
            menu.entries.len(),
            root,
            window,
            cx,
        ),
        Spot::At(point) => raise(
            (id.clone(), "panel"),
            anchored()
                .position(point)
                .snap_to_window()
                .child(
                    root.with_animation((id.clone(), "in"), enter.clone(), |root, t| {
                        root.opacity(t)
                    }),
                ),
        )
        .with_priority(1)
        .into_any_element(),
    };
    let room = window.viewport_size().width;
    let subs = (1..depth).filter_map(|level| {
        let open = state.read(cx);
        let row = open.levels[level - 1].row;
        let width = match open.levels[level].panel.size.width {
            width if width > Pixels::ZERO => width,
            _ => cx.theme().menu_width().to_pixels(window.rem_size()),
        };
        // No room on the right: open on the left.
        let left = row.right() + width > room && row.left() >= width;
        let (corner, from, slide) = match left {
            true => (Anchor::TopRight, row.origin, -1.0),
            false => (Anchor::TopLeft, row.top_right(), 1.0),
        };
        (row != Bounds::default()).then(|| {
            raise(
                (id.clone(), format!("level-{level}")),
                anchored()
                    .anchor(corner)
                    .position(from)
                    .snap_to_window()
                    .child(
                        draw::panel(id, menu, state, level, &close, None, cx)
                            .mt_neg_1()
                            .with_animation(
                                (id.clone(), format!("sub-{level}")),
                                enter.clone(),
                                move |panel, t| {
                                    panel.opacity(t).left(motion::NUDGE * (t - 1.0) * slide)
                                },
                            ),
                    ),
            )
            .with_priority(2)
            .into_any_element()
        })
    });
    Some(
        div()
            .child(placed)
            .children(subs.collect::<Vec<_>>())
            .into_any_element(),
    )
}
