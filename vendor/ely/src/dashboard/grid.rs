use std::rc::Rc;

use gpui::{
    AnyElement, App, AppContext as _, ElementId, EmptyView, EntityId, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Pixels, Point, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, canvas, div,
};

use super::tiles::{Tile, arranged, stepped};
use crate::{
    primitives::{Icon, IconName, tab_stop},
    theme::{ActiveTheme, IconSize, Radius},
};

type OnTiles = Rc<dyn Fn(Vec<Tile>, &mut Window, &mut App)>;

/// What a drag on a tile does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Grip {
    Move,
    Size,
}

/// A drag on a grid's tile: whose grid, which tile, what it does, and where the pointer was pressed.
struct Pull {
    owner: EntityId,
}

/// The grid's own state: its width, and a drag under way with the tile as it would land.
#[derive(Default)]
struct Board {
    width: f32,
    pull: Option<(SharedString, Grip, Point<Pixels>, Tile)>,
    landing: Option<Tile>,
}

/// Widgets on a grid of columns. A tile's top strip drags it and its corner resizes it, a cell at a time; the tiles in the way move down and every tile rises to fill a gap, while the drag shows where it will land. A focused tile moves a column with Left and Right, below the next tile under it with Down or into the place of the one above with Up, and grows or shrinks a cell with Shift. The owner keeps the tiles. Under the theme's `stack_below` width the tiles stand in one column in reading order, each its own height, and moving waits for room.
#[derive(IntoElement)]
pub struct DashboardGrid {
    id: ElementId,
    columns: u32,
    tiles: Vec<Tile>,
    cards: Vec<(SharedString, AnyElement)>,
    on_change: OnTiles,
}

impl DashboardGrid {
    pub fn new(
        id: impl Into<ElementId>,
        columns: u32,
        tiles: impl IntoIterator<Item = Tile>,
        on_change: impl Fn(Vec<Tile>, &mut Window, &mut App) + 'static,
    ) -> Self {
        assert!(columns > 0, "a grid of no columns");
        Self {
            id: id.into(),
            columns,
            tiles: tiles.into_iter().collect(),
            cards: Vec::new(),
            on_change: Rc::new(on_change),
        }
    }

    /// What the tile of `key` shows.
    pub fn card(mut self, key: impl Into<SharedString>, card: impl IntoElement) -> Self {
        self.cards.push((key.into(), card.into_any_element()));
        self
    }
}

/// The tile a drag of `by` pixels makes of `start`, a cell at a time.
fn pulled(start: &Tile, grip: Grip, by: (f32, f32), cell: (f32, f32)) -> Tile {
    let step = |pixels: f32, size: f32| (pixels / size).round() as i64;
    let (dx, dy) = (step(by.0, cell.0), step(by.1, cell.1));
    let moved = |value: u32, by: i64, least: i64| (value as i64 + by).max(least) as u32;
    match grip {
        Grip::Move => Tile {
            x: moved(start.x, dx, 0),
            y: moved(start.y, dy, 0),
            ..start.clone()
        },
        Grip::Size => Tile {
            w: moved(start.w, dx, 1),
            h: moved(start.h, dy, 1),
            ..start.clone()
        },
    }
}

/// A canvas that keeps the grid's width in its board.
fn measure(board: gpui::Entity<Board>) -> impl IntoElement {
    canvas(
        move |bounds, window, cx| {
            let now = f32::from(bounds.size.width);
            if board.read(cx).width != now {
                board.update(cx, |board, _| board.width = now);
                window.request_animation_frame();
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// The tiles in one column in reading order, each as tall as its rows: a grid too narrow for its columns.
fn stacked(
    id: ElementId,
    mut tiles: Vec<Tile>,
    mut cards: Vec<(SharedString, AnyElement)>,
    board: gpui::Entity<Board>,
    cx: &App,
) -> AnyElement {
    let sizes = cx.theme().dashboard();
    tiles.sort_by_key(|tile| (tile.y, tile.x));
    let column: Vec<_> = tiles
        .into_iter()
        .filter_map(|tile| {
            let Some(at) = cards.iter().position(|(key, _)| *key == tile.key) else {
                log::error!(
                    "dashboard grid {id:?}: no card for tile {}; left out",
                    tile.key
                );
                return None;
            };
            let card = cards.remove(at).1;
            let key = tile.key.clone();
            Some(
                div()
                    .debug_selector(move || format!("stacked-{key}"))
                    .h((sizes.row + sizes.gap) * tile.h as f32 - sizes.gap)
                    .child(card),
            )
        })
        .collect();
    if !cards.is_empty() {
        let keys: Vec<_> = cards.iter().map(|(key, _)| key).collect();
        log::error!("dashboard grid: cards with no tile {keys:?}; left out");
    }
    div()
        .id(id)
        .relative()
        .flex()
        .flex_col()
        .gap(sizes.gap)
        .child(measure(board))
        .children(column)
        .into_any_element()
}

impl RenderOnce for DashboardGrid {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let board = window.use_keyed_state((id.clone(), "board"), cx, |_, _| Board::default());
        let owner = board.entity_id();
        let theme = cx.theme();
        let rem = window.rem_size();
        let (row, gap) = (
            f32::from(theme.dashboard().row.to_pixels(rem)),
            f32::from(theme.dashboard().gap.to_pixels(rem)),
        );
        let columns = self.columns;
        let width = board.read(cx).width;
        if width > 0.0 && width < f32::from(theme.dashboard().stack_below.to_pixels(rem)) {
            return stacked(id, self.tiles, self.cards, board, cx);
        }
        let cell = ((width + gap) / columns as f32, row + gap);
        let landing = board.read(cx).landing.clone();
        let shown = match &landing {
            Some(tile) => {
                arranged(&self.tiles, tile.clone(), columns).unwrap_or_else(|| self.tiles.clone())
            }
            None => self.tiles.clone(),
        };
        let height = shown.iter().map(|tile| tile.y + tile.h).max().unwrap_or(0) as f32 * cell.1;
        let tiles = Rc::new(self.tiles);
        let on_change = self.on_change;
        let (colors, radius) = (theme.colors.clone(), theme.radius(Radius::Lg));
        let grip = theme.control_height(crate::theme::ControlSize::Sm);
        let mut cards = self.cards;
        let placed: Vec<AnyElement> = shown
            .iter()
            .filter_map(|tile| {
                let Some(at) = cards.iter().position(|(key, _)| *key == tile.key) else {
                    log::error!(
                        "dashboard grid {id:?}: no card for tile {}; left out",
                        tile.key
                    );
                    return None;
                };
                let card = cards.remove(at).1;
                let focus = tab_stop(
                    (id.clone(), format!("tile-{}", tile.key)).into(),
                    true,
                    window,
                    cx,
                );
                let focused = focus.is_focused(window);
                let key = tile.key.clone();
                let start = |grip: Grip| {
                    let (board, key, tiles) = (board.clone(), key.clone(), tiles.clone());
                    move |event: &gpui::MouseDownEvent, _: &mut Window, cx: &mut App| {
                        cx.stop_propagation();
                        let tile = tiles
                            .iter()
                            .find(|tile| tile.key == key)
                            .expect("a tile of the grid")
                            .clone();
                        board.update(cx, |board, _| {
                            board.pull = Some((key.clone(), grip, event.position, tile))
                        });
                    }
                };
                let (keys, keyed_tiles, keyed_change, own) =
                    (key.clone(), tiles.clone(), on_change.clone(), focus.clone());
                Some(
                    div()
                        .id((id.clone(), format!("tile-{}", tile.key)))
                        .track_focus(&focus)
                        .absolute()
                        .left(Pixels::from(tile.x as f32 * cell.0))
                        .top(Pixels::from(tile.y as f32 * cell.1))
                        .w(Pixels::from(tile.w as f32 * cell.0 - gap))
                        .h(Pixels::from(tile.h as f32 * cell.1 - gap))
                        .rounded(radius)
                        .border_1()
                        .border_color(if focused {
                            colors.focus
                        } else {
                            gpui::transparent_black()
                        })
                        .on_key_down(move |event, window, cx| {
                            if !own.is_focused(window) {
                                return;
                            }
                            let grow = event.keystroke.modifiers.shift;
                            let key = event.keystroke.key.as_str();
                            let (dx, dy): (i64, i64) = match key {
                                "left" => (-1, 0),
                                "right" => (1, 0),
                                "up" => (0, -1),
                                "down" => (0, 1),
                                _ => return,
                            };
                            cx.stop_propagation();
                            let tile = keyed_tiles
                                .iter()
                                .find(|tile| tile.key == keys)
                                .expect("a tile of the grid");
                            let to = match (grow, dy) {
                                (false, 0) => {
                                    pulled(tile, Grip::Move, (dx as f32, 0.0), (1.0, 1.0))
                                }
                                (false, _) => match stepped(&keyed_tiles, &keys, dy > 0) {
                                    Some(to) => to,
                                    None => return,
                                },
                                (true, _) => {
                                    pulled(tile, Grip::Size, (dx as f32, dy as f32), (1.0, 1.0))
                                }
                            };
                            log::info!("dashboard grid: {} to {to:?}", keys);
                            if let Some(tiles) = arranged(&keyed_tiles, to, columns) {
                                keyed_change(tiles, window, cx);
                            }
                        })
                        .child(div().size_full().child(card))
                        .child(
                            div()
                                .id((id.clone(), format!("move-{}", tile.key)))
                                .absolute()
                                .top_0()
                                .left_0()
                                .right(grip)
                                .h(grip)
                                .cursor_grab()
                                .on_mouse_down(MouseButton::Left, start(Grip::Move))
                                .on_drag(Pull { owner }, |_, _, _, cx| cx.new(|_| EmptyView)),
                        )
                        .child(
                            div()
                                .id((id.clone(), format!("size-{}", tile.key)))
                                .absolute()
                                .bottom_0()
                                .right_0()
                                .size(grip)
                                .flex()
                                .items_end()
                                .justify_end()
                                .cursor(gpui::CursorStyle::ResizeUpLeftDownRight)
                                .on_mouse_down(MouseButton::Left, start(Grip::Size))
                                .on_drag(Pull { owner }, |_, _, _, cx| cx.new(|_| EmptyView))
                                .child(
                                    Icon::new(IconName::GripHorizontal)
                                        .size(IconSize::Xs)
                                        .color(colors.fg_subtle),
                                ),
                        )
                        .into_any_element(),
                )
            })
            .collect();
        if !cards.is_empty() {
            let keys: Vec<_> = cards.iter().map(|(key, _)| key).collect();
            log::error!("dashboard grid: cards with no tile {keys:?}; left out");
        }
        let ghost = landing.as_ref().map(|tile| {
            div()
                .absolute()
                .left(Pixels::from(tile.x as f32 * cell.0))
                .top(Pixels::from(tile.y as f32 * cell.1))
                .w(Pixels::from(tile.w as f32 * cell.0 - gap))
                .h(Pixels::from(tile.h as f32 * cell.1 - gap))
                .rounded(radius)
                .bg(colors.accent.alpha(0.06))
                .border_1()
                .border_color(colors.accent.alpha(0.4))
        });
        let (moved, measured, dropped, released, left) = (
            board.clone(),
            board.clone(),
            board.clone(),
            board.clone(),
            board.clone(),
        );
        let end = move |board: &gpui::Entity<Board>,
                        tiles: &[Tile],
                        on_change: &OnTiles,
                        window: &mut Window,
                        cx: &mut App| {
            let landing = board.update(cx, |board, cx| {
                board.pull = None;
                cx.notify();
                board.landing.take()
            });
            if let Some(to) = landing {
                log::info!("dashboard grid: {} lands at {to:?}", to.key);
                if let Some(tiles) = arranged(tiles, to, columns) {
                    on_change(tiles, window, cx);
                }
            }
        };
        let end = Rc::new(end);
        let (drop_end, up_end, out_end) = (end.clone(), end.clone(), end);
        let (drop_tiles, up_tiles, out_tiles) = (tiles.clone(), tiles.clone(), tiles);
        let (drop_change, up_change, out_change) =
            (on_change.clone(), on_change.clone(), on_change);
        div()
            .id(id)
            .relative()
            .h(Pixels::from(height.max(cell.1)))
            .child(measure(measured))
            .on_drag_move::<Pull>(move |event, _, cx| {
                if event.drag(cx).owner != owner {
                    return;
                }
                let Some((_, grip, from, start)) = moved.read(cx).pull.clone() else {
                    return;
                };
                let by = event.event.position - from;
                let to = pulled(&start, grip, (f32::from(by.x), f32::from(by.y)), cell);
                if moved.read(cx).landing.as_ref() != Some(&to) {
                    moved.update(cx, |board, cx| {
                        board.landing = Some(to);
                        cx.notify();
                    });
                }
            })
            .on_drop(move |_: &Pull, window, cx| {
                drop_end(&dropped, &drop_tiles, &drop_change, window, cx)
            })
            .on_mouse_up(MouseButton::Left, move |_, window, cx| {
                up_end(&released, &up_tiles, &up_change, window, cx)
            })
            .on_mouse_up_out(MouseButton::Left, move |_, window, cx| {
                out_end(&left, &out_tiles, &out_change, window, cx)
            })
            .children(ghost)
            .children(placed)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{Grip, pulled};
    use crate::dashboard::Tile;

    #[test]
    fn a_pull_moves_or_grows_a_tile_by_whole_cells() {
        let tile = Tile::new("a", (2, 1), (3, 2));
        assert_eq!(
            pulled(&tile, Grip::Move, (130.0, -70.0), (100.0, 80.0)),
            Tile::new("a", (3, 0), (3, 2))
        );
        assert_eq!(
            pulled(&tile, Grip::Move, (-900.0, 0.0), (100.0, 80.0)),
            Tile::new("a", (0, 1), (3, 2)),
            "not past the left edge"
        );
        assert_eq!(
            pulled(&tile, Grip::Size, (-400.0, 90.0), (100.0, 80.0)),
            Tile::new("a", (2, 1), (1, 3)),
            "at least a cell"
        );
    }
}
