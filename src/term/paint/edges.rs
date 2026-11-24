//! Each row's grounds, the edge cells' carried past the last whole row and column, so a program's ground reaches the box's edges; a row that keeps its ground stops at its cells below.

use gpui::{Bounds, Pixels, Point, Size, Window, fill, point};

use super::{Look, grounds};
use crate::term::frame::Frame;

/// Every row's grounds from `origin`; the room `bounds` holds past whole cells takes its edge cells' grounds.
pub(super) fn paint(
    frame: &Frame,
    bounds: Bounds<Pixels>,
    origin: Point<Pixels>,
    look: &Look,
    window: &mut Window,
) {
    let rows = frame.rows.len();
    for (index, row) in frame.rows.iter().enumerate() {
        for (column, count, bg) in grounds(row) {
            let span = (index, column, count);
            let at = quad(bounds, origin, look.cell, (rows, row.keeps), span);
            window.paint_quad(fill(at, bg));
        }
    }
}

/// Where row `index`'s ground over `count` columns from `column` paints, of `rows`: past the box's whole cells to its edge when it reaches them, and below only while the row does not keep it.
fn quad(
    bounds: Bounds<Pixels>,
    origin: Point<Pixels>,
    cell: Size<Pixels>,
    (rows, keeps): (usize, bool),
    (index, column, count): (usize, usize, usize),
) -> Bounds<Pixels> {
    let columns = (bounds.size.width / cell.width).floor() as usize;
    let lines = (bounds.size.height / cell.height).floor() as usize;
    let x = |column: usize| origin.x + cell.width * column as f32;
    let top = origin.y + cell.height * index as f32;
    let below = match index + 1 == lines && rows == lines && !keeps {
        true => bounds.bottom().max(top + cell.height),
        false => top + cell.height,
    };
    let end = match column + count == columns {
        true => bounds.right().max(x(columns)),
        false => x(column + count),
    };
    Bounds::from_corners(point(x(column), top), point(end, below))
}

#[cfg(test)]
mod tests {
    use gpui::{px, size};

    use super::*;

    #[test]
    fn only_a_ground_at_the_last_whole_row_or_column_reaches_the_box_s_edge() {
        let (cell, origin) = (size(px(8.0), px(16.0)), point(px(0.0), px(0.0)));
        // 10 columns and 3 rows, with 5 points to the right and 9 below.
        let bounds = Bounds::new(origin, size(px(85.0), px(57.0)));
        let at = |span| quad(bounds, origin, cell, (3, false), span);
        let corners = |left: f32, top: f32, right: f32, bottom: f32| {
            Bounds::from_corners(point(px(left), px(top)), point(px(right), px(bottom)))
        };
        assert_eq!(at((0, 0, 4)), corners(0.0, 0.0, 32.0, 16.0), "inside");
        assert_eq!(
            at((1, 6, 4)),
            corners(48.0, 16.0, 85.0, 32.0),
            "to the right"
        );
        assert_eq!(at((2, 0, 4)), corners(0.0, 32.0, 32.0, 57.0), "below");
        assert_eq!(at((2, 0, 10)), corners(0.0, 32.0, 85.0, 57.0), "the corner");
        let early = quad(bounds, origin, cell, (2, false), (1, 0, 4));
        assert_eq!(
            early,
            corners(0.0, 16.0, 32.0, 32.0),
            "rows that do not fill it yet"
        );
        let kept = quad(bounds, origin, cell, (3, true), (2, 0, 10));
        assert_eq!(
            kept,
            corners(0.0, 32.0, 85.0, 48.0),
            "a kept row reaches the right alone"
        );
    }
}
