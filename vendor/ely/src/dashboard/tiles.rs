use gpui::SharedString;

/// A tile's place on a dashboard grid: its key, and its column, row, width and height in cells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tile {
    pub key: SharedString,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Tile {
    pub fn new(key: impl Into<SharedString>, (x, y): (u32, u32), (w, h): (u32, u32)) -> Self {
        assert!(w > 0 && h > 0, "a tile of {w} by {h} cells");
        Self {
            key: key.into(),
            x,
            y,
            w,
            h,
        }
    }

    fn overlaps(&self, other: &Tile) -> bool {
        self.x < other.x + other.w
            && other.x < self.x + self.w
            && self.y < other.y + other.h
            && other.y < self.y + self.h
    }

    /// This tile kept inside `columns`: at least a cell each way, no wider than the grid, and not past its right edge.
    fn fitted(mut self, columns: u32) -> Tile {
        self.w = self.w.clamp(1, columns);
        self.h = self.h.max(1);
        self.x = self.x.min(columns - self.w);
        self
    }
}

/// Tiles risen as far as the ones above allow, top first.
fn compacted(mut tiles: Vec<Tile>) -> Vec<Tile> {
    tiles.sort_by_key(|tile| (tile.y, tile.x));
    let mut out: Vec<Tile> = Vec::new();
    for mut tile in tiles {
        while tile.y > 0 {
            let higher = Tile {
                y: tile.y - 1,
                ..tile.clone()
            };
            if out.iter().any(|placed| placed.overlaps(&higher)) {
                break;
            }
            tile = higher;
        }
        out.push(tile);
    }
    out
}

/// The tiles once `to` takes the place of the tile of its key: it goes where it was put, the tiles it lands on move down below it, then every tile, it too, rises as far as the ones above allow. None, logged, when the grid lost that tile.
pub fn arranged(tiles: &[Tile], to: Tile, columns: u32) -> Option<Vec<Tile>> {
    assert!(columns > 0, "a grid of no columns");
    if !tiles.iter().any(|tile| tile.key == to.key) {
        log::error!(
            "dashboard grid: no tile {} on the grid; nothing moves",
            to.key
        );
        return None;
    }
    let mut placed = vec![to.clone().fitted(columns)];
    let mut rest: Vec<Tile> = tiles
        .iter()
        .filter(|tile| tile.key != to.key)
        .cloned()
        .collect();
    rest.sort_by_key(|tile| (tile.y, tile.x));
    for mut tile in rest {
        while let Some(hit) = placed.iter().find(|placed| placed.overlaps(&tile)) {
            tile.y = hit.y + hit.h;
        }
        placed.push(tile);
    }
    Some(compacted(placed))
}

/// Where a tile goes on a step down or up: below the next tile under it, or into the place of the next one above; none at the edge. Both overlap its columns.
pub fn stepped(tiles: &[Tile], key: &str, down: bool) -> Option<Tile> {
    let Some(tile) = tiles.iter().find(|tile| tile.key == key) else {
        log::error!("dashboard grid: no tile {key} on the grid");
        return None;
    };
    let beside = |other: &&Tile| {
        other.key != tile.key && other.x < tile.x + tile.w && tile.x < other.x + other.w
    };
    if down {
        tiles
            .iter()
            .filter(beside)
            .filter(|other| other.y >= tile.y + tile.h)
            .min_by_key(|other| other.y)
            .map(|next| Tile {
                y: next.y + next.h,
                ..tile.clone()
            })
    } else {
        tiles
            .iter()
            .filter(beside)
            .filter(|other| other.y + other.h <= tile.y)
            .max_by_key(|other| other.y)
            .map(|above| Tile {
                y: above.y,
                ..tile.clone()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{Tile, arranged, stepped};

    fn at(tiles: &[Tile], key: &str) -> (u32, u32, u32, u32) {
        let tile = tiles.iter().find(|tile| tile.key == key).expect("a tile");
        (tile.x, tile.y, tile.w, tile.h)
    }

    fn grid() -> Vec<Tile> {
        vec![
            Tile::new("a", (0, 0), (6, 2)),
            Tile::new("b", (6, 0), (6, 2)),
            Tile::new("c", (0, 2), (12, 3)),
        ]
    }

    #[test]
    fn a_tile_gone_from_the_grid_moves_nothing() {
        let gone = Tile::new("gone", (0, 0), (6, 2));
        assert_eq!(arranged(&grid(), gone, 12), None);
        assert_eq!(stepped(&grid(), "gone", true), None);
    }

    #[test]
    fn a_tile_dropped_on_another_pushes_it_below() {
        let moved =
            arranged(&grid(), Tile::new("b", (0, 0), (6, 2)), 12).expect("a tile of the grid");
        assert_eq!(at(&moved, "b"), (0, 0, 6, 2), "the dropped tile stays");
        assert_eq!(at(&moved, "a"), (0, 2, 6, 2), "a goes below b");
        assert_eq!(at(&moved, "c"), (0, 4, 12, 3), "and c below a");
    }

    #[test]
    fn tiles_rise_into_the_gap_a_tile_leaves() {
        let moved =
            arranged(&grid(), Tile::new("a", (0, 9), (6, 2)), 12).expect("a tile of the grid");
        assert_eq!(at(&moved, "c"), (0, 2, 12, 3), "c keeps below b");
        assert_eq!(at(&moved, "a"), (0, 5, 6, 2), "a rises to rest on c");
    }

    #[test]
    fn a_tile_stays_inside_the_grid() {
        let moved =
            arranged(&grid(), Tile::new("a", (10, 0), (6, 2)), 12).expect("a tile of the grid");
        assert_eq!(
            at(&moved, "a"),
            (6, 0, 6, 2),
            "pulled back from the right edge"
        );
        let wide =
            arranged(&grid(), Tile::new("b", (0, 0), (20, 1)), 12).expect("a tile of the grid");
        assert_eq!(at(&wide, "b"), (0, 0, 12, 1), "no wider than the grid");
    }

    #[test]
    fn a_step_down_passes_the_tile_under_and_a_step_up_takes_its_place() {
        let down = stepped(&grid(), "a", true).expect("c lies under a");
        assert_eq!((down.x, down.y), (0, 5), "a steps under the bottom of c");
        let moved = arranged(&grid(), down, 12).expect("a tile of the grid");
        assert_eq!(at(&moved, "c"), (0, 2, 12, 3), "c rests under b");
        assert_eq!(at(&moved, "a"), (0, 5, 6, 2), "a rests under c");
        let up = stepped(&moved, "a", false).expect("c lies over a");
        assert_eq!(
            at(&arranged(&moved, up, 12).expect("a tile of the grid"), "a"),
            (0, 0, 6, 2),
            "back on top"
        );
        assert_eq!(stepped(&grid(), "b", false), None, "nothing over b");
    }
}
