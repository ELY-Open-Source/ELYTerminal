use std::{rc::Rc, time::Duration};

use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, Pixels, RenderOnce, ScrollHandle,
    Styled, Window, canvas, div, point, prelude::*,
};
use web_time::Instant;

use crate::{
    buttons::{ButtonVariant, IconButton},
    motion,
    primitives::IconName,
    theme::{ActiveTheme, Elevation},
};

const FRAME: Duration = Duration::from_millis(8);

/// Soft shade at each edge where content continues.
#[derive(IntoElement)]
pub struct ScrollShadow {
    handle: ScrollHandle,
}

impl ScrollShadow {
    pub fn new(handle: &ScrollHandle) -> Self {
        Self {
            handle: handle.clone(),
        }
    }
}

impl RenderOnce for ScrollShadow {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let scrolled = -self.handle.offset().y;
        let reach = self.handle.max_offset().y;
        let shade = cx.theme().elevation(Elevation::Floating);
        let edge = || {
            div()
                .absolute()
                .left_0()
                .right_0()
                .h_0()
                .shadow(shade.clone())
        };
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .overflow_hidden()
            .when(scrolled > Pixels::ZERO, |layer| layer.child(edge().top_0()))
            .when(scrolled < reach, |layer| layer.child(edge().bottom_0()))
    }
}

/// Floating button that eases back to the top once the reader has gone far.
#[derive(IntoElement)]
pub struct ScrollToTop {
    id: ElementId,
    handle: ScrollHandle,
}

impl ScrollToTop {
    pub fn new(id: impl Into<ElementId>, handle: &ScrollHandle) -> Self {
        Self {
            id: id.into(),
            handle: handle.clone(),
        }
    }
}

impl RenderOnce for ScrollToTop {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let scrolled = -self.handle.offset().y;
        let view = self.handle.bounds().size.height;
        if view <= Pixels::ZERO || scrolled < view / 2.0 {
            return div().into_any_element();
        }
        let handle = self.handle;
        let duration = motion::duration(motion::SLOW, cx);
        div()
            .absolute()
            .bottom_4()
            .right_4()
            .rounded_full()
            .bg(cx.theme().colors.surface)
            .shadow(cx.theme().elevation(Elevation::Floating))
            .child(
                IconButton::new(self.id, IconName::ArrowUp)
                    .variant(ButtonVariant::Secondary)
                    .on_click(move |_, window, cx| {
                        let handle = handle.clone();
                        let from = handle.offset();
                        let start = Instant::now();
                        window
                            .spawn(cx, async move |cx| {
                                loop {
                                    cx.background_executor().timer(FRAME).await;
                                    let t = (start.elapsed().as_secs_f32()
                                        / duration.as_secs_f32())
                                    .min(1.0);
                                    let left = 1.0 - motion::ease_out_cubic(t);
                                    handle.set_offset(point(from.x, from.y * left));
                                    if cx.update(|window, _| window.refresh()).is_err() || t >= 1.0
                                    {
                                        return;
                                    }
                                }
                            })
                            .detach();
                    }),
            )
            .into_any_element()
    }
}

type Header = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

/// A section whose header pins to the top of `handle`'s viewport.
#[derive(IntoElement)]
pub struct StickyHeader {
    id: ElementId,
    handle: ScrollHandle,
    header: Header,
    body: Vec<AnyElement>,
}

impl StickyHeader {
    pub fn new(
        id: impl Into<ElementId>,
        handle: &ScrollHandle,
        header: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            handle: handle.clone(),
            header: Rc::new(header),
            body: Vec::new(),
        }
    }
}

impl ParentElement for StickyHeader {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

/// Where the section sits in the scrolled content, and its header's height.
#[derive(Clone, Copy, Default, PartialEq)]
struct Placement {
    top: Pixels,
    height: Pixels,
    header: Pixels,
}

/// The header's offset inside its section: 0 at rest, pinned while the section crosses the top.
fn pinned_offset(scrolled: Pixels, at: Placement) -> Pixels {
    let reach = (at.height - at.header).max(Pixels::ZERO);
    (scrolled - at.top).clamp(Pixels::ZERO, reach)
}

impl RenderOnce for StickyHeader {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let seen = window.use_keyed_state(self.id, cx, |_, _| Placement::default());
        let offset = pinned_offset(-self.handle.offset().y, *seen.read(cx));
        let (section_state, header_state, handle) = (seen.clone(), seen, self.handle);
        let bg = cx.theme().colors.bg;
        div()
            .relative()
            .child(
                div()
                    .id("sticky-rest")
                    .invisible()
                    .child((self.header)(window, cx))
                    .child(
                        canvas(
                            move |bounds, window, cx| {
                                if header_state.read(cx).header != bounds.size.height {
                                    header_state.update(cx, |at, cx| {
                                        at.header = bounds.size.height;
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
                    ),
            )
            .children(self.body)
            .child(
                canvas(
                    move |bounds, window, cx| {
                        let content_top = handle.bounds().top() + handle.offset().y;
                        let (top, height) = (bounds.top() - content_top, bounds.size.height);
                        let at = *section_state.read(cx);
                        if at.top != top || at.height != height {
                            section_state.update(cx, |at, cx| {
                                at.top = top;
                                at.height = height;
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
            .child(
                div()
                    .id("sticky-pinned")
                    .absolute()
                    .top(offset)
                    .left_0()
                    .right_0()
                    .bg(bg)
                    .child((self.header)(window, cx)),
            )
    }
}

#[cfg(test)]
mod tests {
    use gpui::px;

    use super::{Placement, pinned_offset};

    #[test]
    fn pins_while_the_section_crosses_the_top_then_yields() {
        let at = Placement {
            top: px(50.0),
            height: px(300.0),
            header: px(30.0),
        };
        assert_eq!(pinned_offset(px(0.0), at), px(0.0));
        assert_eq!(pinned_offset(px(100.0), at), px(50.0));
        assert_eq!(pinned_offset(px(400.0), at), px(270.0));
        let short = Placement {
            height: px(20.0),
            ..at
        };
        assert_eq!(pinned_offset(px(400.0), short), px(0.0));
    }
}
