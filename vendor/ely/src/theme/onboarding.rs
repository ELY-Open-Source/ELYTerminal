use gpui::Rems;

use super::{Theme, tokens::px_to_rems};

/// Onboarding's measures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OnboardingSizes {
    /// A hotspot's dot, wide; its rings reach twice that.
    pub hotspot: Rems,
}

impl Theme {
    pub fn onboarding(&self) -> OnboardingSizes {
        OnboardingSizes {
            hotspot: px_to_rems(10.0),
        }
    }
}
