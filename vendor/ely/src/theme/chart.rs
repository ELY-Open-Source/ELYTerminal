use gpui::{Pixels, Rems, px};

use super::{Theme, tokens::px_to_rems};

/// A chart's measures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartSizes {
    /// A chart's height, legend and axes included.
    pub height: Rems,
    /// Room beside a plot for its value labels.
    pub gutter: Rems,
    /// Room under a plot for its category labels.
    pub foot: Rems,
    /// Room a plot keeps above and to its right, so dots and strokes stay whole.
    pub inset: Rems,
    /// The least room a category label takes along its axis; closer labels thin out.
    pub label: Rems,
    pub stroke: Rems,
    /// Widest a bubble's radius grows.
    pub bubble: Rems,
    /// A market chart's room for each candle before any zoom.
    pub candle: Rems,
    /// How far a slice under the pointer lifts out.
    pub lift: Rems,
    /// Gridlines and rules.
    pub hairline: Pixels,
}

impl Theme {
    pub fn chart(&self) -> ChartSizes {
        ChartSizes {
            height: px_to_rems(280.0),
            gutter: px_to_rems(48.0),
            foot: px_to_rems(28.0),
            inset: px_to_rems(8.0),
            label: px_to_rems(64.0),
            stroke: px_to_rems(2.0),
            bubble: px_to_rems(24.0),
            candle: px_to_rems(8.0),
            lift: px_to_rems(6.0),
            hairline: px(1.0),
        }
    }
}
