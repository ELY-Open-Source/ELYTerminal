use gpui::Rems;

use super::{Theme, tokens::px_to_rems};

/// Interaction's measures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InteractionSizes {
    /// What an arrow key resizes a box by.
    pub step: Rems,
    /// A resize corner's grip, square.
    pub grip: Rems,
}

impl Theme {
    pub fn interaction(&self) -> InteractionSizes {
        InteractionSizes {
            step: px_to_rems(8.0),
            grip: px_to_rems(12.0),
        }
    }
}
