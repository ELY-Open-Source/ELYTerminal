use gpui::{App, ElementId, IntoElement, RenderOnce, Window};

use super::LatLon;
use crate::typography::CopyableText;

/// How a place is written.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CoordFormat {
    /// 51.5072° N, 0.1276° W
    #[default]
    Decimal,
    /// 51°30′26″ N, 0°07′39″ W
    Dms,
}

/// `at` in `format`; a value that rounds to zero reads north or east.
pub(crate) fn written(at: LatLon, format: CoordFormat) -> String {
    let part = |value: f64, (plus, minus): (char, char)| {
        let body = match format {
            CoordFormat::Decimal => format!("{:.4}°", value.abs()),
            CoordFormat::Dms => {
                let seconds = (value.abs() * 3600.0).round() as u64;
                format!(
                    "{}°{:02}′{:02}″",
                    seconds / 3600,
                    seconds / 60 % 60,
                    seconds % 60
                )
            }
        };
        let zero = body.chars().all(|ch| !ch.is_ascii_digit() || ch == '0');
        let side = if value < 0.0 && !zero { minus } else { plus };
        format!("{body} {side}")
    };
    format!("{}, {}", part(at.lat, ('N', 'S')), part(at.lon, ('E', 'W')))
}

/// A place in words, with tabular figures so it holds still as it changes, and a button that copies it.
#[derive(IntoElement)]
pub struct CoordinateDisplay {
    id: ElementId,
    at: LatLon,
    format: CoordFormat,
}

impl CoordinateDisplay {
    pub fn new(id: impl Into<ElementId>, at: LatLon) -> Self {
        Self {
            id: id.into(),
            at,
            format: CoordFormat::default(),
        }
    }

    pub fn format(mut self, format: CoordFormat) -> Self {
        self.format = format;
        self
    }
}

impl RenderOnce for CoordinateDisplay {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        CopyableText::new(self.id, written(self.at, self.format)).tabular()
    }
}

#[cfg(test)]
mod tests {
    use super::{CoordFormat, written};
    use crate::maps::LatLon;

    #[test]
    fn places_read_with_their_hemispheres() {
        let london = LatLon::new(51.507222, -0.1275);
        assert_eq!(
            written(london, CoordFormat::Decimal),
            "51.5072° N, 0.1275° W"
        );
        assert_eq!(written(london, CoordFormat::Dms), "51°30′26″ N, 0°07′39″ W");
        let rio = LatLon::new(-22.9068, -43.1729);
        assert_eq!(written(rio, CoordFormat::Dms), "22°54′24″ S, 43°10′22″ W");
    }

    #[test]
    fn seconds_carry_into_minutes_and_degrees() {
        let edge = LatLon::new(10.999_999_9, 20.5);
        assert_eq!(written(edge, CoordFormat::Dms), "11°00′00″ N, 20°30′00″ E");
    }

    #[test]
    fn a_value_that_rounds_to_zero_reads_north_and_east() {
        let near = LatLon::new(-0.000_01, -0.000_01);
        assert_eq!(written(near, CoordFormat::Decimal), "0.0000° N, 0.0000° E");
        assert_eq!(written(near, CoordFormat::Dms), "0°00′00″ N, 0°00′00″ E");
        let south = LatLon::new(-0.0001, 0.0);
        assert_eq!(written(south, CoordFormat::Decimal), "0.0001° S, 0.0000° E");
    }
}
