use gpui::{Pixels, Rems, px};

use super::{Theme, tokens::px_to_rems};

/// Canvas measures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasSizes {
    /// The top ruler, tall.
    pub ruler: Rems,
    /// The left ruler, wide enough for four figures.
    pub side: Rems,
    /// Grid dots at least this far apart on screen.
    pub grid: Rems,
    /// Ruler labels at least this far apart on screen.
    pub label: Rems,
    /// A grid dot, square.
    pub dot: Pixels,
    /// Lines, wide.
    pub stroke: Rems,
    /// Frames and outlines, wide.
    pub hairline: Pixels,
    /// A note's inset and corner at a zoom of one.
    pub note_inset: Rems,
    pub note_corner: Rems,
    /// A minimap, wide and tall.
    pub map: (Rems, Rems),
    /// A handle, square.
    pub handle: Rems,
    /// An asset's tile, square, and the room around its drawing.
    pub asset: Rems,
    pub asset_inset: Rems,
    /// The constraints picture, square.
    pub pins: Rems,
    /// A place in the auto layout grid, square; the chosen one's mark and the rest's pips.
    pub place: Rems,
    pub place_mark: Rems,
    pub place_pip: Rems,
}

impl Theme {
    pub fn canvas(&self) -> CanvasSizes {
        CanvasSizes {
            ruler: px_to_rems(20.0),
            side: px_to_rems(32.0),
            grid: px_to_rems(16.0),
            label: px_to_rems(64.0),
            dot: px(1.5),
            stroke: px_to_rems(1.5),
            hairline: px(1.0),
            note_inset: px_to_rems(12.0),
            note_corner: px_to_rems(4.0),
            map: (px_to_rems(200.0), px_to_rems(140.0)),
            handle: px_to_rems(8.0),
            asset: px_to_rems(64.0),
            asset_inset: px_to_rems(10.0),
            pins: px_to_rems(72.0),
            place: px_to_rems(20.0),
            place_mark: px_to_rems(8.0),
            place_pip: px_to_rems(3.0),
        }
    }
}
