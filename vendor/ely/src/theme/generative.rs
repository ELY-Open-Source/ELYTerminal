use gpui::Rems;

use super::{Theme, tokens::px_to_rems};

/// Generative tools' measures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GenerativeSizes {
    /// A style preset's picture, square.
    pub preset: Rems,
    /// A second of video along a timeline.
    pub second: Rems,
    /// A shot's height on a timeline.
    pub shot: Rems,
    /// A loss chart's height on a job's card.
    pub loss: Rems,
}

impl Theme {
    pub fn generative(&self) -> GenerativeSizes {
        GenerativeSizes {
            preset: px_to_rems(88.0),
            second: px_to_rems(24.0),
            shot: px_to_rems(56.0),
            loss: px_to_rems(140.0),
        }
    }
}
