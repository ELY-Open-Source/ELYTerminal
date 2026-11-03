use gpui::Rems;

use super::{Theme, tokens::px_to_rems};

/// File views' measures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FileSizes {
    /// A tile in a grid of files, wide.
    pub tile: Rems,
    /// A column of a folder along a path, wide.
    pub column: Rems,
    /// The columns view, tall.
    pub columns: Rems,
}

impl Theme {
    pub fn files(&self) -> FileSizes {
        FileSizes {
            tile: px_to_rems(96.0),
            column: px_to_rems(200.0),
            columns: px_to_rems(288.0),
        }
    }
}
