use std::rc::Rc;

use gpui::{App, Div, ElementId, Entity, ParentElement, SharedString, Styled, Window, div};

use super::{
    super::{Choice, Input, ScrubInput, Select, TextInput},
    Hsva,
};
use crate::theme::{ActiveTheme, ControlSize};

/// How a color picker's typed fields spell the color.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Format {
    #[default]
    Hex,
    Rgb,
    Hsv,
}

impl Format {
    const ALL: [Self; 3] = [Self::Hex, Self::Rgb, Self::Hsv];

    fn label(self) -> &'static str {
        match self {
            Self::Hex => "HEX",
            Self::Rgb => "RGB",
            Self::Hsv => "HSV",
        }
    }
}

pub(crate) type Commit = Rc<dyn Fn(Hsva, &mut Window, &mut App)>;
pub(crate) type SetFormat = Rc<dyn Fn(Format, &mut Window, &mut App)>;

/// A select of the three formats.
pub(crate) fn format_select(id: &ElementId, format: Format, set: SetFormat) -> Select {
    Select::new(
        (id.clone(), "format"),
        Format::ALL.map(|format| Choice::new(format.label(), format.label())),
    )
    .selected(format.label())
    .size(ControlSize::Md)
    .on_change(move |label: &SharedString, window, cx| {
        let format = Format::ALL
            .into_iter()
            .find(|format| format.label() == label.as_ref())
            .expect("a label from the list");
        log::info!("color picker: typed as {}", format.label());
        set(format, window, cx);
    })
}

/// One labeled number of the RGB or HSV stack.
fn part(
    id: ElementId,
    label: &'static str,
    value: f32,
    max: f32,
    set: impl Fn(f32) -> Hsva + 'static,
    commit: Commit,
) -> ScrubInput {
    ScrubInput::new(id, label, f64::from(value.round()))
        .range(0.0, f64::from(max))
        .on_change(move |value, window, cx| commit(set(value as f32), window, cx))
}

/// The fields for `format`: one hex field, or three numbers.
pub(crate) fn typed_fields(
    id: &ElementId,
    format: Format,
    color: Hsva,
    hex_field: &Entity<TextInput>,
    commit: Commit,
    cx: &App,
) -> Div {
    let stack = div().flex().flex_col().gap_1();
    match format {
        Format::Hex => stack.child(
            div()
                .font_family(cx.theme().mono_family.clone())
                .child(Input::new(hex_field)),
        ),
        Format::Rgb => {
            let rgba = color.to_rgba();
            let channel = move |pick: fn(&mut gpui::Rgba) -> &mut f32| {
                move |value: f32| {
                    let mut next = rgba;
                    *pick(&mut next) = value / 255.0;
                    Hsva::from_rgba(next, color.h)
                }
            };
            stack
                .child(part(
                    (id.clone(), "red").into(),
                    "R",
                    rgba.r * 255.0,
                    255.0,
                    channel(|c| &mut c.r),
                    commit.clone(),
                ))
                .child(part(
                    (id.clone(), "green").into(),
                    "G",
                    rgba.g * 255.0,
                    255.0,
                    channel(|c| &mut c.g),
                    commit.clone(),
                ))
                .child(part(
                    (id.clone(), "blue").into(),
                    "B",
                    rgba.b * 255.0,
                    255.0,
                    channel(|c| &mut c.b),
                    commit,
                ))
        }
        Format::Hsv => stack
            .child(part(
                (id.clone(), "hue").into(),
                "H",
                color.h,
                360.0,
                move |h| Hsva { h, ..color },
                commit.clone(),
            ))
            .child(part(
                (id.clone(), "saturation").into(),
                "S",
                color.s * 100.0,
                100.0,
                move |s| Hsva {
                    s: s / 100.0,
                    ..color
                },
                commit.clone(),
            ))
            .child(part(
                (id.clone(), "value").into(),
                "V",
                color.v * 100.0,
                100.0,
                move |v| Hsva {
                    v: v / 100.0,
                    ..color
                },
                commit,
            )),
    }
}
