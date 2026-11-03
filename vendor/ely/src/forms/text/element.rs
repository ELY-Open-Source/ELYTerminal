use std::rc::Rc;

use gpui::{
    App, AvailableSpace, Bounds, ContentMask, DispatchPhase, Element, ElementId,
    ElementInputHandler, Entity, FontStyle, GlobalElementId, Hsla, InspectorElementId, IntoElement,
    LayoutId, MouseMoveEvent, Pixels, Point, SharedString, StrikethroughStyle, Style, TextAlign,
    TextRun, UnderlineStyle, Window, WrappedLine, fill, point, relative, size,
};

use super::{Highlight, Layout, TextInput};
use crate::{
    primitives::{caret_bar, caret_width},
    theme::ActiveTheme,
};

pub(crate) struct TextElement {
    input: Entity<TextInput>,
}

impl TextElement {
    pub fn new(input: Entity<TextInput>) -> Self {
        Self { input }
    }
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

pub(crate) struct Prepaint {
    lines: Rc<[(usize, WrappedLine)]>,
    selections: Vec<Bounds<Pixels>>,
    caret: Option<Bounds<Pixels>>,
    selection_color: Hsla,
    caret_color: Hsla,
}

/// What to show, and whether it is the placeholder.
fn shown(input: &TextInput) -> (String, bool) {
    if input.is_empty() {
        (input.placeholder_text().to_string(), true)
    } else {
        (input.display_text(), false)
    }
}

fn plain_run(len: usize, window: &Window, color: Hsla) -> TextRun {
    TextRun {
        len,
        font: window.text_style().font(),
        color,
        background_color: None,
        underline: None,
        strikethrough: None,
    }
}

/// Splits `len` bytes into runs: highlights, and the underlined marked range, over a base run.
fn runs(
    len: usize,
    base: TextRun,
    spans: Vec<(std::ops::Range<usize>, Highlight, bool)>,
    thickness: Pixels,
) -> Vec<TextRun> {
    let mut cuts: Vec<usize> = vec![0, len];
    for (range, ..) in &spans {
        cuts.extend([range.start.min(len), range.end.min(len)]);
    }
    cuts.sort_unstable();
    cuts.dedup();
    cuts.windows(2)
        .map(|pair| {
            let (start, end) = (pair[0], pair[1]);
            let mut run = TextRun {
                len: end - start,
                ..base.clone()
            };
            for (range, highlight, underline) in &spans {
                if range.start <= start && end <= range.end {
                    run.color = highlight.color;
                    if highlight.background.is_some() {
                        run.background_color = highlight.background;
                    }
                    if let Some(weight) = highlight.weight {
                        run.font.weight = weight;
                    }
                    if highlight.italic {
                        run.font.style = FontStyle::Italic;
                    }
                    if highlight.strike {
                        run.strikethrough = Some(StrikethroughStyle {
                            thickness,
                            color: Some(highlight.color),
                        });
                    }
                    if *underline {
                        run.underline = Some(UnderlineStyle {
                            color: Some(highlight.color),
                            thickness,
                            wavy: false,
                        });
                    }
                }
            }
            run
        })
        .filter(|run| run.len > 0)
        .collect()
}

/// What to show, whether it is the placeholder, and its runs, the same for measure and paint.
fn shown_runs(input: &TextInput, window: &Window, cx: &App) -> (String, bool, Vec<TextRun>) {
    let theme = cx.theme();
    let (text, placeholder) = shown(input);
    let color = window.text_style().color;
    let base = plain_run(
        text.len(),
        window,
        if placeholder {
            theme.colors.fg_subtle
        } else {
            color
        },
    );
    let mut spans = Vec::new();
    if !placeholder {
        for (range, highlight) in input.highlights(cx) {
            spans.push((range, highlight, false));
        }
        if let Some(marked) = input.marked() {
            let marked = input.display_offset(marked.start)..input.display_offset(marked.end);
            spans.push((marked, Highlight::new(color), true));
        }
    }
    let runs = runs(text.len(), base, spans, theme.underline_thickness());
    (text, placeholder, runs)
}

/// Starts of the hard lines in `text`.
fn line_starts(text: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(text.match_indices('\n').map(|(ix, _)| ix + 1))
        .collect()
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = Prepaint;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let input = self.input.read(cx);
        let line_height = window.line_height();
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        let Some((min, max)) = input.rows() else {
            style.size.height = line_height.into();
            return (window.request_layout(style, [], cx), ());
        };
        let (text, _, runs) = shown_runs(input, window, cx);
        let font_size = window.text_style().font_size.to_pixels(window.rem_size());
        let layout = window.request_measured_layout(style, move |known, available, window, _| {
            let width = known.width.or(match available.width {
                AvailableSpace::Definite(width) => Some(width),
                _ => None,
            });
            let shaped = window.text_system().shape_text(
                SharedString::from(text.clone()),
                font_size,
                &runs,
                width,
                None,
            );
            let rows: usize = match shaped {
                Ok(lines) => lines
                    .iter()
                    .map(|line| line.wrap_boundaries().len() + 1)
                    .sum(),
                Err(error) => {
                    log::error!("text input: shaping failed: {error:#}");
                    1
                }
            };
            size(
                width.unwrap_or_default(),
                line_height * rows.clamp(min, max) as f32,
            )
        });
        (layout, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Prepaint {
        let theme = cx.theme();
        let (selection_color, caret_color) = (theme.colors.selection, theme.colors.focus);
        let caret_width = caret_width(
            theme.caret_width().to_pixels(window.rem_size()),
            window.scale_factor(),
        );
        let input = self.input.read(cx);
        let (text, placeholder, runs) = shown_runs(input, window, cx);
        let font_size = window.text_style().font_size.to_pixels(window.rem_size());
        let line_height = window.line_height();
        let wrap = input.rows().map(|_| bounds.size.width);
        let shaped = window
            .text_system()
            .shape_text(text.clone().into(), font_size, &runs, wrap, None)
            .unwrap_or_else(|error| {
                log::error!("text input: shaping failed: {error:#}");
                Default::default()
            });
        let lines: Rc<[(usize, WrappedLine)]> =
            line_starts(&text).into_iter().zip(shaped).collect();
        let focused = input.focus().is_focused(window);
        let caret_on = input.caret_on();
        self.input.update(cx, |input, _| {
            input.layout = Some(Layout {
                lines: lines.clone(),
                bounds,
                line_height,
                placeholder,
            });
        });
        let input = self.input.read(cx);
        let caret_at = input.position_for(input.display_offset(input.cursor()));
        let scroll = match caret_at {
            // Unfocused, a line shows its start, as AppKit's fields do.
            Some(_) if !focused && !input.is_multi_line() => Point::default(),
            Some(at) => keep_in_view(input, &lines, at, caret_width, bounds, line_height),
            None => input.scroll,
        };
        let selection = input.selection();
        let selections = if placeholder || selection.is_empty() {
            Vec::new()
        } else {
            let (start, end) = (
                input.display_offset(selection.start),
                input.display_offset(selection.end),
            );
            selection_rects(&lines, start, end, line_height, bounds.size.width)
                .into_iter()
                .map(|rect| Bounds::new(rect.origin + bounds.origin - scroll, rect.size))
                .collect()
        };
        let caret = caret_at
            .filter(|_| focused && caret_on && selection.is_empty())
            .map(|at| {
                let at = bounds.origin + at - scroll;
                caret_bar(at, caret_width, line_height, window.scale_factor())
            });
        self.input.update(cx, |input, _| input.scroll = scroll);
        Prepaint {
            lines,
            selections,
            caret,
            selection_color,
            caret_color,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        prepaint: &mut Prepaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        let input = self.input.read(cx);
        let (focus, scroll, selecting) = (input.focus().clone(), input.scroll, input.selecting());
        window.handle_input(
            &focus,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        let line_height = window.line_height();
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            let mut origins = Vec::with_capacity(prepaint.lines.len());
            let mut top = Pixels::ZERO;
            for (_, line) in prepaint.lines.iter() {
                origins.push(bounds.origin - scroll + point(Pixels::ZERO, top));
                top += line.size(line_height).height;
            }
            for ((_, line), origin) in prepaint.lines.iter().zip(&origins) {
                let washed =
                    line.paint_background(*origin, line_height, TextAlign::Left, None, window, cx);
                if let Err(error) = washed {
                    log::error!("text input: wash paint failed: {error:#}");
                }
            }
            for rect in prepaint.selections.drain(..) {
                window.paint_quad(fill(rect, prepaint.selection_color));
            }
            for ((_, line), origin) in prepaint.lines.iter().zip(&origins) {
                let painted = line.paint(*origin, line_height, TextAlign::Left, None, window, cx);
                if let Err(error) = painted {
                    log::error!("text input: paint failed: {error:#}");
                }
            }
            if let Some(caret) = prepaint.caret.take() {
                window.paint_quad(fill(caret, prepaint.caret_color));
            }
        });
        if selecting {
            let input = self.input.clone();
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                if phase == DispatchPhase::Bubble && event.dragging() {
                    input.update(cx, |input, cx| {
                        let at = input.offset_for_point(event.position);
                        input.select_to(at, cx);
                    });
                }
            });
        }
    }
}

/// Scroll that keeps the caret inside the box, clamped to the text.
fn keep_in_view(
    input: &TextInput,
    lines: &[(usize, WrappedLine)],
    caret: Point<Pixels>,
    caret_width: Pixels,
    bounds: Bounds<Pixels>,
    line_height: Pixels,
) -> Point<Pixels> {
    let mut scroll = input.scroll;
    if input.is_multi_line() {
        let content = lines.iter().fold(Pixels::ZERO, |sum, (_, line)| {
            sum + line.size(line_height).height
        });
        let view = bounds.size.height;
        if caret.y < scroll.y {
            scroll.y = caret.y;
        } else if caret.y + line_height > scroll.y + view {
            scroll.y = caret.y + line_height - view;
        }
        scroll.y = scroll
            .y
            .clamp(Pixels::ZERO, (content - view).max(Pixels::ZERO));
        scroll.x = Pixels::ZERO;
    } else {
        let content = lines.first().map_or(Pixels::ZERO, |(_, line)| line.width()) + caret_width;
        let view = bounds.size.width;
        if caret.x < scroll.x {
            scroll.x = caret.x;
        } else if caret.x + caret_width > scroll.x + view {
            scroll.x = caret.x + caret_width - view;
        }
        scroll.x = scroll
            .x
            .clamp(Pixels::ZERO, (content - view).max(Pixels::ZERO));
        scroll.y = Pixels::ZERO;
    }
    scroll
}

/// Boxes covering display offsets `start..end`, relative to the text origin.
pub(super) fn selection_rects(
    lines: &[(usize, WrappedLine)],
    start: usize,
    end: usize,
    line_height: Pixels,
    width: Pixels,
) -> Vec<Bounds<Pixels>> {
    let mut rects = Vec::new();
    let mut top = Pixels::ZERO;
    for (line_start, line) in lines {
        let line_end = line_start + line.len();
        let (from, to) = (start.max(*line_start), end.min(line_end));
        if from <= to && start <= line_end && end >= *line_start {
            let at = |ix: usize| {
                line.position_for_index(ix - line_start, line_height)
                    .unwrap_or_default()
            };
            let (a, b) = (at(from), at(to));
            let edge = if end > line_end { width } else { b.x };
            if a.y == b.y {
                rects.push(Bounds::from_corners(
                    point(a.x, top + a.y),
                    point(edge.max(a.x), top + a.y + line_height),
                ));
            } else {
                rects.push(Bounds::from_corners(
                    point(a.x, top + a.y),
                    point(width, top + a.y + line_height),
                ));
                let mut y = a.y + line_height;
                while y < b.y {
                    rects.push(Bounds::from_corners(
                        point(Pixels::ZERO, top + y),
                        point(width, top + y + line_height),
                    ));
                    y += line_height;
                }
                rects.push(Bounds::from_corners(
                    point(Pixels::ZERO, top + b.y),
                    point(edge, top + b.y + line_height),
                ));
            }
        }
        top += line.size(line_height).height;
    }
    rects
}

#[cfg(test)]
mod tests {
    use gpui::{FontWeight, font, hsla, px};

    use super::*;

    #[test]
    fn a_highlight_sets_weight_slant_and_strike_on_its_runs() {
        let base = TextRun {
            len: 3,
            font: font("Inter"),
            color: hsla(0., 0., 0., 1.),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let bold = Highlight {
            weight: Some(FontWeight::SEMIBOLD),
            ..Highlight::new(hsla(0., 0., 0.5, 1.))
        };
        let slanted = Highlight {
            italic: true,
            strike: true,
            ..Highlight::new(hsla(0., 0., 0.2, 1.))
        };
        let split = runs(
            3,
            base,
            vec![(0..2, bold, false), (1..3, slanted, false)],
            px(1.),
        );
        let styles: Vec<_> = split
            .iter()
            .map(|run| {
                (
                    run.len,
                    run.font.weight,
                    run.font.style,
                    run.strikethrough.is_some(),
                )
            })
            .collect();
        assert_eq!(
            styles,
            [
                (1, FontWeight::SEMIBOLD, FontStyle::Normal, false),
                (1, FontWeight::SEMIBOLD, FontStyle::Italic, true),
                (1, FontWeight::NORMAL, FontStyle::Italic, true),
            ]
        );
    }

    #[test]
    fn line_starts_follow_newlines() {
        assert_eq!(line_starts("ab\ncd\n"), vec![0, 3, 6]);
        assert_eq!(line_starts(""), vec![0]);
    }
}
