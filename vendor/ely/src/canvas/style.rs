use std::rc::Rc;

use gpui::{
    App, ElementId, Hsla, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window,
    div, prelude::*,
};

use super::panel::{OnEdit, caption, color_well, editing, heading, pair};
use crate::{
    buttons::{IconButton, SegmentedControl, ToggleButton, ToggleItem},
    forms::{ColorPalette, ColorPicker, ScrubInput},
    primitives::IconName,
    theme::{ActiveTheme, Elevation},
    typography::Ellipsis,
};

/// The most a shadow or a border measures, in pixels.
const FAR: f64 = 1_000.0;

/// A color to edit in full, over the document's colors to pick from.
#[derive(IntoElement)]
pub struct ColorPanel {
    id: ElementId,
    color: Hsla,
    swatches: Vec<(SharedString, Hsla)>,
    on_change: OnEdit<Hsla>,
}

impl ColorPanel {
    pub fn new(
        id: impl Into<ElementId>,
        color: impl Into<Hsla>,
        on_change: impl Fn(Hsla, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            color: color.into(),
            swatches: Vec::new(),
            on_change: Rc::new(on_change),
        }
    }

    /// The document's colors, by name.
    pub fn swatches(
        mut self,
        swatches: impl IntoIterator<Item = (impl Into<SharedString>, impl Into<Hsla>)>,
    ) -> Self {
        self.swatches = swatches
            .into_iter()
            .map(|(name, color)| (name.into(), color.into()))
            .collect();
        self
    }
}

impl RenderOnce for ColorPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_change = self.on_change;
        let (picked, chosen) = (on_change.clone(), on_change);
        let report = |on_change: OnEdit<Hsla>| {
            move |color: Hsla, window: &mut Window, cx: &mut App| {
                log::info!("color panel: {color:?}");
                on_change(color, window, cx);
            }
        };
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                ColorPicker::new((self.id.clone(), "picker"), self.color).on_change(report(picked)),
            )
            .when(!self.swatches.is_empty(), |panel| {
                panel.child(caption("Document colors", cx)).child(
                    ColorPalette::new((self.id, "swatches"), self.swatches)
                        .selected(self.color)
                        .on_change(report(chosen)),
                )
            })
    }
}

/// A drop shadow: its offset, blur and spread in pixels, and its color.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub x: f32,
    pub y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: Hsla,
}

/// Shadows in a list. Each sets its color, offset, blur and spread, and minus drops it; plus adds the theme's floating shadow.
#[derive(IntoElement)]
pub struct ShadowEditor {
    id: ElementId,
    shadows: Vec<Shadow>,
    on_change: OnEdit<Vec<Shadow>>,
}

impl ShadowEditor {
    pub fn new(
        id: impl Into<ElementId>,
        shadows: impl IntoIterator<Item = Shadow>,
        on_change: impl Fn(Vec<Shadow>, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            shadows: shadows.into_iter().collect(),
            on_change: Rc::new(on_change),
        }
    }
}

impl RenderOnce for ShadowEditor {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_change = self.on_change;
        let lift = cx
            .theme()
            .elevation(Elevation::Floating)
            .last()
            .map(|shadow| Shadow {
                x: f32::from(shadow.offset.x),
                y: f32::from(shadow.offset.y),
                blur: f32::from(shadow.blur_radius),
                spread: f32::from(shadow.spread_radius),
                color: shadow.color,
            })
            .expect("a floating elevation casts a shadow");
        let (id, shadows, on_change) = (self.id, self.shadows, &on_change);
        let add = editing("shadow editor", &shadows, on_change, move |all, _: ()| {
            all.push(lift)
        });
        let rows = shadows.iter().enumerate().map(|(ix, shadow)| {
            let at = |edit: fn(&mut Shadow, f64)| {
                editing(
                    "shadow editor",
                    &shadows,
                    on_change,
                    move |all: &mut Vec<Shadow>, value| edit(&mut all[ix], value),
                )
            };
            let scrub = |name: &'static str, value: f32, least: f64| {
                ScrubInput::new((id.clone(), format!("{name}-{ix}")), name, value as f64)
                    .range(least, FAR)
                    .precision(0)
            };
            let tint = editing(
                "shadow editor",
                &shadows,
                on_change,
                move |all, color: Hsla| all[ix].color = color,
            );
            let remove = editing("shadow editor", &shadows, on_change, move |all, _: ()| {
                all.remove(ix);
            });
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(color_well(
                            (id.clone(), format!("color-{ix}")).into(),
                            shadow.color,
                            tint,
                        ))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(Ellipsis::new(format!("Shadow {}", ix + 1))),
                        )
                        .child(
                            IconButton::new((id.clone(), format!("remove-{ix}")), IconName::Minus)
                                .tooltip("Remove the shadow")
                                .on_click(move |_, window, cx| remove((), window, cx)),
                        ),
                )
                .child(pair(
                    scrub("X", shadow.x, -FAR).on_change(at(|s, v| s.x = v as f32)),
                    scrub("Y", shadow.y, -FAR).on_change(at(|s, v| s.y = v as f32)),
                ))
                .child(pair(
                    scrub("Blur", shadow.blur, 0.0).on_change(at(|s, v| s.blur = v as f32)),
                    scrub("Spread", shadow.spread, -FAR).on_change(at(|s, v| s.spread = v as f32)),
                ))
        });
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(heading(
                "Shadows",
                IconButton::new((id.clone(), "add"), IconName::Plus)
                    .tooltip("Add a shadow")
                    .on_click(move |_, window, cx| add((), window, cx)),
                cx,
            ))
            .children(rows)
    }
}

/// How a border's line is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dash {
    Solid,
    Dashed,
    Dotted,
}

impl Dash {
    pub const ALL: [Dash; 3] = [Dash::Solid, Dash::Dashed, Dash::Dotted];

    pub fn word(self) -> &'static str {
        match self {
            Dash::Solid => "Solid",
            Dash::Dashed => "Dashed",
            Dash::Dotted => "Dotted",
        }
    }
}

/// A border: its width in pixels, its line, its color, and each corner's radius from the top left around.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Border {
    pub width: f32,
    pub dash: Dash,
    pub color: Hsla,
    pub radii: [f32; 4],
}

/// A border's color, width and line, and its corners: one radius for all, or each its own once split. Uneven corners always show split; joining them gives all the first one's radius.
#[derive(IntoElement)]
pub struct BorderEditor {
    id: ElementId,
    border: Border,
    on_change: OnEdit<Border>,
}

impl BorderEditor {
    pub fn new(
        id: impl Into<ElementId>,
        border: Border,
        on_change: impl Fn(Border, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            border,
            on_change: Rc::new(on_change),
        }
    }
}

const CORNERS: [&str; 4] = ["TL", "TR", "BR", "BL"];

impl RenderOnce for BorderEditor {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_change = self.on_change;
        let (id, border, on_change) = (self.id, self.border, &on_change);
        let parted = window.use_keyed_state((id.clone(), "split"), cx, |_, _| false);
        let uneven = border.radii.iter().any(|radius| *radius != border.radii[0]);
        let split = *parted.read(cx) || uneven;
        let at = |edit: fn(&mut Border, f64)| editing("border editor", &border, on_change, edit);
        let scrub = |name: &'static str, value: f32| {
            ScrubInput::new((id.clone(), name), name, value as f64)
                .range(0.0, FAR)
                .precision(0)
        };
        let dash = {
            let set = editing("border editor", &border, on_change, |border, dash: Dash| {
                border.dash = dash
            });
            Dash::ALL
                .iter()
                .fold(
                    SegmentedControl::new((id.clone(), "dash"), border.dash.word()),
                    |control, dash| control.segment(dash.word(), dash.word(), None),
                )
                .on_change(move |word, window, cx| {
                    let dash = *Dash::ALL
                        .iter()
                        .find(|dash| dash.word() == word.as_ref())
                        .expect("a dash the control offers");
                    set(dash, window, cx)
                })
        };
        let join = editing("border editor", &border, on_change, |border, _: ()| {
            border.radii = [border.radii[0]; 4]
        });
        let corners = ToggleButton::new(
            (id.clone(), "corners"),
            ToggleItem::new("corners")
                .icon(IconName::SquareRoundCorner)
                .tooltip("Each corner its own"),
            split,
        )
        .on_toggle(move |on, window, cx| {
            log::info!(
                "border editor: corners {}",
                if on { "split" } else { "joined" }
            );
            parted.update(cx, |parted, cx| {
                *parted = on;
                cx.notify();
            });
            if !on && uneven {
                join((), window, cx);
            }
        });
        let corner = |ix: usize| {
            let edit = editing(
                "border editor",
                &border,
                on_change,
                move |border, value: f64| border.radii[ix] = value as f32,
            );
            scrub(CORNERS[ix], border.radii[ix]).on_change(edit)
        };
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(color_well(
                        (id.clone(), "color").into(),
                        border.color,
                        editing(
                            "border editor",
                            &border,
                            on_change,
                            |border, color: Hsla| border.color = color,
                        ),
                    ))
                    .child(div().flex_1().min_w_0().child(
                        scrub("Width", border.width).on_change(at(|b, v| b.width = v as f32)),
                    )),
            )
            .child(dash)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().flex_1().min_w_0().child(if split {
                        Ellipsis::new("Corners").into_any_element()
                    } else {
                        scrub("Radius", border.radii[0])
                            .on_change(at(|b, v| b.radii = [v as f32; 4]))
                            .into_any_element()
                    }))
                    .child(corners),
            )
            .when(split, |editor| {
                editor
                    .child(pair(corner(0), corner(1)))
                    .child(pair(corner(3), corner(2)))
            })
    }
}
