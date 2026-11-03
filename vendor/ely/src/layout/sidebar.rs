use gpui::{
    Animation, AnimationExt, AnyElement, App, ElementId, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, Styled, Window, div,
};
use smallvec::SmallVec;
use web_time::Instant;

use crate::{motion, theme::ActiveTheme};

struct Width {
    collapsed: bool,
    generation: u64,
    changed: Instant,
}

/// Side column that folds to an icon rail, width animated.
#[derive(IntoElement)]
pub struct Sidebar {
    id: ElementId,
    collapsed: bool,
    body: SmallVec<[AnyElement; 4]>,
}

impl Sidebar {
    /// Children should render their own compact form when `collapsed`.
    pub fn new(id: impl Into<ElementId>, collapsed: bool) -> Self {
        Self {
            id: id.into(),
            collapsed,
            body: SmallVec::new(),
        }
    }
}

impl ParentElement for Sidebar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl RenderOnce for Sidebar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let collapsed = self.collapsed;
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| Width {
            collapsed,
            generation: 0,
            changed: Instant::now(),
        });
        if state.read(cx).collapsed != collapsed {
            state.update(cx, |width, _| {
                width.collapsed = collapsed;
                width.generation += 1;
                width.changed = Instant::now();
            });
        }
        let (generation, changed) = {
            let width = state.read(cx);
            (width.generation, width.changed)
        };
        let theme = cx.theme();
        let (wide, narrow) = (theme.sidebar_width(false), theme.sidebar_width(true));
        let duration = motion::duration(motion::BASE, cx);
        let column = div()
            .id(self.id)
            .flex()
            .flex_col()
            .flex_none()
            .h_full()
            .overflow_hidden()
            .border_r_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.bg)
            .children(self.body);
        if generation == 0 || changed.elapsed() >= duration {
            return column
                .w(if collapsed { narrow } else { wide })
                .into_any_element();
        }
        column
            .with_animation(
                ("width", generation),
                Animation::new(duration).with_easing(motion::ease_in_out_cubic),
                move |column, t| {
                    let (from, to) = if collapsed {
                        (wide, narrow)
                    } else {
                        (narrow, wide)
                    };
                    column.w(from + (to - from) * t)
                },
            )
            .into_any_element()
    }
}
