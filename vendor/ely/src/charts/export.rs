use std::fmt::Write;

use gpui::{App, Hsla, Rems, Rgba, SharedString};

use super::{
    geometry::{Geometry, Ink, Rect, frame},
    series::tangents,
};
use crate::theme::{ActiveTheme, Palette, Radius, TextSize};

/// An SVG page: its size, the plot's frame, and the theme's colors and measures at the base rem.
pub(crate) struct Sheet {
    pub size: (f32, f32),
    pub frame: Rect,
    stroke: f32,
    hairline: f32,
    corner: f32,
    text: f32,
    font: SharedString,
    colors: Palette,
    horizontal: bool,
    smooth: bool,
}

impl Sheet {
    pub(crate) fn new(size: (f32, f32), horizontal: bool, smooth: bool, cx: &App) -> Self {
        let theme = cx.theme();
        let (sizes, rem) = (theme.chart(), theme.base_rem());
        let pixels = |length: Rems| f32::from(length.to_pixels(rem));
        Self {
            size,
            frame: frame(
                size,
                pixels(sizes.gutter),
                pixels(sizes.foot),
                pixels(sizes.inset),
            ),
            stroke: pixels(sizes.stroke),
            hairline: f32::from(sizes.hairline),
            corner: pixels(theme.radius(Radius::Sm)),
            text: pixels(theme.text_size(TextSize::Xs)),
            font: theme.font_family.clone(),
            colors: theme.colors.clone(),
            horizontal,
            smooth,
        }
    }
}

/// A color as SVG writes it, and its opacity apart.
fn hex(color: Hsla) -> (String, f32) {
    let rgba: Rgba = color.into();
    let byte = |channel: f32| (channel.clamp(0.0, 1.0) * 255.0).round() as u8;
    (
        format!(
            "#{:02x}{:02x}{:02x}",
            byte(rgba.r),
            byte(rgba.g),
            byte(rgba.b)
        ),
        rgba.a,
    )
}

/// Text safe inside SVG.
fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// A path's commands through `points`, straight or smooth.
fn path(points: &[(f32, f32)], smooth: bool) -> String {
    let mut d = String::new();
    let Some((x, y)) = points.first() else {
        return d;
    };
    write!(d, "M{x:.1},{y:.1}").expect("a string takes writes");
    if !smooth || points.len() < 3 {
        for (x, y) in &points[1..] {
            write!(d, " L{x:.1},{y:.1}").expect("a string takes writes");
        }
        return d;
    }
    let slopes = tangents(points);
    for ix in 0..points.len() - 1 {
        let ((x0, y0), (x1, y1)) = (points[ix], points[ix + 1]);
        let third = (x1 - x0) / 3.0;
        let (ax, ay, bx, by) = (
            x0 + third,
            y0 + slopes[ix] * third,
            x1 - third,
            y1 - slopes[ix + 1] * third,
        );
        write!(d, " C{ax:.1},{ay:.1} {bx:.1},{by:.1} {x1:.1},{y1:.1}")
            .expect("a string takes writes");
    }
    d
}

/// A chart as a standalone SVG drawing: gridlines, marks and axis labels.
pub(crate) fn svg(geometry: &Geometry, sheet: &Sheet, format: &dyn Fn(f64) -> String) -> String {
    let (colors, frame, (width, height)) = (&sheet.colors, sheet.frame, sheet.size);
    let ink = |ink: Ink| hex(ink.color(colors)).0;
    let (grid, _) = hex(colors.border);
    let (label, _) = hex(colors.fg_subtle);
    let (back, _) = hex(colors.bg);
    let (gap, drop) = (sheet.text * 0.75, sheet.text * 1.6);
    let (below, beside) = (frame.y + frame.h + drop, frame.x - gap);
    let mut out = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" font-family="{}, sans-serif" font-size="{}">"#,
        escaped(&sheet.font),
        sheet.text
    );
    let mut put = |text: String| out.push_str(&text);
    put(format!(
        r#"<rect width="{width}" height="{height}" fill="{back}"/>"#
    ));
    let text = |x: f32, y: f32, anchor: &str, words: &str| {
        format!(
            r#"<text x="{x:.1}" y="{y:.1}" fill="{label}" text-anchor="{anchor}">{}</text>"#,
            escaped(words)
        )
    };
    let middle = sheet.text * 0.35;
    for (at, value) in &geometry.ticks {
        let words = format(*value);
        if sheet.horizontal {
            put(format!(
                r#"<line x1="{at:.1}" y1="{:.1}" x2="{at:.1}" y2="{:.1}" stroke="{grid}" stroke-width="{}"/>"#,
                frame.y,
                frame.y + frame.h,
                sheet.hairline
            ));
            put(text(*at, below, "middle", &words));
        } else {
            put(format!(
                r#"<line x1="{:.1}" y1="{at:.1}" x2="{:.1}" y2="{at:.1}" stroke="{grid}" stroke-width="{}"/>"#,
                frame.x,
                frame.x + frame.w,
                sheet.hairline
            ));
            put(text(beside, at + middle, "end", &words));
        }
    }
    for (at, words) in &geometry.labels {
        put(if sheet.horizontal {
            text(beside, at + middle, "end", words)
        } else {
            text(*at, below, "middle", words)
        });
    }
    for line in &geometry.lines {
        if let Some(floor) = &line.floor {
            let back: String = floor
                .iter()
                .rev()
                .map(|(x, y)| format!(" L{x:.1},{y:.1}"))
                .collect();
            put(format!(
                r#"<path d="{}{back} Z" fill="{}" fill-opacity="0.14"/>"#,
                path(&line.points, sheet.smooth),
                ink(line.ink)
            ));
        }
        put(format!(
            r#"<path d="{}" fill="none" stroke="{}" stroke-width="{}" stroke-linejoin="round"/>"#,
            path(&line.points, sheet.smooth),
            ink(line.ink),
            sheet.stroke
        ));
    }
    for (color, rect) in &geometry.bars {
        let radius = sheet.corner.min(rect.w.min(rect.h) / 2.0);
        put(format!(
            r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="{radius:.1}" fill="{}"/>"#,
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            ink(*color)
        ));
    }
    for (color, outline) in &geometry.shapes {
        put(format!(
            r#"<path d="{} Z" fill="{fill}" fill-opacity="0.3" stroke="{fill}" stroke-width="{}"/>"#,
            path(outline, false),
            sheet.hairline,
            fill = ink(*color)
        ));
    }
    for (color, (x1, y1), (x2, y2)) in &geometry.strokes {
        let width = if *color == Ink::Rule {
            sheet.hairline
        } else {
            sheet.stroke
        };
        put(format!(
            r#"<line x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}" stroke="{}" stroke-width="{width}"/>"#,
            ink(*color)
        ));
    }
    for dot in &geometry.dots {
        let (x, y) = dot.center;
        put(format!(
            r#"<circle cx="{x:.1}" cy="{y:.1}" r="{:.1}" fill="{fill}" fill-opacity="0.6" stroke="{fill}" stroke-width="{}"/>"#,
            dot.radius,
            sheet.hairline,
            fill = ink(dot.ink)
        ));
    }
    out.push_str("</svg>");
    out
}

#[cfg(test)]
mod tests {
    use super::{escaped, path};

    #[test]
    fn paths_and_text_write_as_svg_reads_them() {
        assert_eq!(
            path(&[(0.0, 10.0), (5.0, 2.5)], false),
            "M0.0,10.0 L5.0,2.5"
        );
        assert!(path(&[(0.0, 0.0), (1.0, 2.0), (2.0, 0.0)], true).contains(" C"));
        assert_eq!(escaped("R&D <net>"), "R&amp;D &lt;net&gt;");
    }
}
