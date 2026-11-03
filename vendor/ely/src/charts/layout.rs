use super::geometry::Rect;

/// Rectangles for values inside `frame`, laid in rows that keep each close to square (Bruls, Huizing and van Wijk); values in the order given, larger first reads best.
pub(crate) fn squarify(values: &[f64], frame: Rect) -> Vec<Rect> {
    let total: f64 = values.iter().filter(|value| **value > 0.0).sum();
    let mut out = vec![Rect::default(); values.len()];
    if total <= 0.0 || frame.w <= 0.0 || frame.h <= 0.0 {
        return out;
    }
    let scale = f64::from(frame.w * frame.h) / total;
    let areas: Vec<(usize, f64)> = values
        .iter()
        .enumerate()
        .filter(|(_, value)| **value > 0.0)
        .map(|(ix, value)| (ix, value * scale))
        .collect();
    let mut rest = frame;
    let mut start = 0;
    while start < areas.len() {
        let side = f64::from(rest.w.min(rest.h));
        let worst = |row: &[(usize, f64)]| {
            let sum: f64 = row.iter().map(|(_, area)| area).sum();
            let (least, most) = row
                .iter()
                .fold((f64::MAX, 0.0f64), |(least, most), (_, area)| {
                    (least.min(*area), most.max(*area))
                });
            (side * side * most / (sum * sum)).max(sum * sum / (side * side * least))
        };
        let mut end = start + 1;
        while end < areas.len() && worst(&areas[start..=end]) <= worst(&areas[start..end]) {
            end += 1;
        }
        let row = &areas[start..end];
        let sum: f64 = row.iter().map(|(_, area)| area).sum();
        let thick = (sum / side) as f32;
        let mut along = 0.0f32;
        for (ix, area) in row {
            let length = (*area / f64::from(thick)) as f32;
            out[*ix] = if rest.w >= rest.h {
                Rect {
                    x: rest.x,
                    y: rest.y + along,
                    w: thick,
                    h: length,
                }
            } else {
                Rect {
                    x: rest.x + along,
                    y: rest.y,
                    w: length,
                    h: thick,
                }
            };
            along += length;
        }
        rest = if rest.w >= rest.h {
            Rect {
                x: rest.x + thick,
                w: rest.w - thick,
                ..rest
            }
        } else {
            Rect {
                y: rest.y + thick,
                h: rest.h - thick,
                ..rest
            }
        };
        start = end;
    }
    out
}

/// Days of a year of counts as a calendar grid: each day's week column and weekday row, Monday first, and each month's first column.
pub(crate) fn calendar(first_weekday: usize, days: usize) -> (Vec<(usize, usize)>, usize) {
    let places = (0..days)
        .map(|day| ((day + first_weekday) / 7, (day + first_weekday) % 7))
        .collect();
    (places, (days + first_weekday).div_ceil(7))
}

/// A count's step on a scale of five, zero apart: none, then quartiles of the highest.
pub(crate) fn level(count: f64, highest: f64) -> usize {
    if count <= 0.0 || highest <= 0.0 {
        return 0;
    }
    (((count / highest) * 4.0).ceil() as usize).clamp(1, 4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_treemap_fills_its_frame_with_areas_that_match() {
        let frame = Rect {
            x: 0.0,
            y: 0.0,
            w: 600.0,
            h: 400.0,
        };
        let rects = squarify(&[6.0, 6.0, 4.0, 3.0, 2.0, 2.0, 1.0], frame);
        let area: f32 = rects.iter().map(|rect| rect.w * rect.h).sum();
        assert!(
            (area - 240_000.0).abs() < 1.0,
            "the rectangles fill the frame, got {area}"
        );
        assert!(
            (rects[0].w * rects[0].h - 60_000.0).abs() < 1.0,
            "a quarter of the total is a quarter of the area"
        );
        assert!(rects.iter().all(|rect| rect.x >= -0.01
            && rect.y >= -0.01
            && rect.x + rect.w <= 600.01
            && rect.y + rect.h <= 400.01));
        let worst = rects
            .iter()
            .map(|rect| (rect.w / rect.h).max(rect.h / rect.w))
            .fold(0.0f32, f32::max);
        assert!(worst < 3.0, "no sliver: the worst aspect is {worst}");
    }

    #[test]
    fn a_calendar_starts_on_its_weekday_and_counts_weeks() {
        let (places, weeks) = calendar(2, 10);
        assert_eq!(places[0], (0, 2), "the first day sits on its weekday");
        assert_eq!(places[5], (1, 0), "the sixth day starts the next week");
        assert_eq!(weeks, 2);
        assert_eq!(
            (level(0.0, 9.0), level(1.0, 9.0), level(9.0, 9.0)),
            (0, 1, 4)
        );
    }
}
