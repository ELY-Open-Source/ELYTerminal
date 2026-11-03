//! What a row of labels needs, and how wide it got.

use gpui::{
    App, Bounds, Entity, Font, FontWeight, Hsla, IntoElement, Pixels, SharedString, Styled,
    TextRun, Window, canvas,
};

use crate::theme::{ActiveTheme, TextSize};

/// `text` set in `size` and `weight`, in pixels.
pub(crate) fn text_width(
    text: &SharedString,
    size: TextSize,
    weight: FontWeight,
    window: &Window,
    cx: &App,
) -> Pixels {
    let font_size = cx.theme().text_size(size).to_pixels(window.rem_size());
    let run = TextRun {
        len: text.len(),
        font: Font {
            weight,
            ..window.text_style().font()
        },
        color: Hsla::default(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    window
        .text_system()
        .shape_line(text.clone(), font_size, &[run], None)
        .width
}

/// Keeps `width` at the parent's width; a change redraws once.
pub(crate) fn measure_width(width: Entity<Pixels>) -> impl IntoElement {
    canvas(
        move |bounds: Bounds<Pixels>, window, cx| {
            if *width.read(cx) != bounds.size.width {
                width.update(cx, |width, cx| {
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
