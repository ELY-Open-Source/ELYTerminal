use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, Hsla, IntoElement, ParentElement, RenderOnce, Rgba, Styled, Window,
    div,
};

use crate::{
    canvas::color_well,
    data_display::{Badge, Tone},
    theme::{ActiveTheme, Radius, TextSize},
};

/// A color's relative luminance, as WCAG reads it from sRGB.
pub fn luminance(color: Hsla) -> f32 {
    let rgb = Rgba::from(color);
    let linear = |channel: f32| {
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(rgb.r) + 0.7152 * linear(rgb.g) + 0.0722 * linear(rgb.b)
}

/// The contrast between two colors, from 1 for none to 21 for black on white.
pub fn contrast(a: Hsla, b: Hsla) -> f32 {
    let (light, dark) = {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y), x.min(y))
    };
    (light + 0.05) / (dark + 0.05)
}

/// `text` laid over `background` as it shows: its alpha blends it in, in sRGB; the background counts as opaque.
pub fn over(text: Hsla, background: Hsla) -> Hsla {
    let (fg, bg) = (Rgba::from(text), Rgba::from(background));
    let mix = |front: f32, back: f32| front * fg.a + back * (1.0 - fg.a);
    Hsla::from(Rgba {
        r: mix(fg.r, bg.r),
        g: mix(fg.g, bg.g),
        b: mix(fg.b, bg.b),
        a: 1.0,
    })
}

/// The WCAG levels a contrast meets: AA and AAA for body text, and for large text.
pub fn grades(ratio: f32) -> [(&'static str, bool); 4] {
    [
        ("AA", ratio >= 4.5),
        ("AAA", ratio >= 7.0),
        ("AA large", ratio >= 3.0),
        ("AAA large", ratio >= 4.5),
    ]
}

type OnColors = Rc<dyn Fn(Hsla, Hsla, &mut Window, &mut App)>;
type OnColor = Rc<dyn Fn(Hsla, &mut Window, &mut App)>;

/// Text on a background, each color opening a picker, with the contrast between them and the WCAG levels it meets, over a sample of body and large text. Translucent text counts as it blends in; the background counts as opaque.
#[derive(IntoElement)]
pub struct ColorContrastChecker {
    id: ElementId,
    text: Hsla,
    background: Hsla,
    on_change: OnColors,
}

impl ColorContrastChecker {
    pub fn new(
        id: impl Into<ElementId>,
        text: impl Into<Hsla>,
        background: impl Into<Hsla>,
        on_change: impl Fn(Hsla, Hsla, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            background: background.into(),
            on_change: Rc::new(on_change),
        }
    }
}

impl RenderOnce for ColorContrastChecker {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (text, background) = (self.text, self.background);
        let solid = Hsla {
            a: 1.0,
            ..background
        };
        let ratio = contrast(over(text, solid), solid);
        let on_change = self.on_change;
        let theme = cx.theme();
        let well = |name: &'static str, color: Hsla, set: OnColor| {
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(color_well(
                    (self.id.clone(), name).into(),
                    color,
                    move |color, window, cx| set(color, window, cx),
                ))
                .child(div().text_size(theme.text_size(TextSize::Sm)).child(name))
        };
        let (fore, back) = (on_change.clone(), on_change);
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_4()
                    .child(well(
                        "Text",
                        text,
                        Rc::new(move |color, window, cx| {
                            log::info!("color contrast checker: text");
                            fore(color, background, window, cx);
                        }),
                    ))
                    .child(well(
                        "Background",
                        background,
                        Rc::new(move |color, window, cx| {
                            log::info!("color contrast checker: background");
                            back(text, color, window, cx);
                        }),
                    )),
            )
            .child(
                div()
                    .p_4()
                    .rounded(theme.radius(Radius::Md))
                    .border_1()
                    .border_color(theme.colors.border)
                    .bg(background)
                    .text_color(text)
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Base))
                            .child("Body text reads at this size."),
                    )
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Xl))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Large text"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Lg))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!("{ratio:.2} : 1")),
                    )
                    .children(grades(ratio).into_iter().map(|(level, met)| {
                        Badge::new(level).tone(if met { Tone::Success } else { Tone::Danger })
                    })),
            )
    }
}

#[cfg(test)]
mod tests {
    use gpui::{hsla, rgb};

    use super::{contrast, grades, over};

    #[test]
    fn black_on_white_is_twenty_one_and_a_color_on_itself_is_one() {
        let (black, white) = (hsla(0.0, 0.0, 0.0, 1.0), hsla(0.0, 0.0, 1.0, 1.0));
        assert!((contrast(black, white) - 21.0).abs() < 0.01);
        assert!((contrast(white, white) - 1.0).abs() < 0.001);
        let grey: gpui::Hsla = rgb(0x767676).into();
        let ratio = contrast(grey, white);
        assert!(
            (ratio - 4.54).abs() < 0.02,
            "the lightest grey that passes AA on white: {ratio}"
        );
        assert_eq!(grades(ratio).map(|(_, met)| met), [true, false, true, true]);
    }

    #[test]
    fn translucent_text_counts_as_it_blends_in() {
        let white = gpui::hsla(0.0, 0.0, 1.0, 1.0);
        let half = gpui::hsla(0.0, 0.0, 0.0, 0.5);
        let blended = over(half, white);
        assert!(
            (blended.l - 0.5).abs() < 0.01,
            "half black on white is mid gray: {blended:?}"
        );
        assert!(contrast(blended, white) < contrast(gpui::hsla(0.0, 0.0, 0.0, 1.0), white));
    }

    #[test]
    fn each_level_starts_at_its_ratio() {
        let met = |ratio: f32| grades(ratio).map(|(_, met)| met);
        assert_eq!(met(2.9), [false, false, false, false]);
        assert_eq!(met(3.1), [false, false, true, false]);
        assert_eq!(met(4.4), [false, false, true, false]);
        assert_eq!(met(4.6), [true, false, true, true]);
        assert_eq!(met(6.9), [true, false, true, true]);
        assert_eq!(met(7.1), [true, true, true, true]);
    }
}
