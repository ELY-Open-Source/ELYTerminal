use std::rc::Rc;

use gpui::{
    App, Bounds, Div, ElementId, Entity, HoverListenerMode, Hsla, InteractiveElement, IntoElement,
    MouseMoveEvent, ParentElement, Pixels, Refineable, RenderOnce, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, canvas, div, fill,
};

use super::{
    axes::{anchored, below, beside},
    geometry::{Rect, frame},
    layout::squarify,
    paint::{measure, place, tint},
    parts::ChartTooltip,
    plot::Format,
    scale::compact,
    series::drawable,
};
use crate::{
    theme::{ActiveTheme, Radius, TextSize},
    typography::{format, tabular},
};

/// A chart's own state when the pointer picks one part: its box and the part under the pointer.
#[derive(Default)]
pub(crate) struct Tiles {
    pub bounds: Bounds<Pixels>,
    pub hover: Option<usize>,
}

impl Tiles {
    /// The part under the pointer, if there are still that many parts.
    pub(crate) fn pointed(&self, count: usize) -> Option<usize> {
        self.hover.filter(|ix| *ix < count)
    }
}

/// Keeps `tiles` answering the pointer: `find` names the tile at a place in the box.
pub(crate) fn tracked(
    root: Div,
    id: ElementId,
    tiles: &Entity<Tiles>,
    find: impl Fn((f32, f32)) -> Option<usize> + 'static,
) -> gpui::Stateful<Div> {
    let (moved, left) = (tiles.clone(), tiles.clone());
    root.id(id)
        .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
            let offset = event.position - moved.read(cx).bounds.origin;
            let next = find((f32::from(offset.x), f32::from(offset.y)));
            moved.update(cx, |tiles, cx| {
                if tiles.hover != next {
                    tiles.hover = next;
                    cx.notify();
                }
            })
        })
        .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
        .on_hover(move |inside, _, cx| {
            if !*inside {
                left.update(cx, |tiles, cx| {
                    tiles.hover = None;
                    cx.notify();
                })
            }
        })
}

/// A key of five steps from none to most, between two words.
pub(crate) fn key(steps: [Hsla; 5], (less, more): (&'static str, &'static str), cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .flex()
        .items_center()
        .justify_end()
        .gap_1()
        .text_size(theme.text_size(TextSize::Xs))
        .text_color(theme.colors.fg_subtle)
        .child(less)
        .children(steps.map(|step| {
            div()
                .size(theme.status_dot() * 1.5)
                .rounded(theme.radius(Radius::Sm))
                .bg(step)
        }))
        .child(more)
}

/// Parts of a whole as tiles whose areas compare, laid out close to square, largest first. Hover reads one.
#[derive(IntoElement)]
pub struct Treemap {
    base: Div,
    id: ElementId,
    tiles: Vec<(SharedString, f64)>,
    format: Format,
}

impl Treemap {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            tiles: Vec::new(),
            format: Rc::new(compact),
        }
    }

    pub fn tile(mut self, name: impl Into<SharedString>, value: f64) -> Self {
        assert!(
            drawable(value) && value >= 0.0,
            "a tile needs a value of zero or more, within charts::LIMIT"
        );
        self.tiles.push((name.into(), value));
        self
    }

    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }
}

impl Styled for Treemap {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Treemap {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tiles: Entity<Tiles> =
            window.use_keyed_state((self.id.clone(), "tiles"), cx, |_, _| Tiles::default());
        let (bounds, hover) = (
            tiles.read(cx).bounds,
            tiles.read(cx).pointed(self.tiles.len()),
        );
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let mut order: Vec<usize> = (0..self.tiles.len()).collect();
        order.sort_by(|a, b| self.tiles[*b].1.total_cmp(&self.tiles[*a].1));
        let whole = Rect {
            x: 0.0,
            y: 0.0,
            w: f32::from(bounds.size.width),
            h: f32::from(bounds.size.height),
        };
        let laid = squarify(
            &order.iter().map(|ix| self.tiles[*ix].1).collect::<Vec<_>>(),
            whole,
        );
        let mut rects = vec![Rect::default(); self.tiles.len()];
        let gap = pixels(sizes.stroke) / 2.0;
        for (slot, ix) in order.iter().enumerate() {
            let rect = laid[slot];
            rects[*ix] = Rect {
                x: rect.x + gap,
                y: rect.y + gap,
                w: (rect.w - gap * 2.0).max(0.0),
                h: (rect.h - gap * 2.0).max(0.0),
            };
        }
        let total: f64 = self.tiles.iter().map(|(_, value)| value).sum();
        let (room, format) = (pixels(sizes.label), self.format.clone());
        let labels: Vec<Div> = rects
            .iter()
            .zip(&self.tiles)
            .filter(|(rect, _)| rect.w >= room && rect.h >= room / 2.0)
            .map(|(rect, (name, value))| {
                div()
                    .absolute()
                    .left(Pixels::from(rect.x))
                    .top(Pixels::from(rect.y))
                    .w(Pixels::from(rect.w))
                    .h(Pixels::from(rect.h))
                    .overflow_hidden()
                    .p_2()
                    .text_size(theme.text_size(TextSize::Xs))
                    .child(div().truncate().text_color(colors.fg).child(name.clone()))
                    .child(
                        tabular(div())
                            .text_color(colors.fg_muted)
                            .child(format(*value)),
                    )
            })
            .collect();
        let tooltip = hover.map(|ix| {
            let rect = rects[ix];
            let share = if total > 0.0 {
                self.tiles[ix].1 / total
            } else {
                0.0
            };
            let card = ChartTooltip::new(self.tiles[ix].0.clone())
                .row(
                    Some(tint(&colors, ix)),
                    "Share",
                    format::percent(share, 1, false),
                )
                .row(None, "Value", format(self.tiles[ix].1));
            let x = rect.x + rect.w;
            anchored(
                (x.min(whole.w), rect.y + rect.h / 2.0),
                x > whole.w * 0.6,
                card,
            )
        });
        let (corner, hairline, painted, hit) = (
            theme.radius(Radius::Sm).to_pixels(rem),
            sizes.hairline,
            rects.clone(),
            rects,
        );
        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());
        tracked(
            root,
            (self.id.clone(), "treemap").into(),
            &tiles,
            move |at| hit.iter().position(|rect| rect.contains(at)),
        )
        .child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    for (ix, rect) in painted.iter().enumerate() {
                        let ink = tint(&colors, ix);
                        let strength = if hover == Some(ix) { 0.34 } else { 0.2 };
                        let tile = fill(place(bounds.origin, *rect), ink.opacity(strength))
                            .corner_radii(corner);
                        window.paint_quad(
                            tile.border_widths(hairline).border_color(ink.opacity(0.6)),
                        );
                    }
                },
            )
            .absolute()
            .inset_0(),
        )
        .children(labels)
        .children(tooltip)
        .child(measure(tiles, |tiles| &mut tiles.bounds))
    }
}

/// A grid of values, a row per label down the side and a column per label along the bottom, each cell as strong as its value. Hover reads a cell.
#[derive(IntoElement)]
pub struct HeatmapChart {
    base: Div,
    id: ElementId,
    columns: Vec<SharedString>,
    rows: Vec<(SharedString, Vec<f64>)>,
    format: Format,
}

impl HeatmapChart {
    pub fn new(
        id: impl Into<ElementId>,
        columns: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        let columns = columns.into_iter().map(Into::into).collect();
        Self {
            base: div(),
            id: id.into(),
            columns,
            rows: Vec::new(),
            format: Rc::new(compact),
        }
    }

    /// A row of values, one for each column.
    pub fn row(
        mut self,
        label: impl Into<SharedString>,
        values: impl IntoIterator<Item = f64>,
    ) -> Self {
        let values: Vec<f64> = values.into_iter().collect();
        assert_eq!(
            values.len(),
            self.columns.len(),
            "a heatmap row needs a value per column"
        );
        assert!(
            values.iter().all(|value| drawable(*value)),
            "a heatmap needs values, within charts::LIMIT"
        );
        self.rows.push((label.into(), values));
        self
    }

    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }
}

impl Styled for HeatmapChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

/// Five steps of one ink, faint to full, for a key.
fn steps(ink: Hsla) -> [Hsla; 5] {
    [0.08, 0.3, 0.52, 0.76, 1.0].map(|strength| ink.opacity(strength))
}

impl RenderOnce for HeatmapChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tiles: Entity<Tiles> =
            window.use_keyed_state((self.id.clone(), "tiles"), cx, |_, _| Tiles::default());
        let cells = self.rows.len() * self.columns.len();
        let (bounds, hover) = (tiles.read(cx).bounds, tiles.read(cx).pointed(cells));
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let size = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let rect = frame(
            size,
            pixels(sizes.gutter),
            pixels(sizes.foot),
            pixels(sizes.inset),
        );
        let (columns, rows) = (self.columns.len(), self.rows.len());
        let (cell_w, cell_h) = (rect.w / columns.max(1) as f32, rect.h / rows.max(1) as f32);
        let cell = |row: usize, column: usize| Rect {
            x: rect.x + cell_w * column as f32,
            y: rect.y + cell_h * row as f32,
            w: cell_w,
            h: cell_h,
        };
        let (low, high) = self
            .rows
            .iter()
            .flat_map(|(_, values)| values)
            .fold((f64::MAX, f64::MIN), |(low, high), value| {
                (low.min(*value), high.max(*value))
            });
        let ink = tint(&colors, 0);
        let strength = move |value: f64| {
            if high > low {
                0.08 + 0.92 * ((value - low) / (high - low)) as f32
            } else {
                0.5
            }
        };
        let text = |content: SharedString| {
            div()
                .flex_none()
                .whitespace_nowrap()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(colors.fg_subtle)
                .child(content)
        };
        let every = ((self.columns.len() as f32 * pixels(sizes.label) / rect.w.max(1.0)).ceil()
            as usize)
            .max(1);
        let column_labels = self
            .columns
            .iter()
            .enumerate()
            .filter(|(ix, _)| ix % every == 0)
            .map(|(ix, label)| {
                below(
                    rect.x + cell_w * (ix as f32 + 0.5),
                    rect,
                    text(label.clone()),
                )
            });
        let row_labels = self.rows.iter().enumerate().map(|(ix, (label, _))| {
            beside(
                rect.y + cell_h * (ix as f32 + 0.5),
                sizes.gutter.to_pixels(rem),
                text(label.clone()),
            )
        });
        let format = self.format.clone();
        let tooltip = hover.map(|at| {
            let (row, column) = (at / columns, at % columns);
            let place = cell(row, column);
            let card =
                ChartTooltip::new(format!("{} · {}", self.rows[row].0, self.columns[column])).row(
                    Some(ink.opacity(strength(self.rows[row].1[column]))),
                    "Value",
                    format(self.rows[row].1[column]),
                );
            anchored(
                (place.x + place.w, place.y + place.h / 2.0),
                place.x > size.0 * 0.6,
                card,
            )
        });
        let cells: Vec<(Rect, Hsla)> = self
            .rows
            .iter()
            .enumerate()
            .flat_map(|(row, (_, values))| {
                values
                    .iter()
                    .enumerate()
                    .map(move |(column, value)| (row, column, *value))
            })
            .map(|(row, column, value)| (cell(row, column), ink.opacity(strength(value))))
            .collect();
        let (gap, corner, edge, hairline) = (
            pixels(sizes.stroke) / 2.0,
            theme.radius(Radius::Sm).to_pixels(rem),
            colors.fg,
            sizes.hairline,
        );
        let legend = key(steps(ink), ("Less", "More"), cx);
        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());
        let chart = tracked(
            div().relative().flex_1().min_h_0(),
            (self.id.clone(), "heatmap").into(),
            &tiles,
            move |at| {
                (rect.contains(at) && rows > 0 && columns > 0).then(|| {
                    let (row, column) = (
                        ((at.1 - rect.y) / cell_h) as usize,
                        ((at.0 - rect.x) / cell_w) as usize,
                    );
                    row.min(rows - 1) * columns + column.min(columns - 1)
                })
            },
        )
        .child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    for (ix, (cell, color)) in cells.iter().enumerate() {
                        let inner = Rect {
                            x: cell.x + gap,
                            y: cell.y + gap,
                            w: (cell.w - gap * 2.0).max(0.0),
                            h: (cell.h - gap * 2.0).max(0.0),
                        };
                        let quad = fill(place(bounds.origin, inner), *color).corner_radii(corner);
                        window.paint_quad(if hover == Some(ix) {
                            quad.border_widths(hairline).border_color(edge)
                        } else {
                            quad
                        });
                    }
                },
            )
            .absolute()
            .inset_0(),
        )
        .children(column_labels)
        .children(row_labels)
        .children(tooltip)
        .child(measure(tiles, |tiles| &mut tiles.bounds));
        root.flex().flex_col().gap_2().child(chart).child(legend)
    }
}
