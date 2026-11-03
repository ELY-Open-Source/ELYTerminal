use gpui::{
    App, ElementId, InteractiveElement, IntoElement, MouseButton, ObjectFit, ParentElement,
    RenderOnce, StatefulInteractiveElement, Styled, Window, div,
};

use crate::{
    layout::AspectRatio,
    overlays::{Lightbox, Slide},
    primitives::{FocusRing, Image, tab_stop},
};

/// The open picture, or the last when it went past the end; with none left the lightbox closes itself.
fn clamp(open: Option<usize>, count: usize) -> Option<usize> {
    open.map(|at| at.min(count.saturating_sub(1)))
}

/// Pictures in square tiles. A press, or Enter on a focused tile, opens the lightbox at that picture.
#[derive(IntoElement)]
pub struct Gallery {
    id: ElementId,
    slides: Vec<Slide>,
    columns: u16,
}

impl Gallery {
    pub fn new(id: impl Into<ElementId>, slides: impl IntoIterator<Item = Slide>) -> Self {
        Self {
            id: id.into(),
            slides: slides.into_iter().collect(),
            columns: 3,
        }
    }

    pub fn columns(mut self, columns: u16) -> Self {
        assert!(columns > 0, "a gallery needs a column");
        self.columns = columns;
        self
    }
}

impl RenderOnce for Gallery {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open = window.use_keyed_state((self.id.clone(), "open"), cx, |_, _| None::<usize>);
        let show = |at: Option<usize>| {
            let (open, id) = (open.clone(), self.id.clone());
            move |cx: &mut App| {
                log::info!(
                    "gallery {id:?}: {}",
                    at.map_or("closed".into(), |at| format!("opened {at}"))
                );
                open.update(cx, |open, cx| {
                    *open = at;
                    cx.notify();
                })
            }
        };
        let mut tiles = Vec::with_capacity(self.slides.len());
        for (ix, slide) in self.slides.iter().enumerate() {
            let focus = tab_stop(
                (self.id.clone(), format!("focus-{ix}")).into(),
                true,
                window,
                cx,
            );
            let (press, keys) = (show(Some(ix)), show(Some(ix)));
            tiles.push(
                div()
                    .id((self.id.clone(), format!("tile-{ix}")))
                    .border_1()
                    .border_color(gpui::transparent_black())
                    .track_focus(&focus)
                    .focus_ring(cx)
                    .cursor_pointer()
                    .hover(|style| style.opacity(0.85))
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .on_click(move |_, _, cx| press(cx))
                    .on_key_down(move |event, _, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            cx.stop_propagation();
                            keys(cx);
                        }
                    })
                    .child(
                        AspectRatio::new(1.0).child(
                            Image::new(
                                (self.id.clone(), format!("thumb-{ix}")),
                                slide.source.clone(),
                            )
                            .fit(ObjectFit::Cover)
                            .size_full(),
                        ),
                    ),
            );
        }
        let at = clamp(*open.read(cx), self.slides.len());
        if at != *open.read(cx) {
            log::info!(
                "gallery {:?}: pictures went away; open is now {at:?}",
                self.id
            );
            open.update(cx, |open, _| *open = at);
        }
        let lightbox = at.map(|at| {
            let (close, step) = (show(None), open.clone());
            Lightbox::new(
                (self.id.clone(), "lightbox"),
                self.slides,
                at,
                move |_, cx| close(cx),
            )
            .on_step(move |to, _, cx| {
                step.update(cx, |open, cx| {
                    *open = Some(to);
                    cx.notify();
                })
            })
        });
        div()
            .grid()
            .grid_cols(self.columns)
            .gap_1()
            .children(tiles)
            .children(lightbox)
    }
}

#[cfg(test)]
mod tests {
    use super::clamp;

    #[test]
    fn an_open_picture_clamps_when_pictures_go_away() {
        assert_eq!(clamp(Some(1), 3), Some(1));
        assert_eq!(clamp(Some(2), 1), Some(0), "past the end opens the last");
        assert_eq!(clamp(Some(2), 0), Some(0), "the lightbox closes itself");
        assert_eq!(clamp(None, 3), None);
    }
}
