use std::f32::consts::{FRAC_PI_2, TAU};

use gpui::{
    App, Div, ElementId, Entity, HoverListenerMode, Hsla, InteractiveElement, IntoElement,
    MouseMoveEvent, ParentElement, Pixels, Refineable, RenderOnce, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, canvas, div,
};

use super::{
    paint::{finish, measure, tint},
    pie::{bearing, wedge},
    scale::compact,
    series::drawable,
    tiles::Tiles,
};
use crate::{
    theme::{ActiveTheme, TextSize},
    typography::{format, tabular},
};

/// A node of a sunburst: its name, its own value, and the nodes inside it.
#[derive(Clone, Debug, PartialEq)]
pub struct Slice {
    name: SharedString,
    value: f64,
    children: Vec<Slice>,
}

impl Slice {
    pub fn new(name: impl Into<SharedString>, value: f64) -> Self {
        assert!(
            drawable(value) && value >= 0.0,
            "a slice needs a value of zero or more, within charts::LIMIT"
        );
        Self {
            name: name.into(),
            value,
            children: Vec::new(),
        }
    }

    pub fn children(mut self, children: impl IntoIterator<Item = Slice>) -> Self {
        self.children.extend(children);
        self
    }

    /// Its value, or its children's total when that is larger.
    fn weight(&self) -> f64 {
        self.value
            .max(self.children.iter().map(Slice::weight).sum())
    }
}

/// An arc of a sunburst: its ring, start and sweep, the top branch it belongs to, and its path of names.
#[derive(Clone, Debug, PartialEq)]
struct Arc {
    depth: usize,
    start: f32,
    sweep: f32,
    branch: usize,
    path: Vec<SharedString>,
    weight: f64,
}

/// Arcs for a hierarchy, each child inside its parent's sweep.
fn rings(slices: &[Slice]) -> Vec<Arc> {
    fn walk(
        slices: &[Slice],
        (depth, start, sweep, whole): (usize, f32, f32, f64),
        branch: Option<usize>,
        trail: &[SharedString],
        out: &mut Vec<Arc>,
    ) {
        let mut at = start;
        for (ix, slice) in slices.iter().enumerate() {
            let part = if whole > 0.0 {
                (slice.weight() / whole) as f32 * sweep
            } else {
                0.0
            };
            let (branch, mut path) = (branch.unwrap_or(ix), trail.to_vec());
            path.push(slice.name.clone());
            walk(
                &slice.children,
                (depth + 1, at, part, slice.weight()),
                Some(branch),
                &path,
                out,
            );
            out.push(Arc {
                depth,
                start: at,
                sweep: part,
                branch,
                path,
                weight: slice.weight(),
            });
            at += part;
        }
    }
    let mut out = Vec::new();
    let whole = slices.iter().map(Slice::weight).sum();
    walk(slices, (0, -FRAC_PI_2, TAU, whole), None, &[], &mut out);
    out.sort_by_key(|arc| arc.depth);
    out
}

/// Parts of parts as rings around a center that holds the whole, each arc as wide as its share. Hover reads an arc's path and share.
#[derive(IntoElement)]
pub struct Sunburst {
    base: Div,
    id: ElementId,
    slices: Vec<Slice>,
}

impl Sunburst {
    pub fn new(id: impl Into<ElementId>, slices: impl IntoIterator<Item = Slice>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            slices: slices.into_iter().collect(),
        }
    }
}

impl Styled for Sunburst {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Sunburst {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ring: Entity<Tiles> =
            window.use_keyed_state((self.id.clone(), "ring"), cx, |_, _| Tiles::default());
        let arcs = rings(&self.slices);
        let depths = arcs.iter().map(|arc| arc.depth + 1).max().unwrap_or(1);
        let total: f64 = self.slices.iter().map(Slice::weight).sum();
        let hover = ring.read(cx).hover.filter(|ix| *ix < arcs.len());
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let bounds = ring.read(cx).bounds;
        let outer =
            (f32::from(bounds.size.width).min(f32::from(bounds.size.height)) / 2.0).max(0.0);
        let hole = outer * 0.34;
        let band = (outer - hole) / depths as f32;
        let gap = f32::from(sizes.stroke.to_pixels(rem));
        let color = |arc: &Arc, lit: bool| {
            let fade = if lit {
                1.0
            } else {
                (1.0 - 0.22 * arc.depth as f32).max(0.4)
            };
            tint(&colors, arc.branch).opacity(fade)
        };
        let fills: Vec<Hsla> = arcs
            .iter()
            .enumerate()
            .map(|(ix, arc)| color(arc, hover == Some(ix)))
            .collect();
        let (big, small) = match hover.map(|ix| &arcs[ix]) {
            Some(arc) => (
                format::percent(if total > 0.0 { arc.weight / total } else { 0.0 }, 1, false),
                arc.path.join(" › "),
            ),
            None => (compact(total), "Total".to_string()),
        };
        let middle = div()
            .absolute()
            .inset_0()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .child(
                tabular(div())
                    .text_size(theme.text_size(TextSize::Lg))
                    .text_color(colors.fg)
                    .child(big),
            )
            .child(
                div()
                    .max_w(Pixels::from(hole * 1.6))
                    .truncate()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.fg_muted)
                    .child(small),
            );
        let (moved, left, measured, hit) = (ring.clone(), ring.clone(), ring.clone(), arcs.clone());
        let painted = arcs.clone();
        let mut frame = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        frame.style().refine(self.base.style());
        frame
            .id((self.id.clone(), "ring"))
            .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                let center = moved.read(cx).bounds.center();
                let at = (f32::from(event.position.x), f32::from(event.position.y));
                let (angle, distance) = bearing((f32::from(center.x), f32::from(center.y)), at);
                let depth = ((distance - hole) / band).floor();
                let next = (depth >= 0.0 && distance <= outer).then(|| {
                    hit.iter().position(|arc| {
                        arc.depth == depth as usize
                            && angle >= arc.start
                            && angle < arc.start + arc.sweep
                    })
                });
                moved.update(cx, |ring, cx| {
                    let next = next.flatten();
                    if ring.hover != next {
                        ring.hover = next;
                        cx.notify();
                    }
                })
            })
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |inside, _, cx| {
                if !*inside {
                    left.update(cx, |ring, cx| {
                        ring.hover = None;
                        cx.notify();
                    })
                }
            })
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let center = bounds.center();
                        for (ix, arc) in painted.iter().enumerate() {
                            let from = hole + band * arc.depth as f32;
                            if let Some(path) = wedge(
                                center,
                                (from, from + band - gap),
                                (arc.start, arc.sweep),
                                gap,
                            ) {
                                finish(path, fills[ix], window);
                            }
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .child(middle)
            .child(measure(measured, |ring| &mut ring.bounds))
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_PI_2, TAU};

    use super::*;

    #[test]
    fn a_child_takes_only_its_share_of_its_parent() {
        let arcs = rings(&[Slice::new("Whole", 100.0).children([Slice::new("Part", 25.0)])]);
        let part = arcs.iter().find(|arc| arc.depth == 1).expect("the part");
        assert!(
            (part.sweep - TAU / 4.0).abs() < 1e-5,
            "a quarter of the ring, got {}",
            part.sweep
        );
    }

    #[test]
    fn rings_nest_children_inside_their_parent() {
        let tree = [
            Slice::new("Design", 0.0).children([Slice::new("Brand", 1.0), Slice::new("Web", 3.0)]),
            Slice::new("Ops", 4.0),
        ];
        let arcs = rings(&tree);
        assert_eq!(arcs.len(), 4);
        assert_eq!(
            (arcs[0].depth, arcs[0].sweep, arcs[0].weight),
            (0, TAU / 2.0, 4.0),
            "Design weighs its children"
        );
        let web = arcs
            .iter()
            .find(|arc| arc.path.last() == Some(&"Web".into()))
            .expect("Web");
        assert_eq!(
            (web.depth, web.start, web.sweep, web.branch),
            (1, -FRAC_PI_2 + TAU / 8.0, TAU * 3.0 / 8.0, 0)
        );
    }
}
