use std::{
    f32::consts::{FRAC_PI_2, TAU},
    rc::Rc,
};

use gpui::{
    App, Div, ElementId, Entity, IntoElement, ParentElement, PathBuilder, Pixels, Point,
    Refineable, RenderOnce, SharedString, StyleRefinement, Styled, Window, canvas, div, prelude::*,
};

use super::{
    axes::anchored,
    paint::{finish, measure, tint},
    parts::ChartTooltip,
    pie::{bearing, toward, wedge},
    plot::Format,
    scale::compact,
    series::drawable,
    tiles::{Tiles, tracked},
};
use crate::theme::{ActiveTheme, TextSize};

/// Arcs around a circle: each a start and a sweep.
type Arcs = Vec<(f32, f32)>;

/// A chord layout: each group's arc as long as all it sends, split in order into a part per group it sends to, with `pad` between groups.
pub(crate) fn chords(matrix: &[Vec<f64>], pad: f32) -> (Arcs, Vec<Arcs>) {
    let grand: f64 = matrix.iter().flatten().sum();
    let room = (TAU - pad * matrix.len() as f32).max(0.0);
    let share = |value: f64| match grand > 0.0 {
        true => (value / grand * f64::from(room)) as f32,
        false => 0.0,
    };
    let mut angle = -FRAC_PI_2 + pad / 2.0;
    let (mut groups, mut parts) = (Vec::new(), Vec::new());
    for row in matrix {
        let start = angle;
        parts.push(
            row.iter()
                .map(|value| {
                    let part = (angle, share(*value));
                    angle += part.1;
                    part
                })
                .collect(),
        );
        groups.push((start, angle - start));
        angle += pad;
    }
    (groups, parts)
}

/// A ribbon between two parts of the circle, bowing through the center.
fn ribbon(
    center: Point<Pixels>,
    radius: f32,
    (from, reach): (f32, f32),
    (to, span): (f32, f32),
) -> PathBuilder {
    let arc = |path: &mut PathBuilder, start: f32, sweep: f32| {
        let steps = ((sweep / TAU) * 120.0).ceil().max(1.0) as usize;
        (1..=steps).for_each(|step| {
            path.line_to(toward(
                center,
                radius,
                start + sweep * step as f32 / steps as f32,
            ))
        });
    };
    let mut path = PathBuilder::fill();
    path.move_to(toward(center, radius, from));
    arc(&mut path, from, reach);
    path.curve_to(toward(center, radius, to), center);
    arc(&mut path, to, span);
    path.curve_to(toward(center, radius, from), center);
    path.close();
    path
}

/// A label set outside a circle toward `angle`, leaning away from the center.
fn outside(center: (f32, f32), radius: f32, angle: f32, label: Div) -> Div {
    let (dx, dy) = (angle.cos(), angle.sin());
    div()
        .absolute()
        .left(Pixels::from(center.0 + radius * dx))
        .top(Pixels::from(center.1 + radius * dy))
        .w_0()
        .h_0()
        .flex()
        .map(|anchor| match dx {
            dx if dx > 0.3 => anchor.justify_start(),
            dx if dx < -0.3 => anchor.justify_end(),
            _ => anchor.justify_center(),
        })
        .map(|anchor| match dy {
            dy if dy > 0.3 => anchor.items_start(),
            dy if dy < -0.3 => anchor.items_end(),
            _ => anchor.items_center(),
        })
        .child(label)
}

/// Flows between groups around a circle: each group's arc as long as all it sends, each ribbon as wide at each end as what goes that way. Hover a group to follow its ribbons.
#[derive(IntoElement)]
pub struct ChordDiagram {
    base: Div,
    id: ElementId,
    names: Vec<SharedString>,
    matrix: Vec<Vec<f64>>,
    format: Format,
}

impl ChordDiagram {
    /// Groups and what each sends to each: `matrix[from][to]`.
    pub fn new(
        id: impl Into<ElementId>,
        names: impl IntoIterator<Item = impl Into<SharedString>>,
        matrix: Vec<Vec<f64>>,
    ) -> Self {
        let names: Vec<SharedString> = names.into_iter().map(Into::into).collect();
        assert!(
            matrix.len() == names.len() && matrix.iter().all(|row| row.len() == names.len()),
            "a chord diagram needs a square matrix, a row per group"
        );
        assert!(
            matrix
                .iter()
                .flatten()
                .all(|value| drawable(*value) && *value >= 0.0),
            "flows are zero or more, within charts::LIMIT"
        );
        Self {
            base: div(),
            id: id.into(),
            names,
            matrix,
            format: Rc::new(compact),
        }
    }

    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }
}

impl Styled for ChordDiagram {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for ChordDiagram {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tiles: Entity<Tiles> =
            window.use_keyed_state((self.id.clone(), "chords"), cx, |_, _| Tiles::default());
        let (bounds, hover) = (
            tiles.read(cx).bounds,
            tiles.read(cx).pointed(self.names.len()),
        );
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let (width, height) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let center = (width / 2.0, height / 2.0);
        let outer = (width.min(height) / 2.0 - pixels(sizes.foot)).max(0.0);
        let thick = outer * 0.06;
        let (groups, parts) = chords(&self.matrix, pixels(sizes.stroke) * 2.0 / outer.max(1.0));
        let count = self.names.len();
        let pairs: Vec<(usize, usize)> = (0..count)
            .flat_map(|a| (a..count).map(move |b| (a, b)))
            .filter(|(a, b)| self.matrix[*a][*b] + self.matrix[*b][*a] > 0.0)
            .collect();
        let text = |content: SharedString, strong: bool| {
            div()
                .flex_none()
                .whitespace_nowrap()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(if strong { colors.fg } else { colors.fg_muted })
                .child(content)
        };
        let labels: Vec<Div> = groups
            .iter()
            .zip(&self.names)
            .enumerate()
            .map(|(ix, ((start, sweep), name))| {
                outside(
                    center,
                    outer + pixels(sizes.inset),
                    start + sweep / 2.0,
                    text(name.clone(), hover == Some(ix)),
                )
            })
            .collect();
        let format = self.format.clone();
        let tooltip = hover.map(|ix| {
            let (sends, gets) = (
                self.matrix[ix].iter().sum::<f64>(),
                self.matrix.iter().map(|row| row[ix]).sum::<f64>(),
            );
            let card = ChartTooltip::new(self.names[ix].clone())
                .row(Some(tint(&colors, ix)), "Sends", format(sends))
                .row(None, "Receives", format(gets));
            let middle = groups[ix].0 + groups[ix].1 / 2.0;
            anchored(
                (
                    center.0 + outer * middle.cos(),
                    center.1 + outer * middle.sin(),
                ),
                middle.cos() < 0.0,
                card,
            )
        });
        let senders: Vec<usize> = pairs
            .iter()
            .map(|(a, b)| {
                if self.matrix[*a][*b] >= self.matrix[*b][*a] {
                    *a
                } else {
                    *b
                }
            })
            .collect();
        let (palette, arcs, hit) = (colors.clone(), groups.clone(), groups);
        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());
        tracked(
            root,
            (self.id.clone(), "chord").into(),
            &tiles,
            move |place| {
                let (angle, distance) = bearing(center, place);
                (distance <= outer)
                    .then(|| {
                        hit.iter()
                            .position(|(start, sweep)| angle >= *start && angle < start + sweep)
                    })
                    .flatten()
            },
        )
        .child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    let middle = bounds.center();
                    for ((a, b), sender) in pairs.iter().zip(&senders) {
                        let strength = match hover {
                            None => 0.3,
                            Some(on) if on == *a || on == *b => 0.55,
                            Some(_) => 0.06,
                        };
                        finish(
                            ribbon(middle, outer - thick * 1.5, parts[*a][*b], parts[*b][*a]),
                            tint(&palette, *sender).opacity(strength),
                            window,
                        );
                    }
                    for (ix, arc) in arcs.iter().enumerate() {
                        if let Some(path) = wedge(middle, (outer - thick, outer), *arc, 0.0) {
                            finish(path, tint(&palette, ix), window);
                        }
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
    use std::f32::consts::{FRAC_PI_2, TAU};

    use super::*;

    #[test]
    fn chords_split_each_arc_by_where_it_sends() {
        let (groups, parts) = chords(&[vec![1.0, 3.0], vec![2.0, 2.0]], 0.0);
        let near =
            |(a, b): (f32, f32), (c, d): (f32, f32)| (a - c).abs() < 1e-5 && (b - d).abs() < 1e-5;
        assert!(
            near(groups[0], (-FRAC_PI_2, TAU / 2.0)),
            "half of all that is sent: {:?}",
            groups[0]
        );
        assert!(
            near(parts[0][1], (-FRAC_PI_2 + TAU / 8.0, TAU * 3.0 / 8.0)),
            "{:?}",
            parts[0][1]
        );
        assert!(near((groups[1].0, 0.0), (-FRAC_PI_2 + TAU / 2.0, 0.0)));
    }
}
