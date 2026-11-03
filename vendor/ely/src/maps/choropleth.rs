use std::{
    collections::{BTreeSet, HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

use gpui::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
    linear_color_stop, linear_gradient,
};
use serde_json::Value;

use super::{Feature, MapView};
use crate::{
    theme::{ActiveTheme, Radius, TextSize},
    typography::format::{Separators, number},
};

/// How a feature finds its value: by its id, or by a property's text or number.
#[derive(Clone, Debug, PartialEq)]
pub enum ChoroplethKey {
    Id,
    Property(SharedString),
}

/// Areas washed each by its value, from the hue's faintest at the least value to its strongest at the most; an area without a value reads as no data. Its points and lines draw in their wash.
#[derive(Clone)]
pub struct ChoroplethLayer {
    pub(crate) features: Arc<[Feature]>,
    values: Rc<HashMap<String, f64>>,
    key: ChoroplethKey,
    pub(crate) hue: usize,
}

impl ChoroplethLayer {
    /// Values by each feature's key, its id unless `key_by` names a property; fails on a value that is not finite or a key given twice.
    pub fn new(
        features: impl Into<Arc<[Feature]>>,
        values: impl IntoIterator<Item = (impl Into<String>, f64)>,
    ) -> Self {
        let mut kept = HashMap::new();
        for (key, value) in values {
            let key: String = key.into();
            assert!(value.is_finite(), "choropleth value {value} for {key}");
            assert!(
                kept.insert(key.clone(), value).is_none(),
                "choropleth values repeat {key}"
            );
        }
        let values = kept;
        Self {
            features: features.into(),
            values: Rc::new(values),
            key: ChoroplethKey::Id,
            hue: 0,
        }
    }

    /// Finds each feature's value by a property's text or number.
    pub fn key_by(mut self, property: impl Into<SharedString>) -> Self {
        self.key = ChoroplethKey::Property(property.into());
        self
    }

    /// The chart hue the scale runs to.
    pub fn hue(mut self, hue: usize) -> Self {
        self.hue = hue;
        self
    }

    /// The least and the most value; none without values.
    pub(crate) fn span(&self) -> Option<(f64, f64)> {
        let mut values = self.values.values().copied();
        let first = values.next()?;
        Some(values.fold((first, first), |(least, most), value| {
            (least.min(value), most.max(value))
        }))
    }

    /// A feature's key: its id, or its property's text or number.
    pub(crate) fn key_of(&self, feature: &Feature) -> Option<String> {
        match &self.key {
            ChoroplethKey::Id => feature.id.clone(),
            ChoroplethKey::Property(name) => match feature.properties.get(name.as_ref())? {
                Value::String(text) => Some(text.clone()),
                Value::Number(number) => Some(number.to_string()),
                _ => None,
            },
        }
    }

    /// Where a feature's value lies in the span, 0 at the least and 1 at the most, 1 when they are one; none without a value.
    pub(crate) fn share(&self, feature: &Feature) -> Option<f64> {
        let value = *self.values.get(&self.key_of(feature)?)?;
        let (least, most) = self.span()?;
        Some(match most > least {
            true => (value - least) / (most - least),
            false => 1.0,
        })
    }

    /// The keys of values that match no feature, in order.
    pub(crate) fn unmatched(&self) -> BTreeSet<String> {
        let keys: HashSet<String> = self
            .features
            .iter()
            .filter_map(|feature| self.key_of(feature))
            .collect();
        self.values
            .keys()
            .filter(|key| !keys.contains(*key))
            .cloned()
            .collect()
    }
}

/// A map of areas washed by value, with a legend of the scale and of no data at its top left, clear of the zoom buttons and the attribution. It takes the owner's map, with or without tiles, and lays the layer on it; values that match no area are logged once.
#[derive(IntoElement)]
pub struct ChoroplethMap {
    id: ElementId,
    map: MapView,
    layer: ChoroplethLayer,
}

impl ChoroplethMap {
    pub fn new(id: impl Into<ElementId>, map: MapView, layer: ChoroplethLayer) -> Self {
        Self {
            id: id.into(),
            map,
            layer,
        }
    }
}

/// A legend number: tenths across a span under ten, whole numbers past it.
fn label(value: f64, (least, most): (f64, f64)) -> String {
    number(
        value,
        if most - least < 10.0 { 1 } else { 0 },
        Separators::EN,
    )
}

impl RenderOnce for ChoroplethMap {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let logged =
            window.use_keyed_state((self.id.clone(), "unmatched"), cx, |_, _| BTreeSet::new());
        let unmatched = self.layer.unmatched();
        if !unmatched.is_empty() && *logged.read(cx) != unmatched {
            log::warn!("choropleth {:?}: no area for {unmatched:?}", self.id);
            logged.update(cx, |logged, _| *logged = unmatched);
        }
        let theme = cx.theme();
        let colors = &theme.colors;
        let ink = colors.hue(self.layer.hue, format_args!("choropleth {:?}", self.id));
        let small = |text: String| div().flex_none().child(text);
        let scale = self.layer.span().map(|span| {
            div()
                .flex()
                .items_center()
                .gap_1p5()
                .child(small(label(span.0, span)))
                .child(
                    div()
                        .w(theme.maps().legend)
                        .h(theme.maps().dot)
                        .rounded(theme.radius(Radius::Sm))
                        .bg(linear_gradient(
                            90.0,
                            linear_color_stop(ink.alpha(0.15), 0.0),
                            linear_color_stop(ink, 1.0),
                        )),
                )
                .child(small(label(span.1, span)))
        });
        let legend = div()
            .absolute()
            .top_2()
            .left_2()
            .flex()
            .flex_col()
            .gap_1()
            .p_2()
            .rounded(theme.radius(Radius::Md))
            .bg(colors.overlay.alpha(0.9))
            .border_1()
            .border_color(colors.border)
            .text_size(theme.text_size(TextSize::Xs))
            .text_color(colors.fg_muted)
            .children(scale)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .child(
                        div()
                            .size(theme.maps().dot)
                            .rounded(theme.radius(Radius::Sm))
                            .bg(colors.fg_disabled.alpha(0.2)),
                    )
                    .child("No data"),
            );
        div()
            .relative()
            .h_full()
            .child(self.map.layer(self.layer))
            .child(legend)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Map, json};

    use super::ChoroplethLayer;
    use crate::maps::{Feature, Geometry, LatLon};

    fn feature(id: Option<&str>, properties: serde_json::Value) -> Feature {
        let properties: Map<String, serde_json::Value> =
            serde_json::from_value(properties).expect("an object");
        Feature {
            id: id.map(str::to_string),
            geometry: Some(Geometry::Point(LatLon::new(0.0, 0.0))),
            properties,
        }
    }

    #[test]
    fn a_feature_finds_its_share_of_the_span_by_id_or_property() {
        let features = vec![
            feature(Some("a"), json!({"code": "AA", "rank": 3})),
            feature(Some("b"), json!({"code": "BB"})),
            feature(None, json!({"code": "CC"})),
        ];
        let by_id = ChoroplethLayer::new(features.clone(), [("a", 10.0), ("b", 30.0)]);
        assert_eq!(by_id.span(), Some((10.0, 30.0)));
        assert_eq!(by_id.share(&features[0]), Some(0.0));
        assert_eq!(by_id.share(&features[1]), Some(1.0));
        assert_eq!(by_id.share(&features[2]), None, "no id, no value");
        let by_code =
            ChoroplethLayer::new(features.clone(), [("CC", 5.0), ("BB", 15.0), ("DD", 10.0)])
                .key_by("code");
        assert_eq!(by_code.share(&features[2]), Some(0.0));
        assert_eq!(
            by_code.share(&features[0]),
            None,
            "a feature without a value reads as no data"
        );
        assert_eq!(by_code.unmatched().into_iter().collect::<Vec<_>>(), ["DD"]);
        let by_rank = ChoroplethLayer::new(features.clone(), [("3", 1.0)]).key_by("rank");
        assert_eq!(
            by_rank.share(&features[0]),
            Some(1.0),
            "a number property keys as text; one value fills the scale"
        );
    }

    #[test]
    fn legend_numbers_keep_tenths_across_a_narrow_span() {
        assert_eq!(super::label(2.5, (0.0, 5.0)), "2.5");
        assert_eq!(super::label(12_345.6, (0.0, 20_000.0)), "12,346");
    }

    #[test]
    #[should_panic(expected = "choropleth value NaN for a")]
    fn a_value_that_is_not_a_number_fails() {
        ChoroplethLayer::new(Vec::<Feature>::new(), [("a", f64::NAN)]);
    }

    #[test]
    #[should_panic(expected = "choropleth value inf for a")]
    fn an_endless_value_fails() {
        ChoroplethLayer::new(Vec::<Feature>::new(), [("a", f64::INFINITY)]);
    }

    #[test]
    #[should_panic(expected = "choropleth values repeat US")]
    fn a_key_given_twice_fails() {
        ChoroplethLayer::new(Vec::<Feature>::new(), [("US", 1.0), ("US", 100.0)]);
    }
}
