use gpui::{
    AnyElement, App, Div, ElementId, IntoElement, ParentElement, Pixels, RenderOnce,
    StyleRefinement, Styled, Window, canvas, div,
};
use smallvec::SmallVec;

/// Columns for `width`, each at least `min`, `gap` apart.
pub(crate) fn columns_for(width: Pixels, min: Pixels, gap: Pixels) -> u16 {
    let fit = ((width + gap) / (min + gap)).floor();
    fit.clamp(1.0, f32::from(u16::MAX)) as u16
}

/// How many items `item` wide with `gap` between fit across `width`, at least one, and the inset that centers them.
pub(crate) fn fit(width: Pixels, item: Pixels, gap: Pixels) -> (usize, Pixels) {
    let count = columns_for(width, item, gap) as usize;
    let used = (item + gap) * count as f32 - gap;
    (count, ((width - used) / 2.0).max(Pixels::ZERO))
}

/// Index of the shortest column; ties go left.
fn shortest(heights: &[Pixels]) -> usize {
    heights
        .iter()
        .enumerate()
        .min_by(|a, b| f32::from(*a.1).total_cmp(&f32::from(*b.1)))
        .map(|(ix, _)| ix)
        .expect("at least one column")
}

fn measure_width(state: gpui::Entity<Pixels>) -> impl IntoElement {
    canvas(
        move |bounds, window, cx| {
            if *state.read(cx) != bounds.size.width {
                state.update(cx, |width, cx| {
                    *width = bounds.size.width;
                    cx.notify();
                    window.request_animation_frame();
                });
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// Equal columns: as many as fit at `min` width each.
#[derive(IntoElement)]
pub struct SimpleGrid {
    id: ElementId,
    base: Div,
    min: Pixels,
    gap: Pixels,
    children: SmallVec<[AnyElement; 8]>,
}

impl SimpleGrid {
    pub fn new(id: impl Into<ElementId>, min: Pixels, gap: Pixels) -> Self {
        assert!(min > Pixels::ZERO, "simple grid needs a positive min width");
        Self {
            id: id.into(),
            base: div(),
            min,
            gap,
            children: SmallVec::new(),
        }
    }
}

impl Styled for SimpleGrid {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for SimpleGrid {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for SimpleGrid {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let width = window.use_keyed_state(self.id, cx, |_, _| Pixels::ZERO);
        let columns = columns_for(*width.read(cx), self.min, self.gap);
        self.base
            .relative()
            .grid()
            .grid_cols(columns)
            .gap(self.gap)
            .children(self.children)
            .child(measure_width(width))
    }
}

/// Columns fill shortest first, so uneven items pack tight.
#[derive(IntoElement)]
pub struct Masonry {
    id: ElementId,
    columns: usize,
    gap: Pixels,
    children: Vec<AnyElement>,
}

impl Masonry {
    pub fn new(id: impl Into<ElementId>, columns: usize, gap: Pixels) -> Self {
        assert!(columns > 0, "masonry needs a column");
        Self {
            id: id.into(),
            columns,
            gap,
            children: Vec::new(),
        }
    }
}

impl ParentElement for Masonry {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Masonry {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let count = self.children.len();
        let heights = window.use_keyed_state(self.id, cx, |_, _| Vec::<Pixels>::new());
        if heights.read(cx).len() != count {
            heights.update(cx, |heights, _| heights.resize(count, Pixels::ZERO));
        }
        let known = heights.read(cx).clone();
        let mut columns: Vec<Vec<AnyElement>> = (0..self.columns).map(|_| Vec::new()).collect();
        let mut filled = vec![Pixels::ZERO; self.columns];
        for (ix, child) in self.children.into_iter().enumerate() {
            let column = shortest(&filled);
            filled[column] += known[ix] + self.gap;
            let state = heights.clone();
            columns[column].push(
                div()
                    .relative()
                    .child(child)
                    .child(
                        canvas(
                            move |bounds, window, cx| {
                                if state.read(cx)[ix] != bounds.size.height {
                                    state.update(cx, |heights, cx| {
                                        heights[ix] = bounds.size.height;
                                        cx.notify();
                                        window.request_animation_frame();
                                    });
                                }
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full(),
                    )
                    .into_any_element(),
            );
        }
        div()
            .flex()
            .items_start()
            .gap(self.gap)
            .children(columns.into_iter().map(|items| {
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(self.gap)
                    .children(items)
            }))
    }
}

#[cfg(test)]
mod tests {
    use gpui::px;

    use super::{columns_for, fit, shortest};

    #[test]
    fn a_nan_height_never_stops_the_shortest_column() {
        assert_eq!(shortest(&[px(f32::NAN), px(4.0), px(2.0)]), 2);
    }

    #[test]
    fn what_fits_sits_centered() {
        assert_eq!(fit(px(100.0), px(3.0), px(2.0)), (20, px(1.0)));
        assert_eq!(fit(px(98.0), px(3.0), px(2.0)), (20, px(0.0)));
        assert_eq!(fit(px(2.0), px(3.0), px(2.0)), (1, px(0.0)));
    }

    #[test]
    fn columns_fit_the_width() {
        assert_eq!(columns_for(px(1000.0), px(220.0), px(16.0)), 4);
        assert_eq!(columns_for(px(100.0), px(220.0), px(16.0)), 1);
        assert_eq!(columns_for(px(0.0), px(220.0), px(16.0)), 1);
    }

    #[test]
    fn shortest_prefers_the_left_on_ties() {
        assert_eq!(shortest(&[px(3.0), px(1.0), px(1.0)]), 1);
        assert_eq!(shortest(&[px(0.0), px(0.0)]), 0);
    }
}
