use gpui::{
    App, Axis, Bounds, Div, DragMoveEvent, ElementId, Entity, EntityId, FocusHandle, FontWeight,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, Rems, SharedString,
    Stateful, StatefulInteractiveElement, Styled, Window, canvas, div, prelude::*,
    transparent_black,
};

use super::{Column, table::View};
use crate::layout::{bring_into_view, seeded::Seeded};
use crate::{
    primitives::{DragGhost, FocusRing, Icon, IconName},
    theme::{ActiveTheme, IconSize, TextSize},
};

/// A header's right edge being dragged, to resize its column.
pub(crate) struct Resize {
    owner: EntityId,
    key: SharedString,
}

/// A header being dragged, to move its column.
pub(crate) struct Move {
    owner: EntityId,
    key: SharedString,
}

/// The sort after a press on column `key`: alone it replaces the sort, with Shift it joins it; each press turns it rising, falling, then off.
pub(crate) fn next_sort(
    sort: &[(SharedString, bool)],
    key: &SharedString,
    join: bool,
) -> Vec<(SharedString, bool)> {
    let now = sort
        .iter()
        .find(|(known, _)| known == key)
        .map(|(_, rising)| *rising);
    let turned = match now {
        None => Some(true),
        Some(true) => Some(false),
        Some(false) => None,
    };
    let mut next: Vec<(SharedString, bool)> = if join {
        sort.iter()
            .filter(|(known, _)| known != key)
            .cloned()
            .collect()
    } else {
        Vec::new()
    };
    let at = if join {
        sort.iter()
            .position(|(known, _)| known == key)
            .unwrap_or(next.len())
    } else {
        0
    };
    if let Some(rising) = turned {
        next.insert(at.min(next.len()), (key.clone(), rising));
    }
    next
}

/// The column order after moving `key` in front of `target`.
pub(crate) fn moved(
    order: &[SharedString],
    key: &SharedString,
    target: &SharedString,
) -> Vec<SharedString> {
    let mut next: Vec<SharedString> = order
        .iter()
        .filter(|known| *known != key)
        .cloned()
        .collect();
    let at = next
        .iter()
        .position(|known| known == target)
        .unwrap_or(next.len());
    next.insert(at, key.clone());
    next
}

/// Brings a focused header whole into its table's sideways box once, from its painted bounds; forgets it when focus leaves.
fn reveal(
    view: &Entity<View>,
    key: &SharedString,
    focused: bool,
    head: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    if focused == (view.read(cx).revealed.as_ref() == Some(key)) {
        return;
    }
    if !focused {
        view.update(cx, |view, _| view.revealed = None);
        return;
    }
    let scroll = view.read(cx).sideways.clone();
    view.update(cx, |view, _| view.revealed = Some(key.clone()));
    if bring_into_view(&scroll, head, Axis::Horizontal) {
        log::info!("data table: header {key} scrolled into view");
        window.request_animation_frame();
    }
}

/// One header cell, sized by its caller: its title and sort mark; a press sorts, a drag moves it, and its right edge resizes. A sortable one is a Tab stop through `focus`, and scrolls into view when focused.
pub(crate) fn header(
    id: &ElementId,
    view: &Entity<View>,
    sorting: &Entity<Seeded<Vec<(SharedString, bool)>>>,
    column: &Column,
    focus: Option<FocusHandle>,
    cx: &App,
) -> Stateful<Div> {
    let sort = sorting.read(cx).value.clone();
    let theme = cx.theme();
    let colors = &theme.colors;
    let owner = view.entity_id();
    let place = sort.iter().position(|(known, _)| *known == column.key);
    let mark = place.map(|place| (sort[place].1, (sort.len() > 1).then_some(place + 1)));
    let key = column.key.clone();
    let (sorter, pager, mover, sizer, edges) = (
        sorting.clone(),
        view.clone(),
        view.clone(),
        view.clone(),
        view.clone(),
    );
    let title = column.title.clone();
    let cell = div()
        .id((id.clone(), format!("head-{key}")))
        .relative()
        .h_full()
        .flex()
        .items_center()
        .gap_1()
        .px_3()
        .border_1()
        .border_color(transparent_black())
        .text_size(theme.text_size(TextSize::Xs))
        .font_weight(FontWeight::MEDIUM)
        .text_color(colors.fg_muted);
    let revealing = focus.clone().filter(|_| !column.pinned);
    cell.when_some(focus, |cell, focus| cell.track_focus(&focus).focus_ring(cx))
        .when(column.sortable, |cell| {
            let key = key.clone();
            cell.cursor_pointer()
                .hover(|style| style.text_color(colors.fg))
                .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                .on_click(move |event, _, cx| {
                    let join = event.modifiers().shift;
                    sorter.update(cx, |sorting, cx| {
                        sorting.value = next_sort(&sorting.value, &key, join);
                        log::info!("data table: sort {:?}", sorting.value);
                        cx.notify();
                    });
                    pager.update(cx, |view, _| view.page = 0);
                })
        })
        .on_drag(
            Move {
                owner,
                key: key.clone(),
            },
            move |_, _, _, cx| DragGhost::new(title.clone(), None, cx),
        )
        .on_drop({
            let key = key.clone();
            move |drag: &Move, _, cx| {
                if drag.owner != owner || drag.key == key {
                    return;
                }
                mover.update(cx, |view, cx| {
                    view.order = moved(&view.order, &drag.key, &key);
                    log::info!("data table: moved {} before {key}", drag.key);
                    cx.notify();
                })
            }
        })
        .child(column.title.clone())
        .children(mark.map(|(rising, rank)| {
            div()
                .flex()
                .items_center()
                .child(
                    Icon::new(if rising {
                        IconName::ArrowUp
                    } else {
                        IconName::ArrowDown
                    })
                    .size(IconSize::Xs)
                    .color(colors.fg_muted),
                )
                .children(rank.map(|rank| {
                    div()
                        .text_size(theme.text_size(TextSize::Xs))
                        .child(rank.to_string())
                }))
        }))
        .child(grip(id, &key, owner, sizer, theme.handle_hit(), cx))
        .child(
            canvas(
                move |bounds, window, cx| {
                    if edges.read(cx).edges.get(&key) != Some(&bounds.left()) {
                        edges.update(cx, |view, _| view.edges.insert(key.clone(), bounds.left()));
                    }
                    if let Some(focus) = &revealing {
                        reveal(&edges, &key, focus.is_focused(window), bounds, window, cx);
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .inset_0(),
        )
}

/// The resize grip on a header's right edge.
fn grip(
    id: &ElementId,
    key: &SharedString,
    owner: EntityId,
    view: Entity<View>,
    hit: Rems,
    cx: &App,
) -> impl IntoElement + use<> {
    let border = cx.theme().colors.border_strong;
    let key = key.clone();
    div()
        .id((id.clone(), format!("grip-{key}")))
        .absolute()
        .top_1()
        .bottom_1()
        .right_0()
        .w(hit)
        .flex()
        .justify_center()
        .cursor_col_resize()
        .hover(|style| style.bg(border.opacity(0.2)))
        .child(
            div()
                .h_full()
                .border_r_1()
                .border_color(border.opacity(0.5)),
        )
        .on_drag(
            Resize {
                owner,
                key: key.clone(),
            },
            |_, _, _, cx| cx.new(|_| gpui::EmptyView),
        )
        .on_drag_move(move |event: &DragMoveEvent<Resize>, _, cx| {
            let drag = event.drag(cx);
            if drag.owner != owner || drag.key != key {
                return;
            }
            let Some(left) = view.read(cx).edges.get(&key).copied() else {
                return;
            };
            let wide = (event.event.position.x - left).max(view.read(cx).narrowest);
            view.update(cx, |view, cx| {
                view.widths.insert(key.clone(), wide);
                cx.notify();
            })
        })
}

#[cfg(test)]
mod tests {
    use gpui::SharedString;

    use super::{moved, next_sort};

    fn keys(list: &[&'static str]) -> Vec<SharedString> {
        list.iter().map(|key| SharedString::from(*key)).collect()
    }

    #[test]
    fn a_press_replaces_the_sort_and_shift_joins_it() {
        let a = SharedString::from("a");
        let b = SharedString::from("b");
        assert_eq!(next_sort(&[], &a, false), [(a.clone(), true)]);
        assert_eq!(
            next_sort(&[(a.clone(), true)], &b, true),
            [(a.clone(), true), (b.clone(), true)]
        );
        assert_eq!(
            next_sort(&[(a.clone(), true), (b.clone(), true)], &a, true),
            [(a.clone(), false), (b.clone(), true)]
        );
        assert_eq!(
            next_sort(&[(a.clone(), false), (b.clone(), true)], &a, true),
            [(b.clone(), true)]
        );
        assert_eq!(
            next_sort(&[(a.clone(), true), (b.clone(), true)], &b, false),
            [(b, false)]
        );
    }

    #[test]
    fn a_column_moves_in_front_of_its_target() {
        assert_eq!(
            moved(&keys(&["a", "b", "c"]), &"c".into(), &"a".into()),
            keys(&["c", "a", "b"])
        );
        assert_eq!(
            moved(&keys(&["a", "b", "c"]), &"a".into(), &"c".into()),
            keys(&["b", "a", "c"])
        );
    }
}
