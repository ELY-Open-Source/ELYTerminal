use std::{collections::HashSet, rc::Rc};

use gpui::{
    App, Bounds, Div, ElementId, Entity, InteractiveElement, ParentElement, Pixels, Rems,
    SharedString, Styled, Window, div,
};

use super::{
    axes::{PAD, anchored, marks},
    geometry::{Geometry, Ink, Rect, frame},
    glide::Glide,
    paint::{Pen, measure, tint},
    parts::{ChartLegend, ChartTooltip},
    pointer::{Focus, answer, drawing, focus},
    scale::compact,
};
use crate::{
    buttons::{Button, ButtonVariant},
    motion,
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
    typography::text_width,
};

pub(crate) type Format = Rc<dyn Fn(f64) -> String>;
/// Lays out a plot's marks inside its frame.
pub(crate) type Layout = Rc<dyn Fn(&Scene) -> Geometry>;
/// A picked index's tooltip: its title, then each row's dot if any, name and value.
pub(crate) type Tips =
    Rc<dyn Fn(usize, &Scene) -> (SharedString, Vec<(Option<Ink>, SharedString, String)>)>;

/// What a layout lays out: the frame, the values at this moment of a glide, the series the legend hid, and the zoomed span.
pub(crate) struct Scene<'a> {
    pub frame: Rect,
    pub values: &'a [Vec<f64>],
    pub hidden: &'a HashSet<SharedString>,
    pub span: (usize, usize),
}

/// What the pointer picks in a plot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pick {
    /// A category's band, as bars take.
    Band,
    /// The nearest category, as lines take.
    Point,
    /// The nearest dot within reach, as a scatter takes.
    Dot,
}

/// What a plot draws and how it answers the pointer.
pub(crate) struct Plot {
    pub id: ElementId,
    pub labels: Vec<SharedString>,
    /// The series the legend lists and can hide; one or none shows no legend.
    pub names: Vec<SharedString>,
    /// Values that glide when the owner changes them; the layout reads them from its scene.
    pub values: Vec<Vec<f64>>,
    pub layout: Layout,
    pub tips: Tips,
    pub pick: Pick,
    pub horizontal: bool,
    pub smooth: bool,
    pub rules: Vec<(f64, SharedString)>,
    pub notes: Vec<(usize, SharedString)>,
    pub zoom: bool,
    pub format: Format,
}

impl Plot {
    pub(crate) fn new(id: ElementId, pick: Pick, layout: Layout, tips: Tips) -> Self {
        Self {
            id,
            labels: Vec::new(),
            names: Vec::new(),
            values: Vec::new(),
            layout,
            tips,
            pick,
            horizontal: false,
            smooth: false,
            rules: Vec::new(),
            notes: Vec::new(),
            zoom: false,
            format: Rc::new(compact),
        }
    }
}

/// A plot's own state: its box, what the pointer picks and where it is, the series the legend hid, the zoomed span, a brush in progress, and a glide.
#[derive(Default)]
pub(crate) struct State {
    pub bounds: Bounds<Pixels>,
    pub hover: Option<usize>,
    pub pointer: (f32, f32),
    pub hidden: HashSet<SharedString>,
    pub span: Option<(usize, usize)>,
    pub brush: Option<(f32, f32)>,
    glide: Glide,
}

/// Draws a plot: legend, grid, marks and axes, crosshair and tooltip, rules and notes, and a brush that zooms.
pub(crate) fn plot(plot: Plot, base: Div, window: &mut Window, cx: &mut App) -> Div {
    let id = plot.id.clone();
    let state: Entity<State> =
        window.use_keyed_state((id.clone(), "plot"), cx, |_, _| State::default());
    let length = motion::duration(motion::SLOW, cx);
    let (values, moving) = state.update(cx, |state, _| state.glide.follow(&plot.values, length));
    if moving {
        window.request_animation_frame();
    }
    let (bounds, hover, pointer, hidden, brush, zoomed) = {
        let state = state.read(cx);
        let held = (state.hidden.clone(), state.brush, state.span);
        (
            state.bounds,
            state.hover,
            state.pointer,
            held.0,
            held.1,
            held.2,
        )
    };
    let theme = cx.theme();
    let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
    let pixels = |length: Rems| f32::from(length.to_pixels(rem));
    let count = plot.labels.len();
    let span = zoomed
        .filter(|(_, to)| *to < count)
        .unwrap_or((0, count.saturating_sub(1)));
    let box_size = (f32::from(bounds.size.width), f32::from(bounds.size.height));
    let gutter = if plot.horizontal {
        let size = theme.text_size(TextSize::Xs).to_pixels(rem);
        let widest = plot
            .labels
            .iter()
            .map(|label| f32::from(text_width(label, size, window)))
            .fold(0.0, f32::max);
        (widest + pixels(PAD))
            .max(pixels(sizes.gutter))
            .min(box_size.0 / 2.0)
    } else {
        pixels(sizes.gutter)
    };
    let rect = frame(box_size, gutter, pixels(sizes.foot), pixels(sizes.inset));
    let scene = Scene {
        frame: rect,
        values: &values,
        hidden: &hidden,
        span,
    };
    let drawn = rect.w > 0.0 && rect.h > 0.0 && (count > 0 || plot.pick == Pick::Dot);
    let geometry = Rc::new(if drawn {
        (plot.layout)(&scene)
    } else {
        Geometry::default()
    });
    let hover =
        hover.filter(|ix| drawn && (plot.pick == Pick::Dot || (span.0..=span.1).contains(ix)));
    let marks = marks(&plot, &geometry, (rect, span), window, cx);
    let lit = focus(&plot, &geometry, hover, (rect, span));
    let tooltip = hover.zip(lit.as_ref()).map(|(ix, lit)| {
        let (x, y) = match lit {
            Focus::Dot(dot) => dot.center,
            Focus::Band(band) if plot.horizontal => (pointer.0, band.y + band.h / 2.0),
            Focus::Band(band) => (band.x + band.w / 2.0, pointer.1),
            Focus::Line(x, _) => (*x, pointer.1),
        };
        let (title, rows) = (plot.tips)(ix, &scene);
        let card = rows
            .into_iter()
            .fold(ChartTooltip::new(title), |card, (ink, name, value)| {
                card.row(ink.map(|ink| ink.color(&colors)), name, value)
            });
        anchored((x, y), x > box_size.0 * 0.6, card)
    });
    let legend = (plot.names.len() > 1).then(|| {
        let toggled = state.clone();
        let legend = plot.names.iter().enumerate().fold(
            ChartLegend::new((id.clone(), "legend")),
            |legend, (ix, name)| {
                legend.entry(tint(&colors, ix), name.clone(), hidden.contains(name))
            },
        );
        legend.on_toggle(move |name, _, cx| {
            toggled.update(cx, |state, cx| {
                if !state.hidden.remove(name) {
                    state.hidden.insert(name.clone());
                }
                let now = if state.hidden.contains(name) {
                    "hidden"
                } else {
                    "shown"
                };
                log::info!("chart: {name} {now}");
                cx.notify();
            })
        })
    });
    let zoomable = plot.zoom && !plot.horizontal && plot.pick != Pick::Dot && count >= 3;
    let show_all = zoomed.filter(|_| zoomable).map(|_| {
        let reset = state.clone();
        let button = Button::new((id.clone(), "show-all"), "Show all")
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Sm);
        div()
            .absolute()
            .top_0()
            .right_0()
            .child(button.on_click(move |_, _, cx| {
                reset.update(cx, |state, cx| {
                    state.span = None;
                    log::info!("chart: zoom reset");
                    cx.notify();
                })
            }))
    });
    let pen = Pen {
        colors: colors.clone(),
        stroke: sizes.stroke.to_pixels(rem),
        hairline: sizes.hairline,
        corner: theme.radius(Radius::Sm).to_pixels(rem),
        smooth: plot.smooth,
        horizontal: plot.horizontal,
    };
    let root = div().id((id.clone(), "plot")).relative().flex_1().min_h_0();
    let ways = (plot.pick, plot.horizontal, zoomable);
    let root = answer(
        root,
        &state,
        ways,
        (rect, span, pixels(sizes.inset)),
        geometry.clone(),
    )
    .child(drawing(
        geometry,
        pen,
        rect,
        (lit, brush),
        (marks.rules, marks.notes),
    ))
    .children(marks.labels)
    .children(tooltip)
    .children(show_all)
    .child(measure(state, |state| &mut state.bounds));
    base.flex().flex_col().gap_2().children(legend).child(root)
}
