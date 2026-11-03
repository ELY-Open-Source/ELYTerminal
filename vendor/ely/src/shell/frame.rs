use gpui::{
    AnyElement, App, Bounds, CursorStyle, Decorations, HitboxBehavior, IntoElement, MouseButton,
    ParentElement, Pixels, Point, RenderOnce, ResizeEdge, Size, Styled, Tiling, Window, canvas,
    div, point, prelude::*,
};
use smallvec::SmallVec;

use crate::theme::{ActiveTheme, Elevation, Radius};

/// The untiled edge or corner `pos` grabs, within `inset`.
fn resize_edge(
    pos: Point<Pixels>,
    inset: Pixels,
    size: Size<Pixels>,
    tiling: Tiling,
) -> Option<ResizeEdge> {
    let (top, bottom) = (
        !tiling.top && pos.y < inset,
        !tiling.bottom && pos.y > size.height - inset,
    );
    let (left, right) = (
        !tiling.left && pos.x < inset,
        !tiling.right && pos.x > size.width - inset,
    );
    Some(match (top, bottom, left, right) {
        (true, _, true, _) => ResizeEdge::TopLeft,
        (true, _, _, true) => ResizeEdge::TopRight,
        (true, ..) => ResizeEdge::Top,
        (_, true, true, _) => ResizeEdge::BottomLeft,
        (_, true, _, true) => ResizeEdge::BottomRight,
        (_, true, ..) => ResizeEdge::Bottom,
        (.., true, _) => ResizeEdge::Left,
        (.., true) => ResizeEdge::Right,
        _ => return None,
    })
}

fn cursor(edge: ResizeEdge) -> CursorStyle {
    match edge {
        ResizeEdge::Top | ResizeEdge::Bottom => CursorStyle::ResizeUpDown,
        ResizeEdge::Left | ResizeEdge::Right => CursorStyle::ResizeLeftRight,
        ResizeEdge::TopLeft | ResizeEdge::BottomRight => CursorStyle::ResizeUpLeftDownRight,
        ResizeEdge::TopRight | ResizeEdge::BottomLeft => CursorStyle::ResizeUpRightDownLeft,
    }
}

/// Resize margin, hairline and shadow for a client-drawn window; else a plain box.
#[derive(IntoElement, Default)]
pub struct ResizeBorder {
    children: SmallVec<[AnyElement; 2]>,
    glass: bool,
}

impl ResizeBorder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Leaves a see-through window's pane to its content: no ground, nor the shadow gpui draws under it.
    pub fn glass(mut self) -> Self {
        self.glass = true;
        self
    }
}

impl ParentElement for ResizeBorder {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ResizeBorder {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Decorations::Client { tiling } = window.window_decorations() else {
            return div().size_full().children(self.children).into_any_element();
        };
        let theme = cx.theme();
        let inset = theme.window_inset().to_pixels(window.rem_size());
        window.set_client_inset(inset);
        let radius = theme.radius(Radius::Lg);
        let floating = !tiling.is_tiled();
        let glass = self.glass;
        div()
            .id("resize-border")
            .size_full()
            .child(
                canvas(
                    |_, window, _| {
                        window.insert_hitbox(
                            Bounds::new(point(Pixels::ZERO, Pixels::ZERO), window.viewport_size()),
                            HitboxBehavior::Normal,
                        )
                    },
                    move |_, hitbox, window, _| {
                        let (at, size) = (window.mouse_position(), window.viewport_size());
                        if let Some(edge) = resize_edge(at, inset, size, tiling) {
                            window.set_cursor_style(cursor(edge), &hitbox);
                        }
                    },
                )
                .absolute()
                .size_full(),
            )
            .when(!tiling.top, |edge| edge.pt(inset))
            .when(!tiling.bottom, |edge| edge.pb(inset))
            .when(!tiling.left, |edge| edge.pl(inset))
            .when(!tiling.right, |edge| edge.pr(inset))
            .on_mouse_move(|_, window, _| window.refresh())
            // Not `window_bounds`: maximized, it holds the restore size.
            .on_mouse_down(MouseButton::Left, move |event, window, _| {
                if let Some(edge) =
                    resize_edge(event.position, inset, window.viewport_size(), tiling)
                {
                    log::info!("window: resize from {edge:?}");
                    window.start_window_resize(edge);
                }
            })
            .child(
                div()
                    .size_full()
                    .overflow_hidden()
                    .cursor(CursorStyle::Arrow)
                    .when(!glass, |pane| pane.bg(theme.colors.bg))
                    .when(floating, |pane| {
                        pane.rounded(radius)
                            .border_1()
                            .border_color(theme.colors.border_strong)
                    })
                    .when(floating && !glass, |pane| {
                        pane.shadow(theme.elevation(Elevation::Modal))
                    })
                    .on_mouse_move(|_, _, cx| cx.stop_propagation())
                    .children(self.children),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use gpui::{ResizeEdge, Tiling, point, px, size};

    use super::resize_edge;

    #[test]
    fn corners_win_over_edges_and_the_middle_is_free() {
        let window = size(px(400.0), px(300.0));
        let at =
            |x: f32, y: f32| resize_edge(point(px(x), px(y)), px(10.0), window, Tiling::default());
        assert_eq!(at(2.0, 2.0), Some(ResizeEdge::TopLeft));
        assert_eq!(at(398.0, 5.0), Some(ResizeEdge::TopRight));
        assert_eq!(at(200.0, 3.0), Some(ResizeEdge::Top));
        assert_eq!(at(1.0, 299.0), Some(ResizeEdge::BottomLeft));
        assert_eq!(at(395.0, 295.0), Some(ResizeEdge::BottomRight));
        assert_eq!(at(200.0, 295.0), Some(ResizeEdge::Bottom));
        assert_eq!(at(4.0, 150.0), Some(ResizeEdge::Left));
        assert_eq!(at(396.0, 150.0), Some(ResizeEdge::Right));
        assert_eq!(at(200.0, 150.0), None);
    }

    #[test]
    fn a_tiled_edge_grabs_nothing() {
        let window = size(px(400.0), px(300.0));
        let at =
            |x: f32, y: f32, tiling| resize_edge(point(px(x), px(y)), px(10.0), window, tiling);
        let left = Tiling {
            top: true,
            left: true,
            bottom: true,
            right: false,
        };
        assert_eq!(at(4.0, 150.0, left), None);
        assert_eq!(at(200.0, 3.0, left), None);
        assert_eq!(at(398.0, 5.0, left), Some(ResizeEdge::Right));
        assert_eq!(at(396.0, 295.0, left), Some(ResizeEdge::Right));
        // Maximized or full screen, every edge is tiled.
        for (x, y) in [(2.0, 2.0), (396.0, 150.0), (200.0, 295.0), (398.0, 298.0)] {
            assert_eq!(at(x, y, Tiling::tiled()), None);
        }
    }
}
