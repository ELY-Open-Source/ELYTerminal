use gpui::{
    App, Div, ElementId, Entity, FontWeight, Hsla, IntoElement, ParentElement, Pixels, RenderOnce,
    SharedString, StyleRefinement, Styled, Window, canvas, div, fill, prelude::*,
};

use super::quotes::moves;
use crate::{
    charts::{ChartTooltip, Rect, Tiles, anchored, measure, place, squarify, tracked},
    theme::{ActiveTheme, Radius, TextSize},
    typography::{format, tabular},
};

/// Symbols as tiles sized by their weight and tinted by their move: deeper green as they rise, deeper red as they fall, quiet near flat. Hover reads one.
#[derive(IntoElement)]
pub struct MarketHeatmap {
    base: Div,
    id: ElementId,
    tiles: Vec<(SharedString, f64, f64)>,
    full: f64,
    red_up: bool,
}

impl MarketHeatmap {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            tiles: Vec::new(),
            full: 0.03,
            red_up: false,
        }
    }

    /// A symbol, its weight, such as its market value, and its move as a share.
    pub fn tile(mut self, symbol: impl Into<SharedString>, weight: f64, change: f64) -> Self {
        assert!(
            weight.is_finite() && weight >= 0.0 && change.is_finite(),
            "a tile needs a weight of zero or more and a finite move"
        );
        self.tiles.push((symbol.into(), weight, change));
        self
    }

    /// The move, as a share, that takes the deepest tint.
    pub fn full(mut self, share: f64) -> Self {
        assert!(share > 0.0, "the deepest tint needs a move");
        self.full = share;
        self
    }

    pub fn red_up(mut self) -> Self {
        self.red_up = true;
        self
    }
}

impl Styled for MarketHeatmap {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

/// How strongly a move tints its tile: faint near flat, deepest at `full`.
pub(crate) fn strength(change: f64, full: f64) -> f32 {
    0.12 + 0.78 * (change.abs() / full).min(1.0) as f32
}

impl RenderOnce for MarketHeatmap {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tiles: Entity<Tiles> =
            window.use_keyed_state((self.id.clone(), "heat"), cx, |_, _| Tiles::default());
        let (bounds, hover) = (
            tiles.read(cx).bounds,
            tiles.read(cx).pointed(self.tiles.len()),
        );
        let (rise, fall) = moves(self.red_up, cx);
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
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
        let seam = f32::from(sizes.hairline);
        let mut rects = vec![Rect::default(); self.tiles.len()];
        for (slot, ix) in order.iter().enumerate() {
            let rect = laid[slot];
            rects[*ix] = Rect {
                x: rect.x + seam,
                y: rect.y + seam,
                w: (rect.w - seam * 2.0).max(0.0),
                h: (rect.h - seam * 2.0).max(0.0),
            };
        }
        let fills: Vec<(Hsla, f32)> = self
            .tiles
            .iter()
            .map(|(_, _, change)| {
                (
                    if *change >= 0.0 { rise } else { fall },
                    strength(*change, self.full),
                )
            })
            .collect();
        let room = f32::from(sizes.label.to_pixels(rem)) * 0.75;
        let labels: Vec<Div> = rects
            .iter()
            .zip(&self.tiles)
            .zip(&fills)
            .filter(|((rect, _), _)| rect.w >= room && rect.h >= room * 0.6)
            .map(|((rect, (symbol, _, change)), (_, strong))| {
                let ink = if *strong > 0.55 {
                    colors.on_accent
                } else {
                    colors.fg
                };
                div()
                    .absolute()
                    .left(Pixels::from(rect.x))
                    .top(Pixels::from(rect.y))
                    .w(Pixels::from(rect.w))
                    .h(Pixels::from(rect.h))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .text_color(ink)
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(symbol.clone()),
                    )
                    .child(
                        tabular(div())
                            .text_size(theme.text_size(TextSize::Xs))
                            .child(format::percent(*change, 2, true)),
                    )
            })
            .collect();
        let tooltip = hover.map(|ix| {
            let (symbol, weight, change) = &self.tiles[ix];
            let rect = rects[ix];
            let card = ChartTooltip::new(symbol.clone())
                .row(Some(fills[ix].0), "Move", format::percent(*change, 2, true))
                .row(None, "Weight", crate::charts::compact(*weight));
            let x = rect.x + rect.w;
            anchored(
                (x.min(whole.w), rect.y + rect.h / 2.0),
                x > whole.w * 0.6,
                card,
            )
        });
        let (painted, hit, corner, edge) = (
            rects.clone(),
            rects,
            theme.radius(Radius::Sm).to_pixels(rem),
            colors.fg,
        );
        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());
        tracked(
            root,
            (self.id.clone(), "tiles").into(),
            &tiles,
            move |place| hit.iter().position(|rect| rect.contains(place)),
        )
        .child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    for (ix, (rect, (ink, strong))) in painted.iter().zip(&fills).enumerate() {
                        let quad = fill(place(bounds.origin, *rect), ink.opacity(*strong))
                            .corner_radii(corner);
                        window.paint_quad(if hover == Some(ix) {
                            quad.border_widths(sizes.hairline).border_color(edge)
                        } else {
                            quad
                        });
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

#[cfg(test)]
mod tests {
    use super::strength;

    #[test]
    fn moves_tint_up_to_full() {
        assert_eq!(strength(0.0, 0.03), 0.12, "flat stays faint");
        assert_eq!(
            strength(-0.06, 0.03),
            0.9,
            "past full holds the deepest tint"
        );
    }
}
