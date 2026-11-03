//! Hover intent: pause before opening, hold while aiming.

use std::time::Duration;

use gpui::{App, Bounds, Entity, Pixels, Point};

use super::{
    menu::{Open, mark},
    model::Menu,
};

/// A submenu row opens after the pointer rests this long.
pub(super) const OPEN: Duration = Duration::from_millis(150);
/// A row crossed toward an open submenu waits this long.
pub(super) const HOLD: Duration = Duration::from_millis(300);

/// Pointer over a row; `sub` when it opens a submenu.
pub(super) fn hover(
    state: &Entity<Open>,
    menu: &Menu,
    (depth, cell): (usize, (usize, usize)),
    sub: bool,
    at: Point<Pixels>,
    cx: &mut App,
) {
    let seen = state.update(cx, |open, _| {
        let from = open.pointer.replace(at);
        open.over = Some((depth, cell));
        let level = open.levels.get(depth)?;
        let shown = (open.levels.get(depth + 1)).map(|level| level.panel);
        Some((from, (level.at, level.column), shown))
    });
    let Some((from, marked, shown)) = seen else {
        return;
    };
    if marked == (Some(cell.0), cell.1) {
        if !sub || shown.is_some() {
            state.update(cx, |open, _| open.pending = None);
        } else if state.read(cx).pending.is_none() {
            later(state, menu, (depth, cell), OPEN, cx);
        }
        return;
    }
    let aimed = from
        .zip(shown.filter(|panel| *panel != Bounds::default()))
        .is_some_and(|(from, panel)| toward(from, at, panel));
    if aimed {
        log::debug!("menu: row {} of level {depth} waits on the aim", cell.0);
        return later(state, menu, (depth, cell), HOLD, cx);
    }
    state.update(cx, |open, _| open.pending = None);
    mark(state, menu, depth, cell, false, cx);
    if sub {
        later(state, menu, (depth, cell), OPEN, cx);
    }
}

/// The pointer left a row: its delayed mark is dropped.
pub(super) fn leave(state: &Entity<Open>, over: (usize, (usize, usize)), cx: &mut App) {
    state.update(cx, |open, _| {
        if open.over == Some(over) {
            (open.over, open.pending) = (None, None);
        }
    });
}

/// Marks the row after `wait` if still under the pointer.
fn later(
    state: &Entity<Open>,
    menu: &Menu,
    (depth, cell): (usize, (usize, usize)),
    wait: Duration,
    cx: &mut App,
) {
    let (state_, menu) = (state.clone(), menu.clone());
    let task = cx.spawn(async move |cx| {
        cx.background_executor().timer(wait).await;
        cx.update(|cx| {
            if state_.read(cx).over != Some((depth, cell)) {
                return log::debug!("menu: the pointer left row {} before it opened", cell.0);
            }
            state_.update(cx, |open, _| open.pending = None);
            mark(&state_, &menu, depth, cell, true, cx);
        });
    });
    state.update(cx, |open, _| open.pending = Some(task));
}

/// Whether `to` lies between `from` and `panel`'s near edge.
pub(super) fn toward(from: Point<Pixels>, to: Point<Pixels>, panel: Bounds<Pixels>) -> bool {
    let near = if panel.left() >= from.x {
        panel.left()
    } else {
        panel.right()
    };
    let (top, bottom) = (
        Point::new(near, panel.top()),
        Point::new(near, panel.bottom()),
    );
    let side = |a: Point<Pixels>, b: Point<Pixels>| {
        f32::from(b.x - a.x) * f32::from(to.y - a.y) - f32::from(b.y - a.y) * f32::from(to.x - a.x)
    };
    let sides = [side(from, top), side(top, bottom), side(bottom, from)];
    sides.iter().all(|side| *side >= 0.0) || sides.iter().all(|side| *side <= 0.0)
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, point, px, size};

    use super::toward;

    #[test]
    fn the_aim_holds_toward_the_panel_on_either_side() {
        let right = Bounds::new(point(px(200.), px(0.)), size(px(100.), px(200.)));
        let from = point(px(100.), px(100.));
        assert!(toward(from, point(px(110.), px(98.)), right));
        assert!(
            !toward(from, point(px(100.), px(130.)), right),
            "straight down"
        );
        let left = Bounds::new(point(px(0.), px(0.)), size(px(50.), px(200.)));
        assert!(toward(from, point(px(90.), px(102.)), left));
        assert!(
            !toward(from, point(px(110.), px(100.)), left),
            "away from it"
        );
    }
}
