mod glyphs;

use std::{rc::Rc, sync::Arc};

use glyphs::draw;

use alacritty_terminal::vte::ansi::CursorShape;
use gpui::{
    Bounds, ContentMask, Corners, ElementInputHandler, Entity, Font, FontId, FontStyle, FontWeight,
    Hsla, IntoElement, PathBuilder, Pixels, Point, RenderImage, Size, StrikethroughStyle, Styled,
    TextRun, UnderlineStyle, Window, canvas, fill, outline, point, px, size,
};

use super::{
    boxes::{self, Shape},
    frame::{Cursor, Frame, Style, Underline},
    graphics::{Layer, Shown},
    view::TerminalView,
};

/// How the grid paints: font, cell, metrics and the cursor's colors.
#[derive(Clone)]
pub struct Look {
    pub font: Font,
    pub font_id: FontId,
    /// Draws icons the font lacks.
    pub icons: Font,
    pub size: Pixels,
    pub cell: Size<Pixels>,
    /// From a cell's top to the text's baseline.
    pub baseline: Pixels,
    pub descent: Pixels,
    pub rule: Pixels,
    pub cursor: Hsla,
    pub cursor_text: Hsla,
    /// The terminal's own text and ground, for text an input method composes.
    pub fg: Hsla,
    pub bg: Hsla,
}

/// The grid fitted to its box: it sizes the pty, then paints each row, the cursor and composed text.
pub fn grid(
    view: Entity<TerminalView>,
    frame: Rc<Frame>,
    cursor: Option<(Cursor, CursorShape)>,
    preedit: Option<String>,
    look: Look,
    (pictures, stale): (Vec<Shown>, Vec<Arc<RenderImage>>),
) -> impl IntoElement {
    let cell = look.cell;
    let (fit, input) = (view.clone(), view);
    let at = frame.caret;
    canvas(
        move |bounds, window, cx| {
            let columns = (bounds.size.width / cell.width).floor() as usize;
            let lines = (bounds.size.height / cell.height).floor() as usize;
            let origin = snapped(bounds.origin, window);
            fit.update(cx, |view, cx| {
                (view.pointer.origin, view.pointer.cell) = (origin, cell);
                if let Some((row, column)) = at {
                    let corner =
                        origin + point(cell.width * column as f32, cell.height * row as f32);
                    let caret = Bounds::new(corner, cell);
                    if caret != view.caret {
                        log::trace!("terminal {}: the caret moves to {caret:?}", view.pty.pid);
                        view.caret = caret;
                    }
                }
                if view.fit(columns, lines, cx) {
                    cx.notify();
                    window.request_animation_frame();
                }
            });
        },
        move |bounds, _, window, cx| {
            log::trace!(
                "terminal: paint {:?}, row 0 {:?}",
                bounds,
                frame.rows.first().map(|row| row.text.trim_end())
            );
            let focus = input.read(cx).focus.clone();
            window.handle_input(&focus, ElementInputHandler::new(bounds, input.clone()), cx);
            let origin = snapped(bounds.origin, window);
            drop_pictures(stale, window);
            let top = |index: usize| origin + point(px(0.), cell.height * index as f32);
            paint_pictures(&pictures, Layer::Under, origin, cell, window);
            for (index, row) in frame.rows.iter().enumerate() {
                paint_grounds(row, top(index), &look, window);
            }
            paint_pictures(&pictures, Layer::Between, origin, cell, window);
            for (index, row) in frame.rows.iter().enumerate() {
                paint_text(row, top(index), &look, window);
            }
            paint_pictures(&pictures, Layer::Over, origin, cell, window);
            match (&preedit, at, &cursor) {
                (Some(text), Some(at), _) => paint_preedit(text, at, origin, &look, window),
                (None, _, Some((cursor, shape))) => {
                    paint_cursor(cursor, *shape, origin, &look, window)
                }
                _ => {}
            }
        },
    )
    .size_full()
}

/// `origin` on a whole device pixel.
fn snapped(origin: Point<Pixels>, window: &Window) -> Point<Pixels> {
    let scale = window.scale_factor();
    let snap = |length: Pixels| px((f32::from(length) * scale).round() / scale);
    point(snap(origin.x), snap(origin.y))
}

/// Text an input method composes, over the cursor's cell on the terminal's ground, underlined.
fn paint_preedit(
    text: &str,
    (row, column): (usize, usize),
    origin: Point<Pixels>,
    look: &Look,
    window: &mut Window,
) {
    // gpui shapes no newline; a composition may hold one.
    let text = &crate::sessions::one_line(text);
    let cell = look.cell;
    let corner = origin + point(cell.width * column as f32, cell.height * row as f32);
    let style = Style {
        fg: look.fg,
        bg: None,
        bold: false,
        italic: false,
        underline: Underline::Single,
        underline_color: look.fg,
        strike: false,
    };
    let runs = runs(text, &style, look.fg, look, window);
    let width = window
        .text_system()
        .shape_line(text.to_string().into(), look.size, &runs, None)
        .width
        .max(cell.width);
    window.paint_quad(fill(Bounds::new(corner, size(width, cell.height)), look.bg));
    draw(text, &runs, corner, None, look, window);
    decorate(corner, width, &style, look, window);
}

fn face(style: &Style, base: &Font) -> Font {
    Font {
        weight: if style.bold {
            FontWeight::BOLD
        } else {
            FontWeight::NORMAL
        },
        style: if style.italic {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        },
        ..base.clone()
    }
}

/// Each run's style with the columns it covers.
fn spans(row: &super::frame::Row) -> Vec<(usize, usize, Style)> {
    let (mut byte, mut column) = (0, 0);
    row.runs
        .iter()
        .map(|(len, style)| {
            let columns = row.text[byte..byte + len].chars().count();
            let span = (column, columns, *style);
            byte += len;
            column += columns;
            span
        })
        .collect()
}

/// Takes textures out of the window's atlas.
pub fn drop_pictures(pictures: Vec<Arc<RenderImage>>, window: &mut Window) {
    if !pictures.is_empty() {
        log::debug!("terminal: {} textures dropped", pictures.len());
    }
    for picture in pictures {
        if let Err(error) = window.drop_image(picture) {
            log::error!("terminal: a texture did not drop: {error:#}");
        }
    }
}

/// Kitty graphics placements of `layer`, cells measured from `origin`.
fn paint_pictures(
    pictures: &[Shown],
    layer: Layer,
    origin: Point<Pixels>,
    cell: Size<Pixels>,
    window: &mut Window,
) {
    let at = |cells: Bounds<f32>| {
        Bounds::new(
            origin + point(cell.width * cells.origin.x, cell.height * cells.origin.y),
            size(
                cell.width * cells.size.width,
                cell.height * cells.size.height,
            ),
        )
    };
    for shown in pictures.iter().filter(|shown| shown.layer == layer) {
        let (clip, whole) = (at(shown.clip), at(shown.whole));
        let picture = Arc::clone(&shown.picture);
        // A mask clips it; gpui's clip rounds to source pixels.
        let mask = Some(ContentMask { bounds: clip });
        let painted = window.with_content_mask(mask, |window| {
            window.paint_image(whole, whole, Corners::default(), picture, 0, false)
        });
        if let Err(error) = painted {
            log::error!("terminal: an image did not paint: {error:#}");
        }
    }
}

/// A row's grounds, under every row's text.
fn paint_grounds(row: &super::frame::Row, top: Point<Pixels>, look: &Look, window: &mut Window) {
    let cell = look.cell;
    for (column, columns, style) in spans(row) {
        if let Some(bg) = style.bg {
            let corner = top + point(cell.width * column as f32, px(0.));
            let width = cell.width * columns as f32;
            window.paint_quad(fill(Bounds::new(corner, size(width, cell.height)), bg));
        }
    }
}

/// A row's text and decorations.
fn paint_text(row: &super::frame::Row, top: Point<Pixels>, look: &Look, window: &mut Window) {
    let cell = look.cell;
    let at = |column: usize| top + point(cell.width * column as f32, px(0.));
    let spans = spans(row);
    let mut byte = 0;
    let runs: Vec<TextRun> = row
        .runs
        .iter()
        .flat_map(|(len, style)| {
            let text = &row.text[byte..byte + len];
            byte += len;
            runs(text, style, style.fg, look, window)
        })
        .collect();
    draw(&row.text, &runs, top, Some(cell.width), look, window);
    for (column, content, style) in &row.alone {
        paint_alone(content, style.fg, style, at(*column), look, window);
    }
    for (column, columns, style) in spans {
        let width = cell.width * columns as f32;
        decorate(at(column), width, &style, look, window);
    }
}

/// Text runs for `text` in `style` and `color`: icons the font lacks take the icon font.
fn runs(text: &str, style: &Style, color: Hsla, look: &Look, window: &Window) -> Vec<TextRun> {
    let base = face(style, &look.font);
    let system = window.text_system();
    let mut runs: Vec<TextRun> = Vec::new();
    for ch in text.chars() {
        let missing = private(ch) && system.advance(look.font_id, look.size, ch).is_err();
        let font = if missing { &look.icons } else { &base };
        match runs.last_mut() {
            Some(last) if last.font == *font => last.len += ch.len_utf8(),
            _ => runs.push(TextRun {
                len: ch.len_utf8(),
                font: font.clone(),
                color,
                background_color: None,
                underline: None,
                strikethrough: None,
            }),
        }
    }
    runs
}

/// Unicode's private use areas, where Nerd Font icons live.
fn private(ch: char) -> bool {
    matches!(ch, '\u{E000}'..='\u{F8FF}' | '\u{F0000}'..='\u{FFFFD}' | '\u{100000}'..='\u{10FFFD}')
}

/// Paints `content` alone at `origin`, a cell's top-left, its marks where shaping puts them.
fn text(
    content: &str,
    color: Hsla,
    style: &Style,
    origin: Point<Pixels>,
    look: &Look,
    window: &mut Window,
) {
    let runs = runs(content, style, color, look, window);
    draw(content, &runs, origin, None, look, window);
}

/// Paints a cell's text alone at `corner`: a drawn base as shapes, its marks over a no-break space.
fn paint_alone(
    content: &str,
    color: Hsla,
    style: &Style,
    corner: Point<Pixels>,
    look: &Look,
    window: &mut Window,
) {
    let mut chars = content.chars();
    match chars.next() {
        Some(base) if boxes::drawn(base) => {
            paint_drawn(base, color, corner, look, window);
            let marks = chars.as_str();
            if !marks.is_empty() {
                let shown = format!("\u{A0}{marks}");
                text(&shown, color, style, corner, look, window);
            }
        }
        _ => text(content, color, style, corner, look, window),
    }
}

/// Paints `ch` as shapes in `color`, clipped to the cell at `corner`, a device pixel corner.
fn paint_drawn(ch: char, color: Hsla, corner: Point<Pixels>, look: &Look, window: &mut Window) {
    let scale = window.scale_factor();
    let device = |length: Pixels| (f32::from(length) * scale).round() as u32;
    let (w, h, t) = (
        device(look.cell.width),
        device(look.cell.height),
        device(look.rule).max(1),
    );
    let at = |(x, y): (f32, f32)| corner + point(px(x / scale), px(y / scale));
    let mask = ContentMask {
        bounds: Bounds::new(corner, look.cell),
    };
    window.with_content_mask(Some(mask), |window| {
        for piece in boxes::shapes(ch, w, h, t) {
            match piece {
                Shape::Rect { x, y, w, h, alpha } => {
                    let length = |device: u32| px(device as f32 / scale);
                    let bounds = Bounds::new(at((x as f32, y as f32)), size(length(w), length(h)));
                    window.paint_quad(fill(bounds, color.opacity(alpha)));
                }
                Shape::Fill(points) => {
                    let mut builder = PathBuilder::fill();
                    builder.move_to(at(points[0]));
                    for &corner in &points[1..] {
                        builder.line_to(at(corner));
                    }
                    builder.close();
                    match builder.build() {
                        Ok(path) => window.paint_path(path, color),
                        Err(error) => log::error!(
                            "terminal: U+{:04X} failed to tessellate: {error:?}",
                            ch as u32
                        ),
                    }
                }
            }
        }
    });
}

/// Underlines and strikethrough under `width` from `origin`, a cell's top-left.
fn decorate(origin: Point<Pixels>, width: Pixels, style: &Style, look: &Look, window: &mut Window) {
    let rule = look.rule;
    let line = origin.y + look.baseline + (look.descent * 0.45).max(rule);
    let bar = |x: Pixels, y: Pixels, w: Pixels| {
        fill(
            Bounds::new(point(x, y), size(w, rule)),
            style.underline_color,
        )
    };
    match style.underline {
        Underline::None => {}
        Underline::Single => window.paint_quad(bar(origin.x, line, width)),
        Underline::Double => {
            window.paint_quad(bar(origin.x, line - rule, width));
            window.paint_quad(bar(origin.x, line + rule, width));
        }
        Underline::Curly => window.paint_underline(
            point(origin.x, line - rule),
            width,
            &UnderlineStyle {
                thickness: rule,
                color: Some(style.underline_color),
                wavy: true,
            },
        ),
        Underline::Dotted => {
            let mut x = origin.x;
            while x < origin.x + width {
                window.paint_quad(bar(x, line, rule));
                x += rule * 2.;
            }
        }
        Underline::Dashed => {
            let dash = look.cell.width * 0.5;
            let mut x = origin.x;
            while x < origin.x + width {
                window.paint_quad(bar(x, line, dash.min(origin.x + width - x)));
                x += dash * 2.;
            }
        }
    }
    if style.strike {
        let middle = origin.y + look.baseline - look.size * 0.3;
        window.paint_strikethrough(
            point(origin.x, middle),
            width,
            &StrikethroughStyle {
                thickness: rule,
                color: Some(style.fg),
            },
        );
    }
}

fn paint_cursor(
    cursor: &Cursor,
    shape: CursorShape,
    origin: Point<Pixels>,
    look: &Look,
    window: &mut Window,
) {
    let cell = look.cell;
    let corner = origin
        + point(
            cell.width * cursor.column as f32,
            cell.height * cursor.row as f32,
        );
    let width = cell.width * if cursor.wide { 2. } else { 1. };
    let whole = Bounds::new(corner, size(width, cell.height));
    match shape {
        CursorShape::Block => {
            window.paint_quad(fill(whole, look.cursor));
            paint_alone(
                &cursor.text,
                look.cursor_text,
                &cursor.style,
                corner,
                look,
                window,
            );
        }
        CursorShape::HollowBlock => {
            window.paint_quad(outline(whole, look.cursor, gpui::BorderStyle::Solid));
        }
        CursorShape::Beam => {
            let bar = Bounds::new(corner, size((look.rule * 2.).max(px(1.)), cell.height));
            window.paint_quad(fill(bar, look.cursor));
        }
        CursorShape::Underline => {
            let height = look.rule * 2.;
            let bar = Bounds::new(
                corner + point(px(0.), cell.height - height),
                size(width, height),
            );
            window.paint_quad(fill(bar, look.cursor));
        }
        CursorShape::Hidden => {}
    }
}

#[cfg(test)]
mod tests;
