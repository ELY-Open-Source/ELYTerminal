use gpui::Rems;

use super::{Theme, tokens::px_to_rems};

/// The tooling chapter's measures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToolingSizes {
    /// An FPS meter's graph, across and tall.
    pub graph: (Rems, Rems),
    /// How tall an event log grows before it scrolls.
    pub log: Rems,
    /// A token viewer's tile, and its samples' least width.
    pub tile: Rems,
    /// A token viewer's swatch.
    pub swatch: Rems,
}

impl Theme {
    pub fn tooling(&self) -> ToolingSizes {
        ToolingSizes {
            graph: (px_to_rems(160.0), px_to_rems(40.0)),
            log: px_to_rems(200.0),
            tile: px_to_rems(200.0),
            swatch: px_to_rems(28.0),
        }
    }
}
