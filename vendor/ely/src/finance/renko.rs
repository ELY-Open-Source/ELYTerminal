use std::rc::Rc;

use gpui::{
    AnyElement, App, Div, ElementId, Entity, InteractiveElement, IntoElement, ParentElement,
    PathBuilder, Pixels, Refineable, RenderOnce, StyleRefinement, Styled, Window, canvas, div,
    fill,
};

use jiff::tz::TimeZone;

use super::{
    candles::{Candle, griddable, point_and_figure, renko},
    quotes::moves,
    stage::fit,
};
use crate::{
    charts::{
        ChartTooltip, Linear, Rect, Tiles, anchored, at, finish, measure, place, ring, tracked,
    },
    theme::{ActiveTheme, TextSize},
    typography::{
        format::{self, datetime, decimals, system_zone},
        tabular,
    },
};

/// A mark on a price-only chart: its slot, the prices it spans, whether it rose, and its words.
struct Mark {
    slot: usize,
    span: (f64, f64),
    rose: bool,
    title: String,
    reading: String,
}

/// A box size too fine for the candles: logged, and said in the chart's place.
fn too_fine(id: &ElementId, size: f64, cx: &App) -> AnyElement {
    log::error!("{id:?}: boxes of {size} are too fine for these prices");
    let theme = cx.theme();
    div()
        .text_size(theme.text_size(TextSize::Sm))
        .text_color(theme.colors.danger)
        .child(format!("Boxes of {size} are too fine for these prices"))
        .into_any_element()
}

/// What both price-only charts share: a slot per mark along the bottom, prices at the right, and a tooltip for the mark under the pointer.
fn board(
    id: ElementId,
    marks: Vec<Mark>,
    boxes: Option<f64>,
    (base, red_up): (&mut Div, bool),
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement + use<> {
    let tiles: Entity<Tiles> =
        window.use_keyed_state((id.clone(), "board"), cx, |_, _| Tiles::default());
    let (bounds, hover) = (tiles.read(cx).bounds, tiles.read(cx).hover);
    let theme = cx.theme();
    let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
    let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
    let (inset, gutter, foot) = (
        pixels(sizes.inset),
        pixels(sizes.gutter) * 1.25,
        pixels(sizes.foot),
    );
    let frame = Rect {
        x: inset,
        y: inset,
        w: (f32::from(bounds.size.width) - inset - gutter).max(0.0),
        h: (f32::from(bounds.size.height) - inset - foot).max(0.0),
    };
    let (low, high) = marks
        .iter()
        .fold((f64::MAX, f64::MIN), |(low, high), mark| {
            (
                low.min(mark.span.0.min(mark.span.1)),
                high.max(mark.span.0.max(mark.span.1)),
            )
        });
    let ((low, high), ticks) = if marks.is_empty() {
        fit(0.0, 1.0)
    } else {
        fit(low, high)
    };
    let ys = Linear::new((low, high), (frame.y + frame.h, frame.y));
    let slots = marks.iter().map(|mark| mark.slot + 1).max().unwrap_or(1);
    let wide = frame.w / slots as f32;
    let (rise, fall) = moves(red_up, cx);
    let digits = decimals(
        ticks
            .windows(2)
            .next()
            .map_or(1.0, |pair| pair[1] - pair[0]),
    )
    .max(2);
    let edge = frame.x + frame.w;
    let text = |words: String| {
        div()
            .flex_none()
            .whitespace_nowrap()
            .text_size(theme.text_size(TextSize::Xs))
            .text_color(colors.fg_subtle)
            .child(words)
    };
    let mut words: Vec<Div> = ticks
        .iter()
        .map(|tick| {
            div()
                .absolute()
                .left(Pixels::from(edge))
                .top(Pixels::from(ys.at(*tick)))
                .h_0()
                .flex()
                .items_center()
                .pl_2()
                .child(tabular(text(format::number(
                    *tick,
                    digits,
                    format::Separators::EN,
                ))))
        })
        .collect();
    let place_of = |mark: &Mark| {
        let (top, bottom) = (
            ys.at(mark.span.0.max(mark.span.1)),
            ys.at(mark.span.0.min(mark.span.1)),
        );
        Rect {
            x: frame.x + wide * mark.slot as f32,
            y: top,
            w: wide,
            h: bottom - top,
        }
    };
    if let Some(mark) = hover.and_then(|ix| marks.get(ix)) {
        let rect = place_of(mark);
        let card = ChartTooltip::new(mark.title.clone()).row(
            Some(if mark.rose { rise } else { fall }),
            "Prices",
            mark.reading.clone(),
        );
        words.push(anchored(
            (rect.x + rect.w, rect.y + rect.h / 2.0),
            rect.x > f32::from(bounds.size.width) * 0.6,
            card,
        ));
    }
    let rects: Vec<(Rect, bool)> = marks
        .iter()
        .map(|mark| (place_of(mark), mark.rose))
        .collect();
    let hit = rects.clone();
    let (stroke, hairline, palette, corner) = (
        sizes.stroke.to_pixels(rem),
        sizes.hairline,
        colors.clone(),
        theme.radius(crate::theme::Radius::Sm).to_pixels(rem),
    );
    let box_px = boxes.map(|size| (ys.at(0.0) - ys.at(size)).abs());
    let mut root = div()
        .debug_selector(|| "chart-root".into())
        .relative()
        .h(sizes.height);
    root.style().refine(base.style());
    tracked(root, (id, "marks").into(), &tiles, move |place| {
        hit.iter().position(|(rect, _)| rect.contains(place))
    })
    .child(
        canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                let origin = bounds.origin;
                for tick in &ticks {
                    let line = Rect {
                        x: frame.x,
                        y: ys.at(*tick),
                        w: frame.w,
                        h: f32::from(hairline),
                    };
                    window.paint_quad(fill(place(origin, line), palette.border.opacity(0.5)));
                }
                for (ix, (rect, rose)) in rects.iter().enumerate() {
                    let ink = if *rose { rise } else { fall };
                    let ink = if hover.is_some_and(|on| on != ix) {
                        ink.opacity(0.5)
                    } else {
                        ink
                    };
                    let gap = wide * 0.12;
                    let Some(side) = box_px.filter(|side| *side >= 1.0) else {
                        let brick = Rect {
                            x: rect.x + gap,
                            y: rect.y,
                            w: rect.w - gap * 2.0,
                            h: rect.h,
                        };
                        window.paint_quad(
                            fill(place(origin, brick), ink.opacity(0.85))
                                .corner_radii(Pixels::from(f32::from(corner).min(brick.w / 4.0))),
                        );
                        continue;
                    };
                    let count = (rect.h / side).round().max(1.0) as usize;
                    let half = (side.min(wide) / 2.0 - gap).max(f32::from(hairline));
                    for step in 0..count {
                        let center = (rect.x + wide / 2.0, rect.y + side * (step as f32 + 0.5));
                        if *rose {
                            let mut cross = PathBuilder::stroke(stroke * 0.75);
                            cross.move_to(at(origin, (center.0 - half, center.1 - half)));
                            cross.line_to(at(origin, (center.0 + half, center.1 + half)));
                            cross.move_to(at(origin, (center.0 + half, center.1 - half)));
                            cross.line_to(at(origin, (center.0 - half, center.1 + half)));
                            finish(cross, ink, window);
                        } else {
                            ring(
                                at(origin, center),
                                Pixels::from(half),
                                (palette.bg.opacity(0.0), ink),
                                stroke * 0.75,
                                window,
                            );
                        }
                    }
                }
            },
        )
        .absolute()
        .inset_0(),
    )
    .children(words)
    .child(measure(tiles, |tiles| &mut tiles.bounds))
}

/// Prices as bricks of one size, laid only when the close moves a whole brick past the last, so time drops out and trends stand clear. Hover reads a brick.
#[derive(IntoElement)]
pub struct RenkoChart {
    base: Div,
    id: ElementId,
    candles: Rc<Vec<Candle>>,
    size: f64,
    zone: Option<TimeZone>,
    red_up: bool,
}

impl RenkoChart {
    pub fn new(id: impl Into<ElementId>, candles: impl Into<Rc<Vec<Candle>>>, size: f64) -> Self {
        assert!(
            size.is_finite() && size > 0.0,
            "a brick needs a positive size"
        );
        Self {
            base: div(),
            id: id.into(),
            candles: candles.into(),
            size,
            zone: None,
            red_up: false,
        }
    }

    pub fn red_up(mut self) -> Self {
        self.red_up = true;
        self
    }

    /// The time zone its dates read in; the system's unless set.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }
}

impl Styled for RenkoChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for RenkoChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let zone = self
            .zone
            .clone()
            .unwrap_or_else(|| system_zone("RenkoChart"));
        if !griddable(&self.candles, self.size) {
            return too_fine(&self.id, self.size, cx);
        }
        let marks = renko(&self.candles, self.size)
            .into_iter()
            .enumerate()
            .map(|(slot, brick)| {
                let day = datetime(self.candles[brick.at].time, &zone, "%b %-d, %Y")
                    .expect("a fixed pattern");
                let reading = format!(
                    "{} → {}",
                    format::number(brick.from, 2, format::Separators::EN),
                    format::number(brick.to, 2, format::Separators::EN)
                );
                Mark {
                    slot,
                    span: (brick.from, brick.to),
                    rose: brick.to > brick.from,
                    title: day,
                    reading,
                }
            })
            .collect();
        board(
            self.id.clone(),
            marks,
            None,
            (&mut self.base, self.red_up),
            window,
            cx,
        )
        .into_any_element()
    }
}

/// Prices as columns of Xs where they rose and Os where they fell, a new column each time they turn by the reversal. Hover reads a column.
#[derive(IntoElement)]
pub struct PointFigureChart {
    base: Div,
    id: ElementId,
    candles: Rc<Vec<Candle>>,
    size: f64,
    reversal: i64,
    red_up: bool,
}

impl PointFigureChart {
    /// Boxes of `size`, and a turn after `reversal` boxes the other way.
    pub fn new(
        id: impl Into<ElementId>,
        candles: impl Into<Rc<Vec<Candle>>>,
        size: f64,
        reversal: i64,
    ) -> Self {
        assert!(
            size.is_finite() && size > 0.0 && reversal > 0,
            "boxes need a positive size and reversal"
        );
        Self {
            base: div(),
            id: id.into(),
            candles: candles.into(),
            size,
            reversal,
            red_up: false,
        }
    }

    pub fn red_up(mut self) -> Self {
        self.red_up = true;
        self
    }
}

impl Styled for PointFigureChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for PointFigureChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let size = self.size;
        if !griddable(&self.candles, size) {
            return too_fine(&self.id, size, cx);
        }
        let marks = point_and_figure(&self.candles, size, self.reversal)
            .into_iter()
            .enumerate()
            .map(|(slot, column)| {
                let (low, high) = (
                    column.from.min(column.to) as f64 * size,
                    (column.from.max(column.to) + 1) as f64 * size,
                );
                let boxes = column.from.abs_diff(column.to) + 1;
                let title = format!("{} {}", boxes, if column.rising { "X" } else { "O" });
                let reading = format!(
                    "{} – {}",
                    format::number(low, 2, format::Separators::EN),
                    format::number(high, 2, format::Separators::EN)
                );
                Mark {
                    slot,
                    span: (low, high),
                    rose: column.rising,
                    title,
                    reading,
                }
            })
            .collect();
        board(
            self.id.clone(),
            marks,
            Some(size),
            (&mut self.base, self.red_up),
            window,
            cx,
        )
        .into_any_element()
    }
}
