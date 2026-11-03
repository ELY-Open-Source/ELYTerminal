use std::rc::Rc;

use gpui::{
    App, Bounds, ElementId, Entity, HoverListenerMode, InteractiveElement, IntoElement,
    ParentElement, Pixels, Point, RenderOnce, SharedString, StatefulInteractiveElement, Styled,
    Window, canvas, div, fill, point, prelude::*, size, transparent_black,
};

use crate::{
    documents::source,
    forms::{Drawing, pen},
    primitives::{FocusRing, Image, checked_ratio, framed, tab_stop},
    theme::ActiveTheme,
};

/// Cells across a mask; rows follow the picture's shape.
const COLUMNS: usize = 320;

/// The brush's smallest and largest radius, as shares of the picture's width.
pub(crate) const REACH: (f32, f32) = (0.01, 0.2);

/// A stroke of the mask brush: points as shares of the picture across and down, a radius as a share of its width, and whether it erases.
#[derive(Clone, Debug, PartialEq)]
pub struct MaskStroke {
    pub points: Vec<(f32, f32)>,
    pub radius: f32,
    pub erase: bool,
}

/// The squared distance from `p` to the segment from `a` to `b`.
fn reach(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = dx * dx + dy * dy;
    let t = match length {
        0.0 => 0.0,
        _ => (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length).clamp(0.0, 1.0),
    };
    let (x, y) = (a.0 + t * dx - p.0, a.1 + t * dy - p.1);
    x * x + y * y
}

/// Marks the cells `stroke` reaches on a `columns` by `rows` grid over a picture `ratio` wide over tall, or clears them if it erases; reach is measured in widths, so a dab is as tall as wide.
fn paint(cells: &mut [bool], stroke: &MaskStroke, columns: usize, rows: usize, ratio: f32) {
    let column = |x: f32| ((x * columns as f32).floor().max(0.0) as usize).min(columns);
    let row = |y: f32| ((y * ratio * rows as f32).floor().max(0.0) as usize).min(rows);
    let across: Vec<(f32, f32)> = stroke.points.iter().map(|&(x, y)| (x, y / ratio)).collect();
    let pairs: Vec<_> = match across.as_slice() {
        [] => return,
        [only] => vec![(*only, *only)],
        _ => across.windows(2).map(|pair| (pair[0], pair[1])).collect(),
    };
    let (r, near) = (stroke.radius, stroke.radius * stroke.radius);
    for (a, b) in pairs {
        let columns_in = column(a.0.min(b.0) - r)..(column(a.0.max(b.0) + r) + 1).min(columns);
        let rows_in = row(a.1.min(b.1) - r)..(row(a.1.max(b.1) + r) + 1).min(rows);
        for j in rows_in {
            for i in columns_in.clone() {
                let center = (
                    (i as f32 + 0.5) / columns as f32,
                    (j as f32 + 0.5) / rows as f32 / ratio,
                );
                if reach(center, a, b) <= near {
                    cells[j * columns + i] = !stroke.erase;
                }
            }
        }
    }
}

/// The cells the strokes cover, row by row: a later stroke wins, and an erasing one clears.
pub(crate) fn raster(strokes: &[MaskStroke], columns: usize, rows: usize, ratio: f32) -> Vec<bool> {
    let mut cells = vec![false; columns * rows];
    for stroke in strokes {
        paint(&mut cells, stroke, columns, rows, ratio);
    }
    cells
}

/// Covered cells as rectangles: runs in a row, merged down while the rows below run the same. Each is first row, row past the last, first column, column past the last.
pub(crate) fn spans(cells: &[bool], columns: usize) -> Vec<(usize, usize, usize, usize)> {
    let mut open: Vec<(usize, usize, usize, usize)> = Vec::new();
    let mut done = Vec::new();
    for (j, line) in cells.chunks(columns).enumerate() {
        let mut runs = Vec::new();
        let mut start = None;
        for (i, &on) in line.iter().chain(std::iter::once(&false)).enumerate() {
            match (on, start) {
                (true, None) => start = Some(i),
                (false, Some(from)) => {
                    runs.push((from, i));
                    start = None;
                }
                _ => {}
            }
        }
        let mut next = Vec::new();
        for rect in open.drain(..) {
            match runs.iter().position(|&run| run == (rect.2, rect.3)) {
                Some(at) => {
                    runs.remove(at);
                    next.push((rect.0, j + 1, rect.2, rect.3));
                }
                None => done.push(rect),
            }
        }
        next.extend(runs.into_iter().map(|(from, to)| (j, j + 1, from, to)));
        open = next;
    }
    done.extend(open);
    done
}

type OnStroke = Rc<dyn Fn(MaskStroke, &mut Window, &mut App)>;
pub(crate) type OnBrush = Rc<dyn Fn(f32, bool, &mut Window, &mut App)>;

/// A picture to repaint in part: the brush masks where it passes and an erasing brush clears, the mask a wash over the picture. A ring follows the pointer at the brush's size. Focused, [ and ] size the brush and X swaps painting for erasing.
#[derive(IntoElement)]
pub struct InpaintCanvas {
    id: ElementId,
    picture: SharedString,
    ratio: f32,
    strokes: Vec<MaskStroke>,
    radius: f32,
    erase: bool,
    on_stroke: Option<OnStroke>,
    on_brush: Option<OnBrush>,
}

impl InpaintCanvas {
    /// `ratio` is the picture's width over height; `strokes` are the mask so far, oldest first.
    pub fn new(
        id: impl Into<ElementId>,
        picture: impl Into<SharedString>,
        ratio: f32,
        strokes: impl IntoIterator<Item = MaskStroke>,
    ) -> Self {
        let strokes: Vec<MaskStroke> = strokes.into_iter().collect();
        for stroke in &strokes {
            assert!(
                (REACH.0..=REACH.1).contains(&stroke.radius),
                "a mask stroke's radius of {}",
                stroke.radius
            );
            assert!(
                (stroke.points.iter())
                    .all(|&(x, y)| (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y)),
                "a mask stroke outside its picture"
            );
        }
        Self {
            id: id.into(),
            picture: picture.into(),
            ratio: checked_ratio(ratio),
            strokes,
            radius: 0.04,
            erase: false,
            on_stroke: None,
            on_brush: None,
        }
    }

    /// The brush's radius as a share of the picture's width, and whether it erases.
    pub fn brush(mut self, radius: f32, erase: bool) -> Self {
        assert!(
            (REACH.0..=REACH.1).contains(&radius),
            "a brush radius of {radius}"
        );
        self.radius = radius;
        self.erase = erase;
        self
    }

    /// Gets each stroke as the pointer lifts.
    pub fn on_stroke(
        mut self,
        handler: impl Fn(MaskStroke, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_stroke = Some(Rc::new(handler));
        self
    }

    /// Gets the brush the keys set: its radius and whether it erases.
    pub fn on_brush(
        mut self,
        handler: impl Fn(f32, bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_brush = Some(Rc::new(handler));
        self
    }
}

type Rects = Rc<Vec<(usize, usize, usize, usize)>>;

/// The strokes a mask last drew, its rows, and the cells and rectangles they made.
type Shape = (Vec<MaskStroke>, usize, Rc<Vec<bool>>, Rects);

/// The mask's rectangles: the settled strokes' cells kept until they or the rows change, and a stroke under the pen painted over a copy.
fn shaped(
    cache: &Entity<Shape>,
    strokes: Vec<MaskStroke>,
    live: Option<MaskStroke>,
    rows: usize,
    ratio: f32,
    cx: &mut App,
) -> Rects {
    let stale = {
        let (kept, kept_rows, _, _) = cache.read(cx);
        *kept != strokes || *kept_rows != rows
    };
    if stale {
        let cells = raster(&strokes, COLUMNS, rows, ratio);
        let rects = Rc::new(spans(&cells, COLUMNS));
        cache.update(cx, |cache, _| {
            *cache = (strokes, rows, Rc::new(cells), rects)
        });
    }
    let (_, _, cells, rects) = cache.read(cx);
    match live {
        None => rects.clone(),
        Some(stroke) => {
            let mut cells = cells.to_vec();
            paint(&mut cells, &stroke, COLUMNS, rows, ratio);
            Rc::new(spans(&cells, COLUMNS))
        }
    }
}

/// `at` as shares of `bounds`, kept on the picture as the pen keeps its points in the pad; a canvas that shrank mid-stroke ends the stroke at its edge.
fn share(bounds: Bounds<Pixels>, at: Point<Pixels>) -> (f32, f32) {
    let (width, height) = (
        f32::from(bounds.size.width).max(1.0),
        f32::from(bounds.size.height).max(1.0),
    );
    (
        (f32::from(at.x) / width).clamp(0.0, 1.0),
        (f32::from(at.y) / height).clamp(0.0, 1.0),
    )
}

impl RenderOnce for InpaintCanvas {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let drawing =
            window.use_keyed_state((self.id.clone(), "drawing"), cx, |_, _| Drawing::default());
        let pointer =
            window.use_keyed_state(
                (self.id.clone(), "pointer"),
                cx,
                |_, _| None::<Point<Pixels>>,
            );
        let cache: Entity<Shape> =
            window.use_keyed_state((self.id.clone(), "shape"), cx, |_, _| {
                (Vec::new(), 0, Rc::new(Vec::new()), Rc::new(Vec::new()))
            });
        let (keyed, drawn) = (self.on_brush.is_some(), self.on_stroke.is_some());
        let focus = tab_stop((self.id.clone(), "focus").into(), keyed, window, cx);
        let (bounds, under) = (drawing.read(cx).bounds, drawing.read(cx).stroke.clone());
        let (radius, erase) = (self.radius, self.erase);
        let live = (!under.is_empty()).then(|| MaskStroke {
            points: under.iter().map(|at| share(bounds, *at)).collect(),
            radius,
            erase,
        });
        let rows = ((COLUMNS as f32 / self.ratio).round() as usize).max(1);
        let rects = shaped(&cache, self.strokes, live, rows, self.ratio, cx);
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let wash = colors.accent.alpha(0.45);
        let ring = pointer
            .read(cx)
            .filter(|_| drawn && bounds.size.width > Pixels::ZERO)
            .map(|at| {
                let span = bounds.size.width * radius * 2.0;
                div()
                    .debug_selector(|| "inpaint-ring".into())
                    .absolute()
                    .left(at.x - bounds.left() - span / 2.0)
                    .top(at.y - bounds.top() - span / 2.0)
                    .size(span)
                    .rounded_full()
                    .border_1()
                    .border_color(colors.on_media)
                    .child(
                        div()
                            .size_full()
                            .rounded_full()
                            .border_1()
                            .border_color(colors.backdrop),
                    )
            });
        let (measure, hover, leave, keys) = (
            drawing.clone(),
            pointer.clone(),
            pointer,
            self.on_brush.clone(),
        );
        let (id, on_stroke) = (self.id.clone(), self.on_stroke.clone());
        let area = framed(self.ratio, cx)
            .id(self.id.clone())
            .border_1()
            .border_color(transparent_black())
            .when(keyed, |area| area.track_focus(&focus).focus_ring(cx))
            .when(drawn, |area| area.cursor_crosshair())
            .child(Image::new((self.id.clone(), "picture"), source(&self.picture)).size_full())
            .child(
                canvas(
                    move |bounds, _, cx| {
                        if measure.read(cx).bounds != bounds {
                            measure.update(cx, |drawing, _| drawing.bounds = bounds);
                        }
                    },
                    move |bounds, _, window, _| {
                        let scale = window.scale_factor();
                        let snap = |at: f32| Pixels::from((at * scale).round() / scale);
                        let (width, height) =
                            (f32::from(bounds.size.width), f32::from(bounds.size.height));
                        let (left, top) = (f32::from(bounds.left()), f32::from(bounds.top()));
                        for (first, past, from, to) in rects.iter() {
                            let x0 = snap(left + width * *from as f32 / COLUMNS as f32);
                            let x1 = snap(left + width * *to as f32 / COLUMNS as f32);
                            let y0 = snap(top + height * *first as f32 / rows as f32);
                            let y1 = snap(top + height * *past as f32 / rows as f32);
                            window.paint_quad(fill(
                                Bounds::new(point(x0, y0), size(x1 - x0, y1 - y0)),
                                wash,
                            ));
                        }
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .children(ring)
            .when(drawn, |area| {
                area.on_mouse_move(move |event, _, cx| {
                    hover.update(cx, |pointer, cx| {
                        *pointer = Some(event.position);
                        cx.notify();
                    })
                })
                .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
                .on_hover(move |hovered, _, cx| {
                    if !hovered {
                        leave.update(cx, |pointer, cx| {
                            *pointer = None;
                            cx.notify();
                        })
                    }
                })
            })
            .on_key_down(move |event, window, cx| {
                let Some(keys) = &keys else {
                    return;
                };
                let (next, swap) = match event.keystroke.key.as_str() {
                    "[" => ((radius * 0.8).max(REACH.0), erase),
                    "]" => ((radius * 1.25).min(REACH.1), erase),
                    "x" => (radius, !erase),
                    _ => return,
                };
                cx.stop_propagation();
                log::info!("inpaint canvas: brush {next:.3}, erase {swap}");
                keys(next, swap, window, cx)
            });
        let Some(on_stroke) = on_stroke else {
            return area;
        };
        let sized = drawing.clone();
        pen(area, &drawing, move |stroke, window, cx| {
            let bounds = sized.read(cx).bounds;
            let points = stroke.iter().map(|at| share(bounds, *at)).collect();
            log::info!("inpaint canvas {id:?}: a stroke of {} points", stroke.len());
            on_stroke(
                MaskStroke {
                    points,
                    radius,
                    erase,
                },
                window,
                cx,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{MaskStroke, raster, spans};

    fn stroke(points: &[(f32, f32)], radius: f32, erase: bool) -> MaskStroke {
        MaskStroke {
            points: points.to_vec(),
            radius,
            erase,
        }
    }

    fn count(cells: &[bool]) -> usize {
        cells.iter().filter(|on| **on).count()
    }

    #[test]
    fn a_dab_covers_a_disc_as_wide_as_tall() {
        let cells = raster(&[stroke(&[(0.5, 0.5)], 0.1, false)], 20, 10, 2.0);
        assert_eq!(count(&cells), 12, "two cells of reach on square cells");
        assert!(
            cells[3 * 20 + 9] && cells[4 * 20 + 8],
            "as far down as across"
        );
        assert!(!cells[3 * 20 + 8], "the disc's corner stays out");
    }

    #[test]
    fn a_later_stroke_wins_and_an_erasing_one_clears() {
        let paint = stroke(&[(0.5, 0.5)], 0.1, false);
        let erase = stroke(&[(0.5, 0.5)], 0.05, true);
        assert_eq!(
            count(&raster(&[paint.clone(), erase.clone()], 20, 10, 2.0)),
            8
        );
        assert_eq!(count(&raster(&[erase, paint], 20, 10, 2.0)), 12);
    }

    #[test]
    fn a_line_covers_a_band_with_round_ends() {
        let cells = raster(
            &[stroke(&[(0.2, 0.5), (0.8, 0.5)], 0.05, false)],
            20,
            10,
            2.0,
        );
        assert_eq!(count(&cells), 28);
    }

    #[test]
    fn runs_that_match_below_merge_into_one_rectangle() {
        let cells = [
            false, true, true, false, //
            false, true, true, false, //
            true, true, false, false,
        ];
        assert_eq!(spans(&cells, 4), [(0, 2, 1, 3), (2, 3, 0, 2)]);
    }
}
