use gpui::{Bounds, ContentMask, Pixels, Point, TextRun, Window, point, px};

use super::Look;

fn shaping_runs(runs: &[TextRun]) -> Vec<TextRun> {
    runs.iter()
        .cloned()
        .map(|mut run| {
            run.color = gpui::black();
            run
        })
        .collect()
}

fn sections<'a>(
    content: &'a str,
    runs: &'a [TextRun],
) -> impl Iterator<Item = (&'a str, &'a [TextRun])> {
    runs.chunk_by(|a, b| a.color == b.color)
        .scan(0, |byte, runs| {
            let end = *byte + runs.iter().map(|run| run.len).sum::<usize>();
            let text = &content[*byte..end];
            *byte = end;
            Some((text, runs))
        })
}

fn mask(
    outer: Bounds<Pixels>,
    left: Option<Pixels>,
    right: Option<Pixels>,
    scale: f32,
) -> ContentMask<Pixels> {
    let snap = |edge: Pixels| {
        px((f32::from(edge) * scale).round() / scale)
            .max(outer.left())
            .min(outer.right())
    };
    ContentMask {
        bounds: Bounds::from_corners(
            point(left.map_or(outer.left(), snap), outer.top()),
            point(right.map_or(outer.right(), snap), outer.bottom()),
        ),
    }
}

pub(super) fn draw(
    content: &str,
    runs: &[TextRun],
    top: Point<Pixels>,
    force: Option<Pixels>,
    look: &Look,
    window: &mut Window,
) {
    let outer = window.content_mask().bounds;
    let scale = window.scale_factor();
    let (mut byte, mut column, mut offset) = (0, 0, px(0.));
    for (text, runs) in sections(content, runs) {
        // Shape color sections separately instead of making CoreText break a line's many runs.
        let shaped = window.text_system().shape_line(
            text.to_string().into(),
            look.size,
            &shaping_runs(runs),
            force,
        );
        let origin = top + point(offset, px(0.));
        column += text.chars().count();
        let end = force.map_or(origin.x + shaped.width, |cell| top.x + cell * column as f32);
        let left = (byte > 0).then_some(origin.x);
        byte += text.len();
        let right = (byte < content.len()).then_some(end);
        let clip = mask(outer, left, right, scale);
        window.with_content_mask(Some(clip), |window| {
            let baseline = origin.y + look.baseline;
            for run in &shaped.runs {
                for glyph in &run.glyphs {
                    let at = point(
                        origin.x + glyph.position.x,
                        baseline + upward(glyph.position.y),
                    );
                    let painted = if glyph.is_emoji {
                        window.paint_emoji(at, run.font_id, glyph.id, look.size)
                    } else {
                        window.paint_glyph(at, run.font_id, glyph.id, look.size, runs[0].color)
                    };
                    if let Err(error) = painted {
                        log::error!("terminal: a glyph of `{content}` failed to paint: {error:#}");
                    }
                }
            }
        });
        offset = end - top.x;
    }
}

#[cfg(target_os = "macos")]
fn upward(y: Pixels) -> Pixels {
    -y
}

#[cfg(not(target_os = "macos"))]
fn upward(y: Pixels) -> Pixels {
    y
}

#[cfg(test)]
mod tests;
