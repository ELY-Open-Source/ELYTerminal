use std::sync::Arc;

use gpui::{
    App, Div, FontFeatures, FontWeight, Hsla, IntoElement, ParentElement, Refineable, RenderOnce,
    SharedString, StyleRefinement, Styled, Window, div, relative,
};

use crate::theme::{ActiveTheme, Palette, TextSize};

/// Tabular numerals, so digits line up in columns.
pub fn tabular<E: Styled>(mut element: E) -> E {
    let features = FontFeatures(Arc::new(vec![("tnum".into(), 1)]));
    element.text_style().font_features = Some(features);
    element
}

/// Each character drawn as itself, without ligatures: `==` stays two signs in encoded text and keys.
pub fn literal<E: Styled>(mut element: E) -> E {
    let features = FontFeatures(Arc::new(vec![("calt".into(), 0), ("liga".into(), 0)]));
    element.text_style().font_features = Some(features);
    element
}

struct Spec {
    size: TextSize,
    weight: FontWeight,
    color: fn(&Palette) -> Hsla,
    leading: f32,
}

fn styled(text: SharedString, spec: Spec, style: &StyleRefinement, cx: &App) -> Div {
    let theme = cx.theme();
    let mut base = div()
        .text_size(theme.text_size(spec.size))
        .font_weight(spec.weight)
        .text_color((spec.color)(&theme.colors))
        .line_height(relative(spec.leading));
    base.style().refine(style);
    base.child(text)
}

macro_rules! text_kind {
    ($(#[$doc:meta])* $name:ident, $size:ident, $weight:ident, $color:ident, $leading:expr) => {
        $(#[$doc])*
        #[derive(IntoElement)]
        pub struct $name {
            text: SharedString,
            style: StyleRefinement,
        }

        impl $name {
            pub fn new(text: impl Into<SharedString>) -> Self {
                Self { text: text.into(), style: StyleRefinement::default() }
            }
        }

        impl Styled for $name {
            fn style(&mut self) -> &mut StyleRefinement {
                &mut self.style
            }
        }

        impl RenderOnce for $name {
            fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
                let spec = Spec {
                    size: TextSize::$size,
                    weight: FontWeight::$weight,
                    color: |colors| colors.$color,
                    leading: $leading,
                };
                styled(self.text, spec, &self.style, cx)
            }
        }
    };
}

text_kind!(Title, Xl, SEMIBOLD, fg, 1.3);
text_kind!(Subtitle, Lg, NORMAL, fg_muted, 1.45);
text_kind!(
    /// Reading text. Loose leading.
    Paragraph, Md, NORMAL, fg, 1.6
);
text_kind!(Label, Base, MEDIUM, fg, 1.4);
text_kind!(Caption, Sm, NORMAL, fg_muted, 1.45);

/// Small upper-case eyebrow above a heading.
#[derive(IntoElement)]
pub struct Overline {
    text: SharedString,
    style: StyleRefinement,
}

impl Overline {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            style: StyleRefinement::default(),
        }
    }
}

impl Styled for Overline {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Overline {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let spec = Spec {
            size: TextSize::Xs,
            weight: FontWeight::SEMIBOLD,
            color: |colors| colors.fg_subtle,
            leading: 1.4,
        };
        styled(self.text.to_uppercase().into(), spec, &self.style, cx)
    }
}

/// H1 to H6.
#[derive(IntoElement)]
pub struct Heading {
    level: u8,
    text: SharedString,
    style: StyleRefinement,
}

impl Heading {
    /// Levels run 1 to 6; others panic.
    pub fn new(level: u8, text: impl Into<SharedString>) -> Self {
        assert!(
            (1..=6).contains(&level),
            "heading level {level} is not 1 to 6"
        );
        Self {
            level,
            text: text.into(),
            style: StyleRefinement::default(),
        }
    }

    pub fn h1(text: impl Into<SharedString>) -> Self {
        Self::new(1, text)
    }

    pub fn h2(text: impl Into<SharedString>) -> Self {
        Self::new(2, text)
    }

    pub fn h3(text: impl Into<SharedString>) -> Self {
        Self::new(3, text)
    }

    pub fn h4(text: impl Into<SharedString>) -> Self {
        Self::new(4, text)
    }

    pub fn h5(text: impl Into<SharedString>) -> Self {
        Self::new(5, text)
    }

    pub fn h6(text: impl Into<SharedString>) -> Self {
        Self::new(6, text)
    }
}

impl Styled for Heading {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Heading {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let size = match self.level {
            1 => TextSize::Display,
            2 => TextSize::Xxl,
            3 => TextSize::Xl,
            4 => TextSize::Lg,
            5 => TextSize::Md,
            _ => TextSize::Base,
        };
        let spec = Spec {
            size,
            weight: FontWeight::SEMIBOLD,
            color: |colors| colors.fg,
            leading: 1.25,
        };
        styled(self.text, spec, &self.style, cx)
    }
}
