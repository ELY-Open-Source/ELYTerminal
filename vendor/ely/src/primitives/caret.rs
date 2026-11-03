use gpui::{Bounds, Pixels, Point, size};

fn snap(logical: Pixels, least: f32, scale: f32) -> Pixels {
    Pixels::from((logical.as_f32() * scale).round().max(least) / scale)
}

/// The width a caret paints at: whole device pixels, at least one.
pub(crate) fn caret_width(width: Pixels, scale: f32) -> Pixels {
    snap(width, 1.0, scale)
}

/// A caret bar at `origin`, `width` wide, snapped whole to device pixels so it keeps one width.
pub(crate) fn caret_bar(
    origin: Point<Pixels>,
    width: Pixels,
    height: Pixels,
    scale: f32,
) -> Bounds<Pixels> {
    Bounds::new(
        Point::new(snap(origin.x, f32::MIN, scale), origin.y),
        size(caret_width(width, scale), height),
    )
}

#[cfg(test)]
mod tests {
    use gpui::{point, px};

    use super::caret_bar;

    /// gpui's paint snapping: each edge alone, half toward zero.
    fn device(logical: f32, scale: f32) -> f32 {
        let value = logical * scale;
        (value.abs() - 0.5).ceil().copysign(value)
    }

    #[test]
    fn a_caret_keeps_one_device_width_wherever_it_stands() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let widths: Vec<f32> = (0..200)
                .map(|step| {
                    let bar = caret_bar(
                        point(px(10.0 + step as f32 * 0.037), px(3.3)),
                        px(1.5),
                        px(18.0),
                        scale,
                    );
                    device(bar.right().as_f32(), scale) - device(bar.left().as_f32(), scale)
                })
                .collect();
            let first = widths[0];
            assert!(first >= 1.0, "at {scale}x the caret shows");
            assert!(
                widths.iter().all(|width| *width == first),
                "at {scale}x the caret took widths {:?}",
                widths.iter().fold(Vec::new(), |mut seen, width| {
                    if !seen.contains(width) {
                        seen.push(*width);
                    }
                    seen
                })
            );
        }
    }
}
