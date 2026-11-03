use gpui::{
    App, Div, ElementId, InteractiveElement, IntoElement, ParentElement, Pixels, RenderOnce,
    ShapedLine, SharedString, StatefulInteractiveElement, Styled, TextAlign, TextRun, Window,
    canvas, div, point, prelude::*, relative, transparent_black,
};

use crate::{
    primitives::Tooltip,
    theme::{ActiveTheme, TextSize},
};

pub(crate) const LEADING: f32 = 1.4;

/// Keeps `kept` chars: half from the front, half from the back.
fn elide(chars: &[char], kept: usize) -> String {
    let head = kept.div_ceil(2);
    let tail = kept / 2;
    let mut out: String = chars[..head].iter().collect();
    out.push('…');
    out.extend(&chars[chars.len() - tail..]);
    out
}

/// How wide `text` sets at `size` in the text style around it.
pub(crate) fn text_width(text: &str, size: Pixels, window: &Window) -> Pixels {
    let style = window.text_style();
    let run = TextRun {
        len: text.len(),
        font: style.font(),
        color: style.color,
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    window
        .text_system()
        .shape_line(SharedString::from(text.to_string()), size, &[run], None)
        .width
}

fn shape(text: &str, window: &mut Window) -> ShapedLine {
    let style = window.text_style();
    let run = style.to_run(text.len());
    let size = style.font_size.to_pixels(window.rem_size());
    window
        .text_system()
        .shape_line(SharedString::from(text.to_string()), size, &[run], None)
}

fn width_of(text: &str, window: &mut Window) -> Pixels {
    shape(text, window).width
}

/// Longest middle cut that fits `width`.
fn fit_middle(text: &str, width: Pixels, window: &mut Window) -> String {
    if width_of(text, window) <= width {
        return text.to_string();
    }
    if width_of("…", window) > width {
        return String::new();
    }
    let chars: Vec<char> = text.chars().collect();
    let (mut low, mut high) = (0, chars.len());
    while low < high {
        let mid = (low + high).div_ceil(2);
        if width_of(&elide(&chars, mid), window) <= width {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    elide(&chars, low)
}

/// Longest front that fits `width` with an ellipsis after it.
fn fit_end(text: &str, width: Pixels, window: &mut Window) -> String {
    if width_of(text, window) <= width {
        return text.to_string();
    }
    if width_of("…", window) > width {
        return String::new();
    }
    let chars: Vec<char> = text.chars().collect();
    let cut = |kept: usize| {
        let front: String = chars[..kept].iter().collect();
        let mut out: String = front
            .trim_end_matches(|ch: char| ch.is_whitespace() || matches!(ch, '.' | ',' | ';' | ':'))
            .into();
        out.push('…');
        out
    };
    let (mut low, mut high) = (0, chars.len());
    while low < high {
        let mid = (low + high).div_ceil(2);
        if width_of(&cut(mid), window) <= width {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    cut(low)
}

/// Where a line starts in a box with `free` width to spare.
fn offset(align: TextAlign, free: Pixels) -> Pixels {
    let free = free.max(Pixels::ZERO);
    match align {
        TextAlign::Left => Pixels::ZERO,
        TextAlign::Center => free / 2.,
        TextAlign::Right => free,
    }
}

/// One line, cut by `fit` to its box and aligned.
fn cut_line(text: SharedString, fit: fn(&str, Pixels, &mut Window) -> String) -> Div {
    let line = text.clone();
    div()
        .relative()
        .min_w_0()
        .overflow_hidden()
        .whitespace_nowrap()
        .child(div().text_color(transparent_black()).child(text))
        .child(
            canvas(
                move |bounds, window, _| shape(&fit(&line, bounds.size.width, window), window),
                |bounds, shaped, window, cx| {
                    let style = window.text_style();
                    let line = style.line_height_in_pixels(window.rem_size());
                    let x = offset(style.text_align, bounds.size.width - shaped.width);
                    let painted = shaped.paint(
                        bounds.origin + point(x, Pixels::ZERO),
                        line,
                        TextAlign::Left,
                        None,
                        window,
                        cx,
                    );
                    if let Err(error) = painted {
                        log::error!("ellipsis: a cut line failed to paint: {error:#}");
                    }
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
}

/// One line in the text style around it, cut at the end with an ellipsis when its box is too narrow.
#[derive(IntoElement)]
pub struct Ellipsis {
    text: SharedString,
}

impl Ellipsis {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self { text: text.into() }
    }
}

impl RenderOnce for Ellipsis {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        cut_line(self.text, fit_end)
    }
}

/// One line in the text style around it, cut in the middle when its box is too narrow, so both ends stay: paths, file names, hashes.
#[derive(IntoElement)]
pub struct MiddleEllipsis {
    text: SharedString,
}

impl MiddleEllipsis {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self { text: text.into() }
    }
}

impl RenderOnce for MiddleEllipsis {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        cut_line(self.text, fit_middle)
    }
}

/// Truncates at the end; the full text shows on hover only when cut.
#[derive(IntoElement)]
pub struct EllipsisTooltip {
    id: ElementId,
    text: SharedString,
}

impl EllipsisTooltip {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
        }
    }
}

impl RenderOnce for EllipsisTooltip {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let cut = window.use_keyed_state(self.id.clone(), cx, |_, _| false);
        let is_cut = *cut.read(cx);
        let theme = cx.theme();
        let measured = self.text.clone();
        div()
            .id(self.id)
            .relative()
            .truncate()
            .text_size(theme.text_size(TextSize::Base))
            .line_height(relative(LEADING))
            .text_color(theme.colors.fg)
            .child(self.text.clone())
            .child(
                canvas(
                    move |bounds, window, cx| {
                        let natural = width_of(&measured, window);
                        let overflow = natural > bounds.size.width;
                        if *cut.read(cx) != overflow {
                            log::debug!(
                                "ellipsis tooltip: {natural:?} in {:?}, cut {overflow}",
                                bounds.size.width
                            );
                            cut.update(cx, |cut, cx| {
                                *cut = overflow;
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
            .when(is_cut, |el| el.tooltip(Tooltip::text(self.text)))
    }
}

#[cfg(test)]
mod tests {
    use gpui::{TextAlign, px};

    use super::{elide, offset};

    #[test]
    fn a_line_starts_by_its_alignment() {
        assert_eq!(offset(TextAlign::Left, px(40.)), px(0.));
        assert_eq!(offset(TextAlign::Center, px(40.)), px(20.));
        assert_eq!(offset(TextAlign::Right, px(40.)), px(40.));
        assert_eq!(
            offset(TextAlign::Right, px(-8.)),
            px(0.),
            "a cut line fills its box"
        );
    }

    #[test]
    fn elide_keeps_both_ends() {
        let chars: Vec<char> = "abcdefghij".chars().collect();
        assert_eq!(elide(&chars, 4), "ab…ij");
        assert_eq!(elide(&chars, 5), "abc…ij");
        assert_eq!(elide(&chars, 0), "…");
        let wide: Vec<char> = "路径/很长/文件.rs".chars().collect();
        assert_eq!(elide(&wide, 4), "路径…rs");
    }
}

#[cfg(all(test, feature = "test-support"))]
mod fits {
    use gpui::{Pixels, TestAppContext};

    use super::{fit_end, fit_middle, width_of};

    #[gpui::test]
    fn a_middle_cut_keeps_the_front_and_the_extension(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        cx.update(|window, _| {
            let text = "A very long file name that keeps going.pdf";
            let room = width_of("A very long…going.pdf", window);
            let cut = fit_middle(text, room, window);
            assert!(cut.starts_with("A very") && cut.ends_with(".pdf"), "{cut}");
            assert!(width_of(&cut, window) <= room && cut.contains('…'), "{cut}");
        });
    }

    #[gpui::test]
    fn an_ellipsis_keeps_the_longest_front_that_fits(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        cx.update(|window, _| {
            let text = "Good interfaces. Quiet ones";
            let whole = width_of(text, window);
            assert_eq!(
                fit_end(text, whole, window),
                text,
                "a line that fits stays whole"
            );
            let room = width_of("Good interfaces.…", window);
            assert_eq!(
                fit_end(text, room, window),
                "Good interfaces…",
                "no stop or space before the ellipsis"
            );
            assert_eq!(
                fit_end(text, Pixels::ZERO, window),
                "",
                "no room shows nothing"
            );
        });
    }
}
