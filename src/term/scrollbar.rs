//! The terminal's scrollbar: a thumb over the history, in Ely's sizes and colors, that a drag moves through it, whatever the wheel is bound to.

use std::{cell::Cell, rc::Rc};

use alacritty_terminal::{
    grid::{Dimensions, Scroll},
    term::Term,
};
use ely_gpui_component::theme::ActiveTheme;
use gpui::{
    AnyElement, AppContext as _, Context, DragMoveEvent, EmptyView, EntityId, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Pixels, StatefulInteractiveElement, Styled, Window,
    div, relative,
};

use super::view::TerminalView;

/// A drag of the thumb: its view, how far down the history the view stood as it began, and the drag's first point.
pub(super) struct ThumbDrag {
    owner: EntityId,
    from: f32,
    anchor: Rc<Cell<Option<Pixels>>>,
}

/// Where the view stands in the history: the lines above the screen, the screen's lines, and how far up the view is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Span {
    history: usize,
    screen: usize,
    offset: usize,
}

impl Span {
    pub(super) fn of<T>(term: &Term<T>) -> Self {
        Self {
            history: term.grid().history_size(),
            screen: term.screen_lines(),
            offset: term.grid().display_offset(),
        }
    }

    /// The thumb's share of the track.
    fn share(self) -> f32 {
        self.screen as f32 / (self.history + self.screen) as f32
    }

    /// How far down the history the view stands: 0 at its top, 1 at the bottom.
    fn ratio(self) -> f32 {
        1.0 - self.offset as f32 / self.history as f32
    }

    /// How far up the view stands with the thumb `ratio` down the track.
    fn offset_at(self, ratio: f32) -> usize {
        ((1.0 - ratio.clamp(0.0, 1.0)) * self.history as f32).round() as usize
    }
}

/// The bar along the view's right edge, over the padding there; none without history, as on the alternate screen.
pub(super) fn bar(span: Span, window: &Window, cx: &Context<TerminalView>) -> Option<AnyElement> {
    if span.history == 0 {
        return None;
    }
    let theme = cx.theme();
    let rem = window.rem_size();
    let thickness = theme.scrollbar_thickness().to_pixels(rem);
    let min_thumb = theme.scrollbar_min_thumb().to_pixels(rem);
    let (idle, busy) = (theme.colors.fg.opacity(0.22), theme.colors.fg.opacity(0.42));
    let ratio = span.ratio();
    // The room above and below the thumb shares what it leaves as the view stands.
    let (mut above, mut below) = (div(), div());
    above.style().flex_grow = Some(ratio);
    below.style().flex_grow = Some(1.0 - ratio);
    let drag = ThumbDrag {
        owner: cx.entity_id(),
        from: ratio,
        anchor: Rc::default(),
    };
    let thumb = div()
        .id("thumb")
        .w(thickness)
        .h(relative(span.share()))
        .min_h(min_thumb)
        .flex_shrink_0()
        .rounded_full()
        .bg(idle)
        .hover(|style| style.bg(busy))
        .on_drag(drag, |_, _, _, cx| cx.new(|_| EmptyView));
    let bar = div()
        .id("scrollbar")
        .absolute()
        .top_0()
        .bottom_0()
        .right_0()
        .w(thickness * 2.0)
        .pl(thickness * 0.5)
        .flex()
        .flex_col()
        .occlude()
        // It covers the grid, so it gives its terminal the keys.
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|view, _, window, cx| window.focus(&view.focus, cx)),
        )
        .on_drag_move(cx.listener(TerminalView::drag_thumb))
        .child(above)
        .child(thumb)
        .child(below);
    Some(bar.into_any_element())
}

impl TerminalView {
    /// Scrolls the history after the thumb, along its track's bounds.
    fn drag_thumb(
        &mut self,
        event: &DragMoveEvent<ThumbDrag>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let drag = event.drag(cx);
        if drag.owner != cx.entity_id() {
            return;
        }
        let min_thumb = cx
            .theme()
            .scrollbar_min_thumb()
            .to_pixels(window.rem_size());
        let mut term = self.pty.term.lock();
        let span = Span::of(&term);
        let height = event.bounds.size.height;
        let travel = height - (height * span.share()).max(min_thumb).min(height);
        if span.history == 0 || travel <= Pixels::ZERO {
            return;
        }
        let along = event.event.position.y - event.bounds.origin.y;
        let anchor = drag.anchor.get().unwrap_or(along);
        drag.anchor.set(Some(anchor));
        let offset = span.offset_at(drag.from + (along - anchor) / travel);
        let delta = offset as i32 - span.offset as i32;
        if delta != 0 {
            term.scroll_display(Scroll::Delta(delta));
            drop(term);
            log::trace!(
                "terminal {}: the scrollbar shows {offset} lines up",
                self.pty.pid
            );
            cx.notify();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_thumb_stands_where_the_view_stands_in_the_history() {
        let span = |offset| Span {
            history: 300,
            screen: 100,
            offset,
        };
        assert_eq!(span(0).share(), 0.25, "a screen of four");
        assert_eq!(span(0).ratio(), 1.0, "at the bottom");
        assert_eq!(span(300).ratio(), 0.0, "at the top");
        assert_eq!(span(150).ratio(), 0.5);
        assert_eq!(
            span(0).offset_at(0.0),
            300,
            "the thumb at the top shows the oldest lines"
        );
        assert_eq!(span(0).offset_at(1.0), 0);
        assert_eq!(span(0).offset_at(0.5), 150);
        assert_eq!(span(0).offset_at(-0.2), 300, "past the track's ends");
        assert_eq!(span(0).offset_at(1.3), 0);
    }
}
