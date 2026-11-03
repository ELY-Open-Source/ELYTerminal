use anyhow::{Context, Result, bail};
use serde_json::{Map, Value};

use super::LatLon;

/// A GeoJSON shape with its positions as places, as RFC 7946 has it.
#[derive(Clone, Debug, PartialEq)]
pub enum Geometry {
    Point(LatLon),
    MultiPoint(Vec<LatLon>),
    LineString(Vec<LatLon>),
    MultiLineString(Vec<Vec<LatLon>>),
    /// Rings: the outer one first, then its holes.
    Polygon(Vec<Vec<LatLon>>),
    MultiPolygon(Vec<Vec<Vec<LatLon>>>),
    Collection(Vec<Geometry>),
}

/// A GeoJSON feature: its id when it has one, its shape when it is placed, and its properties.
#[derive(Clone, Debug, PartialEq)]
pub struct Feature {
    pub id: Option<String>,
    pub geometry: Option<Geometry>,
    pub properties: Map<String, Value>,
}

/// Reads GeoJSON: a FeatureCollection, a Feature or a bare geometry. Fails on text that is not JSON, an unknown type, a position past the globe, a line of one position or a ring that does not close.
pub fn read_geojson(text: &str) -> Result<Vec<Feature>> {
    let json: Value = serde_json::from_str(text).context("the GeoJSON is not JSON")?;
    match kind(&json)? {
        "FeatureCollection" => json
            .get("features")
            .and_then(Value::as_array)
            .context("a FeatureCollection without features")?
            .iter()
            .enumerate()
            .map(|(ix, each)| feature(each).with_context(|| format!("feature {ix}")))
            .collect(),
        "Feature" => Ok(vec![feature(&json)?]),
        _ => Ok(vec![Feature {
            id: None,
            geometry: Some(geometry(&json)?),
            properties: Map::new(),
        }]),
    }
}

fn kind(json: &Value) -> Result<&str> {
    json.get("type")
        .and_then(Value::as_str)
        .context("a GeoJSON object without a type")
}

fn feature(json: &Value) -> Result<Feature> {
    let kind = kind(json)?;
    if kind != "Feature" {
        bail!("a {kind} where a Feature stands");
    }
    let id = match json.get("id") {
        None | Some(Value::Null) => None,
        Some(Value::String(text)) => Some(text.clone()),
        Some(Value::Number(number)) => Some(number.to_string()),
        Some(other) => bail!("an id {other} that is neither text nor a number"),
    };
    let geometry = match json.get("geometry") {
        None => bail!("a Feature without a geometry member"),
        Some(Value::Null) => None,
        Some(shape) => Some(geometry(shape)?),
    };
    let properties = match json.get("properties") {
        None | Some(Value::Null) => Map::new(),
        Some(Value::Object(map)) => map.clone(),
        Some(other) => bail!("properties {other} that are not an object"),
    };
    Ok(Feature {
        id,
        geometry,
        properties,
    })
}

fn geometry(json: &Value) -> Result<Geometry> {
    let coordinates = || {
        json.get("coordinates")
            .context("a geometry without coordinates")
    };
    Ok(match kind(json)? {
        "Point" => Geometry::Point(position(coordinates()?)?),
        "MultiPoint" => Geometry::MultiPoint(list(coordinates()?, position)?),
        "LineString" => Geometry::LineString(line(coordinates()?)?),
        "MultiLineString" => Geometry::MultiLineString(list(coordinates()?, line)?),
        "Polygon" => Geometry::Polygon(list(coordinates()?, ring)?),
        "MultiPolygon" => {
            Geometry::MultiPolygon(list(coordinates()?, |polygon| list(polygon, ring))?)
        }
        "GeometryCollection" => Geometry::Collection(
            json.get("geometries")
                .and_then(Value::as_array)
                .context("a GeometryCollection without geometries")?
                .iter()
                .map(geometry)
                .collect::<Result<_>>()?,
        ),
        other => bail!("an unknown GeoJSON type {other:?}"),
    })
}

fn list<T>(json: &Value, each: impl Fn(&Value) -> Result<T>) -> Result<Vec<T>> {
    json.as_array()
        .with_context(|| format!("coordinates {json} that are not a list"))?
        .iter()
        .map(each)
        .collect()
}

/// A position: longitude, then latitude; an altitude past them is dropped.
fn position(json: &Value) -> Result<LatLon> {
    let numbers = json
        .as_array()
        .with_context(|| format!("a position {json} that is not a list"))?;
    let axis = |ix: usize| {
        numbers
            .get(ix)
            .and_then(Value::as_f64)
            .with_context(|| format!("a position {json} without a number at {ix}"))
    };
    let (lon, lat) = (axis(0)?, axis(1)?);
    if !(-180.0..=180.0).contains(&lon) || !(-90.0..=90.0).contains(&lat) {
        bail!("a position {json} past the globe");
    }
    Ok(LatLon::new(lat, lon))
}

fn line(json: &Value) -> Result<Vec<LatLon>> {
    let line = list(json, position)?;
    if line.len() < 2 {
        bail!("a line of {} position", line.len());
    }
    Ok(line)
}

fn ring(json: &Value) -> Result<Vec<LatLon>> {
    let ring = list(json, position)?;
    if ring.len() < 4 {
        bail!(
            "a ring of {} positions; a ring holds four or more",
            ring.len()
        );
    }
    if ring.first() != ring.last() {
        bail!("a ring that does not close");
    }
    Ok(ring)
}

#[cfg(test)]
mod tests {
    use super::{Geometry, read_geojson};
    use crate::maps::LatLon;

    const SAMPLE: &str = r#"{"type": "FeatureCollection", "features": [
        {"type": "Feature", "id": 7, "properties": {"name": "Spot"},
         "geometry": {"type": "Point", "coordinates": [2.35, 48.86, 35.0]}},
        {"type": "Feature", "id": "path", "properties": null,
         "geometry": {"type": "LineString", "coordinates": [[0, 0], [10, 5]]}},
        {"type": "Feature", "geometry": {"type": "Polygon", "coordinates": [
            [[0, 0], [4, 0], [4, 4], [0, 4], [0, 0]],
            [[1, 1], [2, 1], [2, 2], [1, 1]]]}},
        {"type": "Feature", "geometry": null}
    ]}"#;

    #[test]
    fn a_collection_reads_its_features_in_order() {
        let features = read_geojson(SAMPLE).expect("valid GeoJSON");
        assert_eq!(features.len(), 4);
        assert_eq!(features[0].id.as_deref(), Some("7"));
        assert_eq!(features[0].properties["name"], "Spot");
        assert_eq!(
            features[0].geometry,
            Some(Geometry::Point(LatLon::new(48.86, 2.35))),
            "longitude first, the altitude dropped"
        );
        assert_eq!(features[1].id.as_deref(), Some("path"));
        assert!(features[1].properties.is_empty());
        match &features[2].geometry {
            Some(Geometry::Polygon(rings)) => assert_eq!((rings.len(), rings[1].len()), (2, 4)),
            other => panic!("a polygon with a hole, not {other:?}"),
        }
        assert_eq!(features[3].geometry, None, "a feature placed nowhere");
    }

    #[test]
    fn bare_geometries_and_collections_read() {
        let multi = r#"{"type": "MultiPolygon", "coordinates": [[[[0,0],[1,0],[1,1],[0,0]]]]}"#;
        assert!(matches!(
            read_geojson(multi).expect("valid")[0].geometry,
            Some(Geometry::MultiPolygon(_))
        ));
        let set = r#"{"type": "GeometryCollection", "geometries": [
            {"type": "MultiPoint", "coordinates": [[1, 2]]},
            {"type": "MultiLineString", "coordinates": [[[0, 0], [1, 1]]]}]}"#;
        match &read_geojson(set).expect("valid")[0].geometry {
            Some(Geometry::Collection(parts)) => assert_eq!(parts.len(), 2),
            other => panic!("a collection, not {other:?}"),
        }
    }

    #[test]
    fn broken_geojson_fails_with_its_reason() {
        let reason = |text: &str| format!("{:#}", read_geojson(text).expect_err("broken"));
        assert!(reason("{").contains("not JSON"));
        assert!(
            reason(r#"{"type": "Circle", "coordinates": [0, 0]}"#).contains("unknown GeoJSON type")
        );
        assert!(reason(r#"{"type": "Point", "coordinates": [200, 0]}"#).contains("past the globe"));
        assert!(reason(r#"{"type": "Point", "coordinates": [0, 95]}"#).contains("past the globe"));
        assert!(reason(r#"{"type": "LineString", "coordinates": [[0, 0]]}"#).contains("line of 1"));
        let open = r#"{"type": "Polygon", "coordinates": [[[0,0],[1,0],[1,1],[0,1]]]}"#;
        assert!(reason(open).contains("does not close"));
        let short = r#"{"type": "Polygon", "coordinates": [[[0,0],[1,1],[0,0]]]}"#;
        assert!(reason(short).contains("four or more"));
        let named = r#"{"type": "FeatureCollection", "features": [{"type": "Feature", "geometry": {"type": "Point"}}]}"#;
        let named = reason(named);
        assert!(
            named.contains("feature 0") && named.contains("without coordinates"),
            "{named}"
        );
    }
}
