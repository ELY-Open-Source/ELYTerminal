use gpui::Rems;

use super::{Theme, tokens::px_to_rems};

/// Messaging's measures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MessagingSizes {
    /// A call's strip of tiles beside a shared screen, wide.
    pub tiles: Rems,
    /// A call's tile at its narrowest, before a grid drops a column.
    pub tile: Rems,
    /// A sticker's cell in a picker, square.
    pub sticker: Rems,
}

impl Theme {
    pub fn messaging(&self) -> MessagingSizes {
        MessagingSizes {
            tiles: px_to_rems(200.0),
            tile: px_to_rems(160.0),
            sticker: px_to_rems(64.0),
        }
    }
}
