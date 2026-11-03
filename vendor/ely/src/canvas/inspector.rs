use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, Hsla, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div,
};

use super::{
    panel::{OnEdit, caption, editing, pair},
    shape::Shape,
};
use crate::{
    feedback::EmptyState,
    forms::{ColorPalette, ScrubInput, Switch},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, HUE_NAMES, IconSize},
    typography::Ellipsis,
};

/// How far from the origin a figure may lie, in canvas units either way.
const REACH: f64 = 100_000.0;

/// The selected layer's name and kind, its frame, its fill among the chart hues, and whether it shows and is locked. Each edit hands the owner the changed shape; with none or several selected it says so.
#[derive(IntoElement)]
pub struct InspectorPanel {
    id: ElementId,
    selection: Vec<Shape>,
    on_change: OnEdit<Shape>,
}

impl InspectorPanel {
    pub fn new(
        id: impl Into<ElementId>,
        selection: impl IntoIterator<Item = Shape>,
        on_change: impl Fn(Shape, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            selection: selection.into_iter().collect(),
            on_change: Rc::new(on_change),
        }
    }
}

impl RenderOnce for InspectorPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_change = self.on_change;
        let id = self.id;
        let [shape] = self.selection.as_slice() else {
            let title: SharedString = match self.selection.len() {
                0 => "Nothing selected".into(),
                count => format!("{count} layers selected").into(),
            };
            return EmptyState::new((id, "empty"), IconName::MousePointer2, title)
                .body("Select one layer to see its properties.")
                .into_any_element();
        };
        let on_change = &on_change;
        let at = |edit: fn(&mut Shape, f64)| editing("inspector", shape, on_change, edit);
        let scrub = |name: &'static str, value: f32, least: f64| {
            ScrubInput::new((id.clone(), name), name, value as f64)
                .range(least, REACH)
                .precision(0)
        };
        let frame = shape.frame;
        let theme = cx.theme();
        let chart = theme.colors.chart;
        let fill = theme
            .colors
            .hue(shape.hue, format_args!("layer {}", shape.key));
        let hue = editing("inspector", shape, on_change, move |shape, color: Hsla| {
            shape.hue = chart
                .iter()
                .position(|hue| *hue == color)
                .expect("the fill is a chart hue");
        });
        let shown = editing("inspector", shape, on_change, |shape, on: bool| {
            shape.hidden = !on
        });
        let locked = editing("inspector", shape, on_change, |shape, on: bool| {
            shape.locked = on
        });
        let part = |title: &'static str| div().flex().flex_col().gap_2().child(caption(title, cx));
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Icon::new(shape.kind.icon())
                            .size(IconSize::Sm)
                            .color(theme.colors.fg_muted),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(Ellipsis::new(shape.name.clone())),
                    )
                    .child(caption(shape.kind.word(), cx).flex_none()),
            )
            .child(
                part("Frame")
                    .child(pair(
                        scrub("X", frame.x, -REACH).on_change(at(|s, v| s.frame.x = v as f32)),
                        scrub("Y", frame.y, -REACH).on_change(at(|s, v| s.frame.y = v as f32)),
                    ))
                    .child(pair(
                        scrub("W", frame.w, 0.0).on_change(at(|s, v| s.frame.w = v as f32)),
                        scrub("H", frame.h, 0.0).on_change(at(|s, v| s.frame.h = v as f32)),
                    )),
            )
            .child(
                part("Fill").child(
                    ColorPalette::new((id.clone(), "fill"), HUE_NAMES.iter().copied().zip(chart))
                        .selected(fill)
                        .on_change(hue),
                ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        Switch::new((id.clone(), "visible"), !shape.hidden)
                            .label("Visible")
                            .on_change(shown),
                    )
                    .child(
                        Switch::new((id, "locked"), shape.locked)
                            .label("Locked")
                            .on_change(locked),
                    ),
            )
            .into_any_element()
    }
}
