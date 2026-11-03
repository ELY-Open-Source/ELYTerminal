use std::rc::Rc;

use gpui::{
    App, ElementId, Entity, FontWeight, IntoElement, ParentElement, Pixels, RenderOnce,
    SharedString, Styled, TextAlign, Window, div, prelude::*, relative,
};

use super::panel::{OnEdit, editing, pair};
use crate::{
    buttons::SegmentedControl,
    forms::{Choice, FontPicker, ScrubInput, Select, TextInput},
    primitives::IconName,
    theme::{ActiveTheme, Radius},
};

/// How text is set: its family and weight, its size in pixels, its line height as a share of the size, its tracking in pixels, and its alignment.
#[derive(Clone, Debug, PartialEq)]
pub struct TypeStyle {
    pub family: SharedString,
    pub weight: FontWeight,
    pub size: f32,
    pub line_height: f32,
    pub tracking: f32,
    pub align: TextAlign,
}

const WEIGHTS: [(f32, &str); 9] = [
    (100.0, "Thin"),
    (200.0, "Extra light"),
    (300.0, "Light"),
    (400.0, "Regular"),
    (500.0, "Medium"),
    (600.0, "Semibold"),
    (700.0, "Bold"),
    (800.0, "Extra bold"),
    (900.0, "Black"),
];

const ALIGNS: [(TextAlign, &str, IconName); 3] = [
    (TextAlign::Left, "Left", IconName::AlignLeft),
    (TextAlign::Center, "Center", IconName::AlignCenter),
    (TextAlign::Right, "Right", IconName::AlignRight),
];

/// A text style's family, weight, size, line height, tracking and alignment, over a line set in it. The line shows all but the tracking, which gpui does not set.
#[derive(IntoElement)]
pub struct TypographyPanel {
    id: ElementId,
    style: TypeStyle,
    search: Entity<TextInput>,
    on_change: OnEdit<TypeStyle>,
}

impl TypographyPanel {
    /// `search` is the family field's text, which the owner keeps.
    pub fn new(
        id: impl Into<ElementId>,
        style: TypeStyle,
        search: &Entity<TextInput>,
        on_change: impl Fn(TypeStyle, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            style,
            search: search.clone(),
            on_change: Rc::new(on_change),
        }
    }
}

impl RenderOnce for TypographyPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_change = self.on_change;
        let (id, style, on_change) = (self.id, self.style, &on_change);
        let theme = cx.theme();
        let family = editing(
            "typography panel",
            &style,
            on_change,
            |style, family: SharedString| style.family = family,
        );
        let weight = editing(
            "typography panel",
            &style,
            on_change,
            |style, weight: f32| style.weight = FontWeight(weight),
        );
        let align = editing(
            "typography panel",
            &style,
            on_change,
            |style, align: TextAlign| style.align = align,
        );
        let at =
            |edit: fn(&mut TypeStyle, f64)| editing("typography panel", &style, on_change, edit);
        let scrub = |name: &'static str, value: f32| {
            ScrubInput::new((id.clone(), name), name, value as f64)
        };
        let named = WEIGHTS
            .iter()
            .find(|(weight, _)| *weight == style.weight.0)
            .unwrap_or_else(|| panic!("typography panel: no weight {}", style.weight.0));
        let (_, word, _) = ALIGNS
            .iter()
            .find(|(align, _, _)| *align == style.align)
            .expect("every alignment is offered");
        let sample = div()
            .p_3()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(theme.colors.border)
            .overflow_hidden()
            .font_family(style.family.clone())
            .font_weight(style.weight)
            .text_size(Pixels::from(style.size))
            .line_height(relative(style.line_height))
            .text_color(theme.colors.fg)
            .map(|line| match style.align {
                TextAlign::Left => line.text_left(),
                TextAlign::Center => line.text_center(),
                TextAlign::Right => line.text_right(),
            })
            .child("The quick brown fox");
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                FontPicker::new((id.clone(), "family"), &self.search)
                    .selected(style.family.clone())
                    .on_change(move |chosen, window, cx| family(chosen.clone(), window, cx)),
            )
            .child(pair(
                Select::new(
                    (id.clone(), "weight"),
                    WEIGHTS
                        .iter()
                        .map(|(weight, name)| Choice::new(weight.to_string(), *name)),
                )
                .selected(named.0.to_string())
                .on_change(move |value, window, cx| {
                    weight(
                        value.parse().expect("a weight the select offers"),
                        window,
                        cx,
                    )
                }),
                scrub("Size", style.size)
                    .range(1.0, 999.0)
                    .precision(0)
                    .on_change(at(|s, v| s.size = v as f32)),
            ))
            .child(
                scrub("Line height", style.line_height)
                    .range(0.5, 4.0)
                    .step(0.05)
                    .precision(2)
                    .on_change(at(|s, v| s.line_height = v as f32)),
            )
            .child(
                scrub("Tracking", style.tracking)
                    .range(-20.0, 100.0)
                    .step(0.1)
                    .precision(1)
                    .on_change(at(|s, v| s.tracking = v as f32)),
            )
            .child(
                ALIGNS
                    .iter()
                    .fold(
                        SegmentedControl::new((id, "align"), *word),
                        |control, (_, word, icon)| control.segment(*word, *word, Some(*icon)),
                    )
                    .on_change(move |word, window, cx| {
                        let (chosen, _, _) = ALIGNS
                            .iter()
                            .find(|(_, name, _)| *name == word.as_ref())
                            .expect("an alignment the control offers");
                        align(*chosen, window, cx)
                    }),
            )
            .child(sample)
    }
}
