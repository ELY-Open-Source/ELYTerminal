use jiff::{SignedDuration, Timestamp, civil::Date, tz::TimeZone};

use super::event::{Event, When};

/// Minutes in a day.
pub(crate) const DAY: i64 = 24 * 60;

/// The step a drag snaps to, in minutes.
pub(crate) const SNAP: i64 = 15;

/// An event's stretch of `day` in `zone`, in minutes from midnight: clipped to the day, as a night that runs past midnight shows on both.
pub(crate) fn minutes(event: &Event, day: Date, zone: &TimeZone) -> Option<(i64, i64)> {
    let When::Timed { start, end } = event.when else {
        return None;
    };
    let midnight = day
        .to_zoned(zone.clone())
        .expect("the zone holds the day")
        .timestamp();
    let from = start.duration_since(midnight).as_mins();
    let to = end.duration_since(midnight).as_mins();
    (from < DAY && to > 0).then(|| (from.max(0), to.min(DAY)))
}

/// A block in a day column: which event, its minutes, and its lane among the lanes of the blocks it overlaps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Block {
    pub event: usize,
    pub from: i64,
    pub to: i64,
    pub lane: usize,
    pub lanes: usize,
}

/// A day's timed events as blocks: those that overlap share the width in lanes, earliest first, each in the first lane free.
pub(crate) fn blocks(events: &[Event], day: Date, zone: &TimeZone) -> Vec<Block> {
    let mut spans: Vec<(usize, i64, i64)> = events
        .iter()
        .enumerate()
        .filter_map(|(ix, event)| minutes(event, day, zone).map(|(from, to)| (ix, from, to)))
        .collect();
    spans.sort_by_key(|&(ix, from, to)| (from, -to, ix));
    let mut blocks: Vec<Block> = Vec::with_capacity(spans.len());
    let mut cluster = 0;
    let mut reach = i64::MIN;
    let mut ends: Vec<i64> = Vec::new();
    for (event, from, to) in spans {
        if from >= reach {
            close(&mut blocks[cluster..]);
            cluster = blocks.len();
            ends.clear();
        }
        let lane = ends
            .iter()
            .position(|end| *end <= from)
            .unwrap_or(ends.len());
        match ends.get_mut(lane) {
            Some(end) => *end = to,
            None => ends.push(to),
        }
        reach = reach.max(to);
        blocks.push(Block {
            event,
            from,
            to,
            lane,
            lanes: 0,
        });
    }
    close(&mut blocks[cluster..]);
    blocks
}

/// Gives each block of a cluster the cluster's count of lanes.
fn close(cluster: &mut [Block]) {
    let lanes = cluster
        .iter()
        .map(|block| block.lane + 1)
        .max()
        .unwrap_or(0);
    for block in cluster {
        block.lanes = lanes;
    }
}

/// `minutes` to the nearest step of `SNAP`, inside the day.
pub(crate) fn snap(minutes: f32) -> i64 {
    let step = SNAP as f32;
    ((minutes / step).round() as i64 * SNAP).clamp(0, DAY)
}

/// The quarter hours a drag from `from` to `to` covers, in pixels down a column where an hour is `hour` tall; none under a quarter.
pub(crate) fn dragged(from: f32, to: f32, hour: f32) -> Option<(i64, i64)> {
    let (top, bottom) = (from.min(to), from.max(to));
    let (start, end) = (snap(top / hour * 60.0), snap(bottom / hour * 60.0));
    (end - start >= SNAP).then_some((start, end))
}

/// The moment `minutes` after `day`'s midnight in `zone`.
pub(crate) fn moment(day: Date, minutes: i64, zone: &TimeZone) -> Timestamp {
    let midnight = day
        .to_zoned(zone.clone())
        .expect("the zone holds the day")
        .timestamp();
    midnight + SignedDuration::from_mins(minutes)
}

#[cfg(test)]
mod tests {
    use jiff::{Timestamp, civil::date, tz::TimeZone};

    use super::{Block, blocks, dragged, minutes, moment, snap};
    use crate::calendar::Event;

    fn at(day: i8, hour: i8, minute: i8) -> Timestamp {
        date(2026, 9, day)
            .at(hour, minute, 0, 0)
            .to_zoned(TimeZone::UTC)
            .expect("a UTC time")
            .timestamp()
    }

    fn block(event: usize, from: i64, to: i64, lane: usize, lanes: usize) -> Block {
        Block {
            event,
            from,
            to,
            lane,
            lanes,
        }
    }

    #[test]
    fn a_night_shows_on_both_days_it_touches() {
        let night = Event::timed("night", "Night", at(22, 21, 0), at(23, 2, 30), 0);
        let utc = &TimeZone::UTC;
        assert_eq!(
            minutes(&night, date(2026, 9, 22), utc),
            Some((21 * 60, 24 * 60))
        );
        assert_eq!(minutes(&night, date(2026, 9, 23), utc), Some((0, 150)));
        assert_eq!(minutes(&night, date(2026, 9, 24), utc), None);
        let all = Event::all_day("all", "All", date(2026, 9, 22), date(2026, 9, 22), 0);
        assert_eq!(
            minutes(&all, date(2026, 9, 22), utc),
            None,
            "whole days sit above"
        );
    }

    #[test]
    fn overlapping_blocks_share_the_width_and_the_rest_take_it_all() {
        let events = [
            Event::timed("a", "A", at(22, 9, 0), at(22, 10, 0), 0),
            Event::timed("b", "B", at(22, 9, 30), at(22, 11, 0), 0),
            Event::timed("c", "C", at(22, 10, 0), at(22, 10, 30), 0),
            Event::timed("d", "D", at(22, 13, 0), at(22, 14, 0), 0),
        ];
        assert_eq!(
            blocks(&events, date(2026, 9, 22), &TimeZone::UTC),
            [
                block(0, 540, 600, 0, 2),
                block(1, 570, 660, 1, 2),
                block(2, 600, 630, 0, 2),
                block(3, 780, 840, 0, 1),
            ],
            "C takes A's lane once A ends; D stands alone"
        );
    }

    #[test]
    fn a_drag_snaps_to_quarters_inside_the_day() {
        assert_eq!(snap(7.0), 0);
        assert_eq!(snap(8.0), 15);
        assert_eq!(snap(9.0 * 60.0 + 22.0), 9 * 60 + 15);
        assert_eq!(snap(-30.0), 0);
        assert_eq!(snap(25.0 * 60.0), 24 * 60);
        assert_eq!(
            moment(date(2026, 9, 22), 9 * 60 + 15, &TimeZone::UTC),
            at(22, 9, 15)
        );
    }

    #[test]
    fn a_drag_either_way_covers_its_quarters() {
        let hour = 48.0;
        assert_eq!(dragged(9.0 * hour, 10.5 * hour, hour), Some((540, 630)));
        assert_eq!(
            dragged(10.5 * hour, 9.0 * hour, hour),
            Some((540, 630)),
            "upward"
        );
        assert_eq!(
            dragged(9.0 * hour, 9.1 * hour, hour),
            None,
            "under a quarter"
        );
        assert_eq!(
            dragged(9.0 * hour, 9.25 * hour, hour),
            Some((540, 555)),
            "a quarter"
        );
    }
}
