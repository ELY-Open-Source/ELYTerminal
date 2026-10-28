use gpui::{Pixels, Point, TextRun, Window, point};

use super::Look;

/// The runs with one color, so shaping keys on text and font.
fn shaping_runs(runs: &[TextRun]) -> Vec<TextRun> {
    runs.iter()
        .cloned()
        .map(|mut run| {
            run.color = gpui::black();
            run
        })
        .collect()
}

/// Each stretch of neighboring runs in one color, with its text.
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

/// Shapes each color section apart; `force` sets every advance.
pub(super) fn draw(
    content: &str,
    runs: &[TextRun],
    top: Point<Pixels>,
    force: Option<Pixels>,
    look: &Look,
    window: &mut Window,
) {
    let baseline = top.y + look.baseline;
    let mut x = top.x;
    for (text, runs) in sections(content, runs) {
        let shaped = window.text_system().shape_line(
            text.to_string().into(),
            look.size,
            &shaping_runs(runs),
            force,
        );
        let color = runs[0].color;
        for run in &shaped.runs {
            for glyph in &run.glyphs {
                let at = point(x + glyph.position.x, baseline + upward(glyph.position.y));
                let painted = if glyph.is_emoji {
                    window.paint_emoji(at, run.font_id, glyph.id, look.size)
                } else {
                    window.paint_glyph(at, run.font_id, glyph.id, look.size, color)
                };
                if let Err(error) = painted {
                    log::error!("terminal: a glyph of `{content}` failed to paint: {error:#}");
                }
            }
        }
        x += force.map_or(shaped.width, |cell| cell * text.chars().count() as f32);
    }
}

/// CoreText gives glyph offsets growing upward; the screen grows downward.
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
