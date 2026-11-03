use gpui::Rems;

use super::{Theme, tokens::px_to_rems};

/// Dashboard measures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DashboardSizes {
    /// A grid row, tall.
    pub row: Rems,
    /// Between tiles, either way.
    pub gap: Rems,
    /// A status dot, square.
    pub dot: Rems,
    /// Under this width the grid stacks its tiles.
    pub stack_below: Rems,
}

impl Theme {
    pub fn dashboard(&self) -> DashboardSizes {
        DashboardSizes {
            row: px_to_rems(88.0),
            gap: px_to_rems(16.0),
            dot: px_to_rems(8.0),
            stack_below: px_to_rems(560.0),
        }
    }
}
