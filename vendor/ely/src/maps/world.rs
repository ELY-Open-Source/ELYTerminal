use std::sync::{Arc, LazyLock};

use gpui::{App, ElementId, IntoElement, RenderOnce, Window};
use isocountry::CountryCode;
use serde_json::Value;

use super::{ChoroplethLayer, ChoroplethMap, Feature, MapView, read_geojson};
use crate::Assets;

/// Natural Earth's 110m countries, read once from the bundle: each keeps its name and, but for two, its ISO 3166 alpha-2 code as `iso`.
pub(crate) static COUNTRIES: LazyLock<Arc<[Feature]>> = LazyLock::new(|| {
    let file = Assets::get("maps/countries.geojson").expect("the world's countries are bundled");
    let text = std::str::from_utf8(&file.data).expect("the countries are UTF-8");
    read_geojson(text)
        .expect("the bundled countries read")
        .into()
});

fn iso(country: &Feature) -> Option<&str> {
    country.properties.get("iso").and_then(Value::as_str)
}

/// The world's countries, washed each by its value keyed by ISO 3166 alpha-2 code, with a legend; Natural Earth is named in the corner unless the map names a source. Fails on a key that is neither an ISO code nor one of the bundle's; a code with no country at this scale draws nowhere and is logged once.
#[derive(IntoElement)]
pub struct WorldMap {
    id: ElementId,
    map: MapView,
    values: Vec<(String, f64)>,
    hue: usize,
}

impl WorldMap {
    pub fn new(
        id: impl Into<ElementId>,
        map: MapView,
        values: impl IntoIterator<Item = (impl Into<String>, f64)>,
    ) -> Self {
        Self {
            id: id.into(),
            map,
            values: values
                .into_iter()
                .map(|(code, value)| (code.into(), value))
                .collect(),
            hue: 0,
        }
    }

    /// The chart hue the scale runs to.
    pub fn hue(mut self, hue: usize) -> Self {
        self.hue = hue;
        self
    }
}

/// Whether a key names a country: an ISO 3166 alpha-2 code, or a code the bundle carries, as Kosovo's XK.
fn known(code: &str) -> bool {
    CountryCode::for_alpha2(code).is_ok()
        || COUNTRIES.iter().any(|country| iso(country) == Some(code))
}

/// The map with Natural Earth named in its corner, unless it names a source of its own.
fn sourced(map: MapView) -> MapView {
    match map.attribution.is_some() {
        true => map,
        false => map.attribution("Countries: Natural Earth"),
    }
}

/// The bundled countries keyed by their ISO codes.
fn countries(values: Vec<(String, f64)>, hue: usize) -> ChoroplethLayer {
    ChoroplethLayer::new(COUNTRIES.clone(), values)
        .key_by("iso")
        .hue(hue)
}

impl RenderOnce for WorldMap {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        for (code, _) in &self.values {
            assert!(
                known(code),
                "world map {:?}: {code} is no ISO 3166 alpha-2 code",
                self.id
            );
        }
        ChoroplethMap::new(self.id, sourced(self.map), countries(self.values, self.hue))
    }
}

#[cfg(test)]
mod tests {
    use super::{COUNTRIES, countries, iso, known, sourced};
    use crate::maps::{LatLon, MapView, MapViewport};

    #[test]
    fn the_bundle_holds_the_worlds_countries_by_code() {
        assert_eq!(COUNTRIES.len(), 177);
        let named = |code: &str| {
            COUNTRIES
                .iter()
                .find(|country| iso(country) == Some(code))
                .and_then(|country| country.properties.get("name"))
                .and_then(|name| name.as_str())
        };
        assert_eq!(
            named("FR"),
            Some("France"),
            "France keeps its code though Natural Earth's ISO_A2 lacks it"
        );
        assert_eq!(named("JP"), Some("Japan"));
        assert_eq!(
            COUNTRIES
                .iter()
                .filter(|country| iso(country).is_none())
                .count(),
            2
        );
    }

    #[test]
    fn countries_wash_by_iso_code() {
        let layer = countries(
            vec![("FR".into(), 10.0), ("JP".into(), 30.0), ("SG".into(), 5.0)],
            0,
        );
        let france = COUNTRIES
            .iter()
            .find(|country| iso(country) == Some("FR"))
            .expect("France");
        assert_eq!(layer.share(france), Some(0.2), "France keys by its code");
        assert_eq!(
            layer.unmatched().into_iter().collect::<Vec<_>>(),
            ["SG"],
            "Singapore has no country here"
        );
    }

    #[test]
    fn a_code_is_a_country_by_iso_or_by_the_bundle() {
        assert!(known("FR"));
        assert!(known("XK"), "Kosovo, in the bundle though not in ISO 3166");
        assert!(
            known("SG"),
            "Singapore, in ISO 3166 though not in the bundle"
        );
        assert!(!known("ZZ"));
    }

    #[test]
    fn natural_earth_is_named_unless_the_map_names_its_source() {
        let map = || MapView::new("map", MapViewport::new(LatLon::new(0.0, 0.0), 0.0));
        let source = |map: MapView| sourced(map).attribution.map(|text| text.to_string());
        assert_eq!(source(map()), Some("Countries: Natural Earth".into()));
        assert_eq!(
            source(map().attribution("Tiles: Ely")),
            Some("Tiles: Ely".into())
        );
    }
}
