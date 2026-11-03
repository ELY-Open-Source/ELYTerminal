use gpui::{
    App, Div, IntoElement, ParentElement, Pixels, Rems, SharedString, Styled, Window, div,
    prelude::*, relative,
};

use super::{
    geometry::{Geometry, Rect},
    plot::Plot,
    scale::Linear,
};
use crate::{
    theme::{ActiveTheme, Radius, TextSize},
    typography::{Ellipsis, LEADING, tabular, text_width},
};

/// Room between a gutter's label and the plot.
pub(crate) const PAD: Rems = Rems(0.5);

/// A label centered under `x`, below the frame.
pub(crate) fn below(x: f32, frame: Rect, label: Div) -> Div {
    div()
        .absolute()
        .left(Pixels::from(x))
        .top(Pixels::from(frame.y + frame.h))
        .w_0()
        .flex()
        .justify_center()
        .pt_1p5()
        .child(label)
}

/// A label centered on `y`, set right in the gutter.
pub(crate) fn beside(y: f32, gutter: Pixels, label: Div) -> Div {
    div()
        .absolute()
        .left_0()
        .top(Pixels::from(y))
        .w(gutter)
        .h_0()
        .flex()
        .items_center()
        .justify_end()
        .pr(PAD)
        .child(label)
}

/// A tooltip beside a place, centered on it, to its right or, flipped, to its left.
pub(crate) fn anchored((x, y): (f32, f32), flip: bool, card: impl IntoElement) -> Div {
    let card = div()
        .flex_none()
        .when(flip, |card| card.mr_3())
        .when(!flip, |card| card.ml_3())
        .child(card);
    div()
        .absolute()
        .left(Pixels::from(x))
        .top(Pixels::from(y))
        .w_0()
        .h_0()
        .flex()
        .items_center()
        .when(flip, |anchor| anchor.justify_end())
        .child(card)
}

/// The words around a plot, and where its rules and notes run.
pub(crate) struct Marks {
    pub labels: Vec<Div>,
    pub rules: Vec<f32>,
    pub notes: Vec<f32>,
}

/// Value ticks; category names, thinned to fit, or a scatter's x ticks; and the labels of rules and notes.
pub(crate) fn marks(
    plot: &Plot,
    geometry: &Geometry,
    (rect, span): (Rect, (usize, usize)),
    window: &Window,
    cx: &App,
) -> Marks {
    let theme = cx.theme();
    let (colors, sizes, rem) = (&theme.colors, theme.chart(), window.rem_size());
    let text = |content: SharedString| {
        div()
            .flex_none()
            .whitespace_nowrap()
            .text_size(theme.text_size(TextSize::Xs))
            .text_color(colors.fg_subtle)
            .child(content)
    };
    let mut labels: Vec<Div> = geometry
        .ticks
        .iter()
        .map(|(at, value)| {
            let label = tabular(text((plot.format)(*value).into()));
            if plot.horizontal {
                below(*at, rect, label)
            } else {
                beside(*at, Pixels::from(rect.x), label)
            }
        })
        .collect();
    let along = if plot.horizontal { rect.h } else { rect.w };
    let room = f32::from(sizes.label.to_pixels(rem));
    let named: Vec<&(f32, SharedString)> = geometry
        .labels
        .iter()
        .filter(|(_, label)| !label.is_empty())
        .collect();
    let every = if plot.horizontal {
        1
    } else {
        ((named.len() as f32 * room / along.max(1.0)).ceil() as usize).max(1)
    };
    labels.extend(
        named
            .iter()
            .enumerate()
            .filter(|(ix, _)| ix % every == 0)
            .map(|(_, (at, label))| {
                if plot.horizontal {
                    let name = div()
                        .debug_selector(|| "chart-band".into())
                        .min_w_0()
                        .text_right()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(colors.fg_subtle)
                        .child(Ellipsis::new(label.clone()));
                    beside(*at, Pixels::from(rect.x), name)
                } else {
                    below(*at, rect, text(label.clone()))
                }
            }),
    );
    let place_value = |value: f64| {
        let (first, last) = (geometry.ticks.first()?, geometry.ticks.last()?);
        Some(Linear::new((first.1, last.1), (first.0, last.0)).at(value))
    };
    let rules: Vec<(f32, SharedString)> = plot
        .rules
        .iter()
        .filter_map(|(value, label)| Some((place_value(*value)?, label.clone())))
        .collect();
    labels.extend(rules.iter().map(|(at, label)| {
        if plot.horizontal {
            div()
                .absolute()
                .left(Pixels::from(*at))
                .top(Pixels::from(rect.y))
                .pl_1p5()
                .child(text(label.clone()))
        } else {
            div()
                .absolute()
                .left(Pixels::from(rect.x))
                .w(Pixels::from(rect.w))
                .top(Pixels::from(*at))
                .h_0()
                .flex()
                .justify_end()
                .items_end()
                .child(text(label.clone()).pb_0p5())
        }
    }));
    let mut notes: Vec<(f32, SharedString)> = plot
        .notes
        .iter()
        .filter(|(ix, _)| !plot.horizontal && (span.0..=span.1).contains(ix))
        .filter_map(|(ix, label)| {
            geometry
                .labels
                .get(ix - span.0)
                .map(|(at, _)| (*at, label.clone()))
        })
        .collect();
    notes.sort_by(|a, b| a.0.total_cmp(&b.0));
    let size = theme.text_size(TextSize::Xs).to_pixels(rem);
    let mut ends: Vec<f32> = Vec::new();
    for (at, label) in &notes {
        let row = ends.iter().position(|end| end <= at).unwrap_or(ends.len());
        let end = at + f32::from(text_width(label, size, window) + size);
        if row == ends.len() {
            ends.push(end);
        } else {
            ends[row] = end;
        }
        labels.push(
            div()
                .absolute()
                .left(Pixels::from(*at))
                .top(Pixels::from(
                    rect.y + f32::from(size) * LEADING * row as f32,
                ))
                .pl_1p5()
                .line_height(relative(LEADING))
                .child(
                    text(label.clone())
                        .px_1()
                        .rounded(theme.radius(Radius::Sm))
                        .bg(colors.bg),
                ),
        );
    }
    Marks {
        labels,
        rules: rules.iter().map(|(at, _)| *at).collect(),
        notes: notes.iter().map(|(at, _)| *at).collect(),
    }
}
