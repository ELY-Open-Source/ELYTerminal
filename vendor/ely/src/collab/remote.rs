use std::ops::Range;

use gpui::{
    App, Bounds, ContentMask, Entity, FontWeight, IntoElement, Pixels, RenderOnce, SharedString,
    Styled, TextAlign, TextRun, Window, canvas, fill, point, size,
};

use super::peers::Peer;
use crate::{
    forms::TextInput,
    primitives::caret_bar,
    theme::{ActiveTheme, Radius, TextSize},
    typography::LEADING,
};

/// How strongly a peer's color washes their selection.
const WASH: f32 = 0.2;

/// Whether a caret shows inside the field's box.
pub(crate) fn in_view(field: Bounds<Pixels>, caret: Bounds<Pixels>) -> bool {
    caret.bottom() > field.top()
        && caret.top() < field.bottom()
        && caret.left() >= field.left()
        && caret.left() < field.right()
}

/// Someone else's caret in a shared field: a bar in their color, their name on a flag above. Lay it over the field, in a box its size; it reads where `offset` sits as the field paints.
#[derive(IntoElement)]
pub struct RemoteCursor {
    peer: Peer,
    input: Entity<TextInput>,
    offset: usize,
}

impl RemoteCursor {
    pub fn new(peer: &Peer, input: &Entity<TextInput>, offset: usize) -> Self {
        Self {
            peer: peer.clone(),
            input: input.clone(),
            offset,
        }
    }
}

impl RenderOnce for RemoteCursor {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let rem = window.rem_size();
        let (color, ink) = (self.peer.color(cx), theme.colors.on_accent);
        let bar = theme.caret_width().to_pixels(rem);
        let text = theme.text_size(TextSize::Xs).to_pixels(rem);
        let radius = theme.radius(Radius::Sm).to_pixels(rem);
        let name: SharedString = self.peer.name.clone();
        let (input, offset) = (self.input, self.offset);
        canvas(
            |_, _, _| {},
            move |bounds, _, window, cx| {
                let Some(caret) = input.read(cx).bounds_for(offset) else {
                    return;
                };
                if !in_view(bounds, caret) {
                    return;
                }
                window.with_content_mask(Some(ContentMask { bounds }), |window| {
                    let scale = window.scale_factor();
                    let bar = caret_bar(caret.origin, bar, caret.size.height, scale);
                    window.paint_quad(fill(bar, color));
                });
                let font = gpui::Font {
                    weight: FontWeight::MEDIUM,
                    ..window.text_style().font()
                };
                let run = TextRun {
                    len: name.len(),
                    font,
                    color: ink,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                let line = window
                    .text_system()
                    .shape_line(name.clone(), text, &[run], None);
                let (height, pad) = (text * LEADING, text / 2.0);
                let flag = gpui::Bounds::new(
                    point(caret.left(), caret.top() - height),
                    size(line.width + pad * 2.0, height),
                );
                window.paint_quad(fill(flag, color).corner_radii(gpui::Corners {
                    top_left: radius,
                    top_right: radius,
                    bottom_right: radius,
                    bottom_left: gpui::Pixels::ZERO,
                }));
                if let Err(error) = line.paint(
                    point(flag.left() + pad, flag.top()),
                    height,
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                ) {
                    log::error!("remote cursor: name did not paint: {error:#}");
                }
            },
        )
        .absolute()
        .inset_0()
    }
}

/// Someone else's selection in a shared field, washed in their color. Lay it over the field, in a box its size; it reads where `range` sits as the field paints.
#[derive(IntoElement)]
pub struct RemoteSelection {
    peer: Peer,
    input: Entity<TextInput>,
    range: Range<usize>,
}

impl RemoteSelection {
    pub fn new(peer: &Peer, input: &Entity<TextInput>, range: Range<usize>) -> Self {
        Self {
            peer: peer.clone(),
            input: input.clone(),
            range,
        }
    }
}

impl RenderOnce for RemoteSelection {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let wash = self.peer.color(cx).opacity(WASH);
        let (input, range) = (self.input, self.range);
        canvas(
            |_, _, _| {},
            move |bounds, _, window, cx| {
                let lines = input.read(cx).bounds_for_range(range.clone());
                window.with_content_mask(Some(ContentMask { bounds }), |window| {
                    for line in lines {
                        window.paint_quad(fill(line, wash));
                    }
                });
            },
        )
        .absolute()
        .inset_0()
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, point, px, size};

    use super::in_view;

    #[test]
    fn a_caret_outside_the_field_hides() {
        let field = Bounds::new(point(px(100.0), px(100.0)), size(px(200.0), px(60.0)));
        let caret = |x: f32, y: f32| Bounds::new(point(px(x), px(y)), size(px(0.0), px(18.0)));
        assert!(in_view(field, caret(150.0, 110.0)));
        assert!(!in_view(field, caret(150.0, 200.0)), "below");
        assert!(
            !in_view(field, caret(40.0, 110.0)),
            "scrolled off to the left"
        );
        assert!(
            !in_view(field, caret(320.0, 110.0)),
            "scrolled off to the right"
        );
        assert!(in_view(field, caret(100.0, 110.0)), "at the left edge");
    }
}
