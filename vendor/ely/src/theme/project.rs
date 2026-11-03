use gpui::Rems;

use super::{Theme, tokens::px_to_rems};

/// Project's measures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectSizes {
    /// A board's column, wide.
    pub column: Rems,
    /// A roadmap's month at least, wide.
    pub month: Rems,
    /// A pomodoro's ring, wide.
    pub pomodoro: Rems,
}

impl Theme {
    pub fn project(&self) -> ProjectSizes {
        ProjectSizes {
            column: px_to_rems(272.0),
            month: px_to_rems(72.0),
            pomodoro: px_to_rems(176.0),
        }
    }
}
