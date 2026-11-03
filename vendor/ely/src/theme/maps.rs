use gpui::{Pixels, Rems, px};

use super::{Theme, tokens::px_to_rems};

/// A map's measures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapSizes {
    /// A tile's side at its own zoom, as tile servers cut them.
    pub tile: Pixels,
    /// A popup's width, less where the map is narrower.
    pub popup: Rems,
    /// Room a popup keeps from the map's edges and from its point.
    pub margin: Rems,
    /// A route's stroke.
    pub route: Rems,
    /// A GeoJSON line's and outline's stroke.
    pub line: Rems,
    /// A GeoJSON point's side.
    pub dot: Rems,
    /// A heat cell's side.
    pub heat_cell: Rems,
    /// How far a heat point spreads.
    pub heat_reach: Rems,
    /// A cluster's side.
    pub cluster: Rems,
    /// The square of the world that gathers pins into one cluster.
    pub gather: Rems,
    /// A choropleth legend's scale bar.
    pub legend: Rems,
}

impl Theme {
    pub fn maps(&self) -> MapSizes {
        MapSizes {
            tile: px(256.0),
            popup: px_to_rems(240.0),
            margin: px_to_rems(8.0),
            route: px_to_rems(3.0),
            line: px_to_rems(1.5),
            dot: px_to_rems(8.0),
            heat_cell: px_to_rems(4.0),
            heat_reach: px_to_rems(28.0),
            cluster: px_to_rems(32.0),
            gather: px_to_rems(56.0),
            legend: px_to_rems(96.0),
        }
    }
}
