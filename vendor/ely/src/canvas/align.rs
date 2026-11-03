use std::rc::Rc;

use gpui::{App, ElementId, IntoElement, ParentElement, RenderOnce, Window};

use super::view::Frame;
use crate::{
    buttons::IconButton,
    primitives::IconName,
    shell::{Toolbar, ToolbarGroup, ToolbarSeparator},
};

type OnAlign = Rc<dyn Fn(Align, &mut Window, &mut App)>;

/// A way to line up selected shapes, or to spread them so the gaps between them match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
    Top,
    Middle,
    Bottom,
    SpreadAcross,
    SpreadDown,
}

impl Align {
    pub const ALL: [Align; 8] = [
        Align::Left,
        Align::Center,
        Align::Right,
        Align::Top,
        Align::Middle,
        Align::Bottom,
        Align::SpreadAcross,
        Align::SpreadDown,
    ];

    pub fn words(self) -> &'static str {
        match self {
            Align::Left => "Align left edges",
            Align::Center => "Align centers",
            Align::Right => "Align right edges",
            Align::Top => "Align top edges",
            Align::Middle => "Align middles",
            Align::Bottom => "Align bottom edges",
            Align::SpreadAcross => "Spread across",
            Align::SpreadDown => "Spread down",
        }
    }

    fn icon(self) -> IconName {
        match self {
            Align::Left => IconName::AlignStartVertical,
            Align::Center => IconName::AlignCenterVertical,
            Align::Right => IconName::AlignEndVertical,
            Align::Top => IconName::AlignStartHorizontal,
            Align::Middle => IconName::AlignCenterHorizontal,
            Align::Bottom => IconName::AlignEndHorizontal,
            Align::SpreadAcross => IconName::AlignHorizontalDistributeCenter,
            Align::SpreadDown => IconName::AlignVerticalDistributeCenter,
        }
    }

    /// The fewest shapes it moves.
    pub fn least(self) -> usize {
        match self {
            Align::SpreadAcross | Align::SpreadDown => 3,
            _ => 2,
        }
    }
}

/// `frames` lined up on the edge or center line they span together, or spread so the gaps between them match while the first and last stay.
pub fn aligned(frames: &[Frame], align: Align) -> Vec<Frame> {
    assert!(
        frames.len() >= align.least(),
        "{} takes {} shapes, not {}",
        align.words(),
        align.least(),
        frames.len()
    );
    let whole = frames
        .iter()
        .copied()
        .reduce(|whole, frame| whole.union(&frame))
        .expect("frames to align");
    let at = |frame: &Frame, x: f32, y: f32| Frame::new(x, y, frame.w, frame.h);
    let each = |place: &dyn Fn(&Frame) -> Frame| frames.iter().map(place).collect::<Vec<_>>();
    match align {
        Align::Left => each(&|f| at(f, whole.x, f.y)),
        Align::Center => each(&|f| at(f, whole.x + (whole.w - f.w) / 2.0, f.y)),
        Align::Right => each(&|f| at(f, whole.right() - f.w, f.y)),
        Align::Top => each(&|f| at(f, f.x, whole.y)),
        Align::Middle => each(&|f| at(f, f.x, whole.y + (whole.h - f.h) / 2.0)),
        Align::Bottom => each(&|f| at(f, f.x, whole.bottom() - f.h)),
        Align::SpreadAcross => spread(frames, |f| (f.x, f.w), |f, x| at(f, x, f.y)),
        Align::SpreadDown => spread(frames, |f| (f.y, f.h), |f, y| at(f, f.x, y)),
    }
}

/// Frames placed along one axis in the order they start, the gaps between them equal.
fn spread(
    frames: &[Frame],
    span: impl Fn(&Frame) -> (f32, f32),
    to: impl Fn(&Frame, f32) -> Frame,
) -> Vec<Frame> {
    let mut order: Vec<usize> = (0..frames.len()).collect();
    order.sort_by(|a, b| span(&frames[*a]).0.total_cmp(&span(&frames[*b]).0));
    let first = span(&frames[order[0]]);
    let last = span(&frames[order[order.len() - 1]]);
    let used: f32 = frames.iter().map(|frame| span(frame).1).sum();
    let gap = (last.0 + last.1 - first.0 - used) / (frames.len() - 1) as f32;
    let mut out = frames.to_vec();
    let mut next = first.0;
    for ix in order {
        out[ix] = to(&frames[ix], next);
        next += span(&frames[ix]).1 + gap;
    }
    out
}

/// Buttons that line up or spread the selection. Each asks the owner, who moves the shapes with `aligned`; a way rests disabled until enough shapes are selected for it.
#[derive(IntoElement)]
pub struct AlignmentToolbar {
    id: ElementId,
    count: usize,
    on_align: Option<OnAlign>,
}

impl AlignmentToolbar {
    /// `count` shapes are selected.
    pub fn new(id: impl Into<ElementId>, count: usize) -> Self {
        Self {
            id: id.into(),
            count,
            on_align: None,
        }
    }

    pub fn on_align(mut self, handler: impl Fn(Align, &mut Window, &mut App) + 'static) -> Self {
        self.on_align = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for AlignmentToolbar {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let button = |align: Align| {
            let on_align = self.on_align.clone();
            IconButton::new((self.id.clone(), align.words()), align.icon())
                .tooltip(align.words())
                .disabled(self.count < align.least())
                .on_click(move |_, window, cx| {
                    log::info!("alignment toolbar: {}", align.words());
                    if let Some(on_align) = &on_align {
                        on_align(align, window, cx);
                    }
                })
        };
        let (lines, spreads) = Align::ALL.split_at(6);
        Toolbar::new()
            .child(ToolbarGroup::new().children(lines.iter().map(|align| button(*align))))
            .child(ToolbarSeparator)
            .child(ToolbarGroup::new().children(spreads.iter().map(|align| button(*align))))
    }
}

#[cfg(test)]
mod tests {
    use super::{Align, aligned};
    use crate::canvas::Frame;

    fn three() -> Vec<Frame> {
        vec![
            Frame::new(0.0, 10.0, 20.0, 20.0),
            Frame::new(100.0, 0.0, 40.0, 60.0),
            Frame::new(40.0, 30.0, 10.0, 10.0),
        ]
    }

    #[test]
    fn a_line_up_meets_the_edge_or_center_the_frames_span() {
        let xs = |align| {
            aligned(&three(), align)
                .iter()
                .map(|f| f.x)
                .collect::<Vec<_>>()
        };
        assert_eq!(xs(Align::Left), [0.0, 0.0, 0.0]);
        assert_eq!(xs(Align::Right), [120.0, 100.0, 130.0]);
        assert_eq!(xs(Align::Center), [60.0, 50.0, 65.0]);
        let ys = |align| {
            aligned(&three(), align)
                .iter()
                .map(|f| f.y)
                .collect::<Vec<_>>()
        };
        assert_eq!(ys(Align::Top), [0.0, 0.0, 0.0]);
        assert_eq!(ys(Align::Middle), [20.0, 0.0, 25.0]);
        assert_eq!(ys(Align::Bottom), [40.0, 0.0, 50.0]);
    }

    #[test]
    fn a_spread_keeps_the_ends_and_evens_the_gaps() {
        let spread = aligned(&three(), Align::SpreadAcross);
        assert_eq!(spread[0].x, 0.0, "the first stays");
        assert_eq!(spread[1].x, 100.0, "the last stays");
        assert_eq!(spread[2].x, 55.0, "35 either side of the middle one");
        assert_eq!(
            spread.iter().map(|f| f.y).collect::<Vec<_>>(),
            [10.0, 0.0, 30.0],
            "a spread across leaves heights alone"
        );
    }

    #[test]
    #[should_panic(expected = "takes 3 shapes, not 2")]
    fn a_spread_takes_three() {
        aligned(&three()[..2], Align::SpreadDown);
    }
}
