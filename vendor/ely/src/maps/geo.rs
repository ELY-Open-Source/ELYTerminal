use std::f64::consts::PI;

/// A place on the globe in degrees: latitude north, longitude east.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

impl LatLon {
    /// Fails past 90° of latitude or 180° of longitude.
    pub fn new(lat: f64, lon: f64) -> Self {
        assert!(
            (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon),
            "no place at {lat}, {lon}"
        );
        Self { lat, lon }
    }
}

/// Web Mercator's last latitude, where its square world ends.
pub const MAX_LAT: f64 = 85.051_128_78;

/// A place on Web Mercator's unit square: x east from 180° west, y south from the top.
pub(crate) fn project(at: LatLon) -> (f64, f64) {
    let phi = at.lat.clamp(-MAX_LAT, MAX_LAT).to_radians();
    let y = (1.0 - (phi.tan() + 1.0 / phi.cos()).ln() / PI) / 2.0;
    ((at.lon + 180.0) / 360.0, y)
}

/// The place at a point of the unit square; x wraps around the globe.
pub(crate) fn unproject((x, y): (f64, f64)) -> LatLon {
    let lat = (PI * (1.0 - 2.0 * y.clamp(0.0, 1.0))).sinh().atan();
    LatLon::new(lat.to_degrees(), x.rem_euclid(1.0) * 360.0 - 180.0)
}

/// A tile of the world at zoom `z`: `x` east and `y` south, each from 0 to 2^z − 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Tile {
    pub z: u8,
    pub x: u32,
    pub y: u32,
}

/// A tile laid in a view: its column before it wraps around the globe, and its box in whole view pixels, so neighbors meet without a seam.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Laid {
    pub tile: Tile,
    pub column: i64,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Where a map looks: the place at its center, and its zoom, 0 for the world in one tile.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapViewport {
    pub center: LatLon,
    pub zoom: f64,
}

impl MapViewport {
    /// The farthest out and the nearest in a map zooms.
    pub const ZOOMS: (f64, f64) = (0.0, 19.0);

    pub fn new(center: LatLon, zoom: f64) -> Self {
        let (least, most) = Self::ZOOMS;
        assert!(
            (least..=most).contains(&zoom),
            "zoom {zoom} past {least} to {most}"
        );
        Self { center, zoom }
    }

    /// The world's side in view pixels.
    fn world(&self, tile: f32) -> f64 {
        f64::from(tile) * self.zoom.exp2()
    }

    /// The center in world pixels.
    fn middle(&self, tile: f32) -> (f64, f64) {
        let (x, y) = project(self.center);
        let side = self.world(tile);
        (x * side, y * side)
    }

    /// A place in view pixels from the view's top left: of the globe's copies, the one nearest the center.
    pub(crate) fn to_view(self, at: LatLon, (w, h): (f32, f32), tile: f32) -> (f32, f32) {
        let side = self.world(tile);
        let (cx, cy) = self.middle(tile);
        let (x, y) = project(at);
        let across = (x * side - cx + side / 2.0).rem_euclid(side) - side / 2.0;
        (
            (across + f64::from(w) / 2.0) as f32,
            (y * side - cy + f64::from(h) / 2.0) as f32,
        )
    }

    /// The place at a view point.
    pub(crate) fn to_geo(self, (x, y): (f32, f32), (w, h): (f32, f32), tile: f32) -> LatLon {
        let side = self.world(tile);
        let (cx, cy) = self.middle(tile);
        unproject((
            (cx + f64::from(x) - f64::from(w) / 2.0) / side,
            (cy + f64::from(y) - f64::from(h) / 2.0) / side,
        ))
    }

    /// Moved so the map slides by a view distance.
    pub(crate) fn panned(self, (dx, dy): (f32, f32), size: (f32, f32), tile: f32) -> Self {
        let (cx, cy) = self.middle(tile);
        self.centered((cx - f64::from(dx), cy - f64::from(dy)), size, tile)
    }

    /// Zoomed by `levels` about a view point, whose place stays under it; held inside the zooms.
    pub(crate) fn zoomed(
        self,
        levels: f64,
        about: (f32, f32),
        size: (f32, f32),
        tile: f32,
    ) -> Self {
        let (least, most) = Self::ZOOMS;
        let (x, y) = project(self.to_geo(about, size, tile));
        let next = Self {
            zoom: (self.zoom + levels).clamp(least, most),
            ..self
        };
        let side = next.world(tile);
        let (w, h) = size;
        next.centered(
            (
                x * side - (f64::from(about.0) - f64::from(w) / 2.0),
                y * side - (f64::from(about.1) - f64::from(h) / 2.0),
            ),
            size,
            tile,
        )
    }

    /// Centered on a world pixel; a world as tall as the view or taller keeps the view filled.
    fn centered(self, (x, y): (f64, f64), (_, h): (f32, f32), tile: f32) -> Self {
        let side = self.world(tile);
        let half = f64::from(h) / 2.0;
        let y = if side >= f64::from(h) {
            y.clamp(half, side - half)
        } else {
            side / 2.0
        };
        Self {
            center: unproject((x / side, y / side)),
            ..self
        }
    }

    /// Places along a line or ring in view pixels, each wrap around the globe undone against the place before, the first place the copy nearest the center.
    pub(crate) fn run(self, places: &[LatLon], size: (f32, f32), tile: f32) -> Vec<(f32, f32)> {
        let Some(first) = places.first() else {
            return Vec::new();
        };
        let side = self.world(tile);
        let (start, _) = self.to_view(*first, size, tile);
        let (cy, begin) = (self.middle(tile).1, project(*first).0);
        let mut last = begin;
        places
            .iter()
            .map(|place| {
                let (x, y) = project(*place);
                let x = x + (last - x + 0.5).floor();
                last = x;
                (
                    start + ((x - begin) * side) as f32,
                    (y * side - cy + f64::from(size.1) / 2.0) as f32,
                )
            })
            .collect()
    }

    /// The shifts by whole worlds that bring a span of view x, from `least` to `most`, into a view `w` wide.
    pub(crate) fn copies(self, (least, most): (f32, f32), w: f32, tile: f32) -> Vec<f32> {
        let side = self.world(tile) as f32;
        let first = (-most / side).ceil() as i64;
        let last = ((w - least) / side).floor() as i64;
        (first..=last).map(|shift| shift as f32 * side).collect()
    }

    /// Shows `places` whole and centered, `margin` view pixels clear of each side, never farther out than now; places at one spot bring it two levels nearer.
    pub(crate) fn fitting(
        self,
        places: &[LatLon],
        size: (f32, f32),
        tile: f32,
        margin: f32,
    ) -> Self {
        let unit = MapViewport { zoom: 0.0, ..self };
        let run = unit.run(places, (0.0, 0.0), tile);
        let span = |pick: fn(&(f32, f32)) -> f32| {
            let values = run.iter().map(pick);
            let least = values.clone().fold(f32::INFINITY, f32::min);
            (least, values.fold(f32::NEG_INFINITY, f32::max))
        };
        let ((left, right), (top, bottom)) = (span(|point| point.0), span(|point| point.1));
        let (w, h) = (right - left, bottom - top);
        let room = (
            (size.0 - margin * 2.0).max(1.0),
            (size.1 - margin * 2.0).max(1.0),
        );
        let (least, most) = Self::ZOOMS;
        let zoom = if w <= 0.0 && h <= 0.0 {
            self.zoom + 2.0
        } else {
            f64::from((room.0 / w.max(f32::EPSILON)).min(room.1 / h.max(f32::EPSILON))).log2()
        };
        let middle = unit.to_geo(
            ((left + right) / 2.0, (top + bottom) / 2.0),
            (0.0, 0.0),
            tile,
        );
        let next = Self {
            center: middle,
            zoom: zoom.max(self.zoom).clamp(least, most),
        };
        let (x, y) = project(middle);
        let side = next.world(tile);
        next.centered((x * side, y * side), size, tile)
    }

    /// The tiles that cover a view, cut at the nearest zoom within `zooms` and scaled to the view's.
    pub(crate) fn tiles(&self, (w, h): (f32, f32), tile: f32, zooms: (u8, u8)) -> Vec<Laid> {
        if w <= 0.0 || h <= 0.0 {
            return Vec::new();
        }
        let z = (self.zoom.round() as u8).clamp(zooms.0, zooms.1);
        let count = 1i64 << z;
        let side = f64::from(tile) * (self.zoom - f64::from(z)).exp2();
        let (cx, cy) = self.middle(tile);
        let (left, top) = (f64::from(w) / 2.0 - cx, f64::from(h) / 2.0 - cy);
        let span =
            |from: f64, to: f64| (from / side).floor() as i64..=(to / side).ceil() as i64 - 1;
        let edge = |start: f64, ix: i64| (start + ix as f64 * side).round() as f32;
        let rows = span(-top, f64::from(h) - top);
        let (first, last) = ((*rows.start()).max(0), (*rows.end()).min(count - 1));
        span(-left, f64::from(w) - left)
            .flat_map(|column| {
                (first..=last).map(move |row| {
                    let (x, y) = (edge(left, column), edge(top, row));
                    Laid {
                        tile: Tile {
                            z,
                            x: column.rem_euclid(count) as u32,
                            y: row as u32,
                        },
                        column,
                        x,
                        y,
                        w: edge(left, column + 1) - x,
                        h: edge(top, row + 1) - y,
                    }
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{LatLon, MAX_LAT, MapViewport, Tile, project, unproject};

    const TILE: f32 = 256.0;
    const VIEW: (f32, f32) = (800.0, 600.0);

    fn near(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3
    }

    #[test]
    fn the_square_holds_the_globe_up_to_its_last_latitude() {
        assert_eq!(project(LatLon::new(0.0, 0.0)), (0.5, 0.5));
        let (_, top) = project(LatLon::new(MAX_LAT, -180.0));
        assert!(top.abs() < 1e-9, "the last latitude is the top: {top}");
        assert_eq!(
            project(LatLon::new(90.0, 0.0)),
            project(LatLon::new(MAX_LAT, 0.0))
        );
        let london = LatLon::new(51.5072, -0.1276);
        let back = unproject(project(london));
        assert!((back.lat - london.lat).abs() < 1e-9 && (back.lon - london.lon).abs() < 1e-9);
        assert_eq!(
            unproject((1.25, 0.5)).lon,
            -90.0,
            "x wraps around the globe"
        );
    }

    #[test]
    #[should_panic(expected = "no place at 91, 0")]
    fn a_place_past_the_pole_fails() {
        LatLon::new(91.0, 0.0);
    }

    #[test]
    #[should_panic(expected = "zoom 20 past 0 to 19")]
    fn a_zoom_past_the_nearest_fails() {
        MapViewport::new(LatLon::new(0.0, 0.0), 20.0);
    }

    #[test]
    fn a_place_nearest_the_center_across_the_date_line() {
        let view = MapViewport::new(LatLon::new(0.0, 179.0), 2.0);
        let (x, _) = view.to_view(LatLon::new(0.0, -179.0), VIEW, TILE);
        let degree = TILE * 4.0 / 360.0;
        assert!(
            (x - (400.0 + 2.0 * degree)).abs() < 1e-3,
            "east of the center, not a world away: {x}"
        );
    }

    #[test]
    fn a_pan_slides_the_map_with_the_pointer() {
        let view = MapViewport::new(LatLon::new(20.0, 10.0), 3.0);
        let place = view.to_geo((300.0, 200.0), VIEW, TILE);
        let moved = view.panned((50.0, -30.0), VIEW, TILE);
        assert!(near(moved.to_view(place, VIEW, TILE), (350.0, 170.0)));
    }

    #[test]
    fn a_zoom_keeps_the_place_under_the_pointer() {
        let view = MapViewport::new(LatLon::new(40.0, -74.0), 4.0);
        let about = (620.0, 150.0);
        let place = view.to_geo(about, VIEW, TILE);
        let nearer = view.zoomed(1.5, about, VIEW, TILE);
        assert_eq!(nearer.zoom, 5.5);
        assert!(near(nearer.to_view(place, VIEW, TILE), about));
        assert_eq!(
            view.zoomed(99.0, about, VIEW, TILE).zoom,
            MapViewport::ZOOMS.1
        );
    }

    #[test]
    fn the_world_fills_the_view_it_is_tall_enough_for() {
        let north = MapViewport::new(LatLon::new(80.0, 0.0), 2.0).panned((0.0, 0.0), VIEW, TILE);
        let (_, top) = north.to_view(LatLon::new(MAX_LAT, 0.0), VIEW, TILE);
        assert!(
            top.abs() < 1e-3,
            "the top edge stays at the view's top: {top}"
        );
        let whole = MapViewport::new(LatLon::new(60.0, 0.0), 0.0).panned((0.0, 40.0), VIEW, TILE);
        assert!(
            whole.center.lat.abs() < 1e-9,
            "a world shorter than the view sits in its middle"
        );
    }

    #[test]
    fn tiles_cover_the_view_and_wrap_around_the_globe() {
        let view = MapViewport::new(LatLon::new(0.0, 0.0), 1.0);
        let laid = view.tiles(VIEW, TILE, (0, 19));
        let columns: Vec<i64> = laid.iter().map(|laid| laid.column).collect();
        assert_eq!(laid.len(), 8, "two rows of four columns");
        assert_eq!(columns.iter().min(), Some(&-1));
        assert_eq!(columns.iter().max(), Some(&2));
        let west = laid
            .iter()
            .find(|laid| laid.column == -1)
            .expect("a copy past the west edge");
        assert_eq!(west.tile, Tile { z: 1, x: 1, y: 0 });
        assert_eq!(
            (west.x, west.y, west.w, west.h),
            (-112.0, 44.0, 256.0, 256.0)
        );
        assert!(
            laid.iter()
                .all(|laid| laid.x < VIEW.0 && laid.x + laid.w > 0.0)
        );
    }

    #[test]
    fn tiles_meet_on_whole_pixels_at_a_fractional_zoom() {
        let laid = MapViewport::new(LatLon::new(12.3, 45.6), 2.37).tiles(VIEW, TILE, (0, 19));
        for tile in &laid {
            assert_eq!(
                (tile.x.fract(), tile.y.fract(), tile.w.fract()),
                (0.0, 0.0, 0.0)
            );
            let next = |column: i64, row: u32| {
                laid.iter()
                    .find(|each| each.column == column && each.tile.y == row)
            };
            if let Some(east) = next(tile.column + 1, tile.tile.y) {
                assert_eq!(
                    tile.x + tile.w,
                    east.x,
                    "no seam between {tile:?} and {east:?}"
                );
            }
            if let Some(south) = next(tile.column, tile.tile.y + 1) {
                assert_eq!(tile.y + tile.h, south.y);
            }
        }
    }

    #[test]
    fn a_run_crosses_the_date_line_the_short_way() {
        let view = MapViewport::new(LatLon::new(0.0, 180.0), 3.0);
        let run = view.run(
            &[LatLon::new(0.0, 170.0), LatLon::new(0.0, -170.0)],
            VIEW,
            TILE,
        );
        let degree = TILE * 8.0 / 360.0;
        assert!((run[0].0 - (400.0 - 10.0 * degree)).abs() < 1e-2, "{run:?}");
        assert!(
            (run[1].0 - (400.0 + 10.0 * degree)).abs() < 1e-2,
            "east across the line: {run:?}"
        );
    }

    #[test]
    fn copies_repeat_a_span_across_every_world_in_view() {
        let whole = MapViewport::new(LatLon::new(0.0, 0.0), 0.0);
        assert_eq!(
            whole.copies((0.0, 10.0), 800.0, TILE),
            [0.0, 256.0, 512.0, 768.0]
        );
        assert_eq!(
            whole.copies((-300.0, -200.0), 800.0, TILE),
            [256.0, 512.0, 768.0, 1024.0]
        );
        let near = MapViewport::new(LatLon::new(0.0, 0.0), 6.0);
        assert_eq!(
            near.copies((100.0, 200.0), 800.0, TILE),
            [0.0],
            "one world when it is wider than the view"
        );
    }

    #[test]
    fn a_fit_shows_its_places_whole_and_never_zooms_out() {
        let view = MapViewport::new(LatLon::new(50.0, 0.0), 3.0);
        let places = [
            LatLon::new(51.5, -0.1),
            LatLon::new(48.9, 2.35),
            LatLon::new(52.5, 13.4),
        ];
        let fit = view.fitting(&places, VIEW, TILE, 32.0);
        assert!(fit.zoom > view.zoom, "nearer in: {}", fit.zoom);
        for place in places {
            let (x, y) = fit.to_view(place, VIEW, TILE);
            assert!(
                (31.9..=768.1).contains(&x) && (31.9..=568.1).contains(&y),
                "{place:?} at {x}, {y}"
            );
        }
        let one = view.fitting(
            &[LatLon::new(1.0, 1.0), LatLon::new(1.0, 1.0)],
            VIEW,
            TILE,
            32.0,
        );
        assert_eq!(one.zoom, 5.0, "places at one spot come two levels nearer");
        let wide = [LatLon::new(60.0, -150.0), LatLon::new(-40.0, 150.0)];
        assert_eq!(
            view.fitting(&wide, VIEW, TILE, 32.0).zoom,
            3.0,
            "never farther out"
        );
    }

    #[test]
    fn tiles_past_the_hosts_zooms_scale() {
        let view = MapViewport::new(LatLon::new(0.0, 0.0), 5.0);
        let laid = view.tiles(VIEW, TILE, (0, 3));
        assert!(
            laid.iter()
                .all(|laid| laid.tile.z == 3 && laid.w == 1024.0 && laid.h == 1024.0)
        );
        let far = MapViewport::new(LatLon::new(0.0, 0.0), 2.4).tiles(VIEW, TILE, (0, 19));
        assert!(
            far.iter().all(|laid| laid.tile.z == 2),
            "the nearest whole zoom"
        );
        assert!(view.tiles((0.0, 600.0), TILE, (0, 19)).is_empty());
    }
}
