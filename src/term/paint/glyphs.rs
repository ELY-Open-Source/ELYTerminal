use gpui::{Pixels, Point, TextRun, Window, point};

use super::Look;

/// Runs made black, so shaping caches on text and font.
fn shaping_runs(runs: &[TextRun]) -> Vec<TextRun> {
    runs.iter()
        .cloned()
        .map(|mut run| {
            run.color = gpui::black();
            run
        })
        .collect()
}

/// Neighboring runs of one color, each with its text.
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

/// Each character's column, at its first byte.
fn columns(content: &str) -> Vec<usize> {
    let mut map = vec![0; content.len()];
    for (column, (byte, _)) in content.char_indices().enumerate() {
        map[byte] = column;
    }
    map
}

/// Each glyph's x: its character's column, keeping logical order.
fn places(
    columns: &[usize],
    start: usize,
    glyphs: &[(usize, Pixels)],
    cell: Pixels,
) -> Vec<Pixels> {
    let mut first: Option<(usize, Pixels)> = None;
    glyphs
        .iter()
        .map(|&(index, x)| {
            let base = match first {
                Some((at, base)) if at == index => base,
                _ => first.insert((index, x)).1,
            };
            cell * columns[start + index] as f32 + (x - base)
        })
        .collect()
}

/// Shapes each color section apart; `force` puts glyphs in cells.
pub(super) fn draw(
    content: &str,
    runs: &[TextRun],
    top: Point<Pixels>,
    force: Option<Pixels>,
    look: &Look,
    window: &mut Window,
) {
    let baseline = top.y + look.baseline;
    let columns = columns(content);
    let (mut start, mut pen) = (0, top.x);
    for (text, runs) in sections(content, runs) {
        let shaped = window.text_system().shape_line(
            text.to_string().into(),
            look.size,
            &shaping_runs(runs),
            force,
        );
        let color = runs[0].color;
        let glyphs: Vec<_> = shaped
            .runs
            .iter()
            .flat_map(|run| run.glyphs.iter().map(move |glyph| (run.font_id, glyph)))
            .collect();
        let xs: Vec<_> = match force {
            Some(cell) => {
                let at: Vec<_> = glyphs
                    .iter()
                    .map(|(_, g)| (g.index, g.position.x))
                    .collect();
                let xs = places(&columns, start, &at, cell);
                xs.into_iter().map(|x| top.x + x).collect()
            }
            None => glyphs.iter().map(|(_, g)| pen + g.position.x).collect(),
        };
        for ((font_id, glyph), x) in glyphs.iter().zip(xs) {
            let at = point(x, baseline + upward(glyph.position.y));
            let painted = if glyph.is_emoji {
                window.paint_emoji(at, *font_id, glyph.id, look.size)
            } else {
                window.paint_glyph(at, *font_id, glyph.id, look.size, color)
            };
            if let Err(error) = painted {
                log::error!("terminal: a glyph of `{content}` failed to paint: {error:#}");
            }
        }
        start += text.len();
        pen += shaped.width;
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
