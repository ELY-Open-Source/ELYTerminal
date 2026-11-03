use std::rc::Rc;

use gpui::{
    Animation, AnimationExt, AnyElement, App, ElementId, FocusHandle, FontWeight,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, Point, RenderOnce, Role,
    ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*, rems,
};

use super::button::label_size;
use crate::{
    layout::{measure_width, on_axis, reveal_when_focused, text_width},
    motion::{self, Axis, Marker, glide, measure_item, measure_origin, slide},
    primitives::{FocusRing, Icon, IconName, Tooltip, tab_stop},
    theme::{ActiveTheme, ControlSize, Elevation, Radius},
    typography::Ellipsis,
};

type OnChange = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;
type OnContext = Rc<dyn Fn(&SharedString, Point<Pixels>, &mut Window, &mut App)>;

/// Mutually exclusive segments. A thumb slides to the chosen one. Too narrow for every label in full, segments that all have icons show only their icons, each label a tooltip; otherwise labels end in an ellipsis, down to twice a control's height each. Narrower still, the strip scrolls sideways and a focused segment comes into view.
#[derive(IntoElement)]
pub struct SegmentedControl {
    id: ElementId,
    segments: Vec<(SharedString, SharedString, Option<IconName>)>,
    selected: SharedString,
    size: ControlSize,
    on_change: Option<OnChange>,
    on_context: Option<OnContext>,
}

impl SegmentedControl {
    pub fn new(id: impl Into<ElementId>, selected: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            segments: Vec::new(),
            selected: selected.into(),
            size: ControlSize::default(),
            on_change: None,
            on_context: None,
        }
    }

    pub fn segment(
        mut self,
        value: impl Into<SharedString>,
        label: impl Into<SharedString>,
        icon: Option<IconName>,
    ) -> Self {
        self.segments.push((value.into(), label.into(), icon));
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    /// A right press on a segment: its value and the pointer.
    pub fn on_context(
        mut self,
        handler: impl Fn(&SharedString, Point<Pixels>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_context = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SegmentedControl {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let values: Vec<SharedString> = self
            .segments
            .iter()
            .map(|(value, ..)| value.clone())
            .collect();
        let chosen = values.iter().position(|value| *value == self.selected);
        let (state, marker) = slide(self.id.clone(), &values, &self.selected, window, cx);
        let scroll = window
            .use_keyed_state((self.id.clone(), "scroll"), cx, |_, _| ScrollHandle::new())
            .read(cx)
            .clone();
        let mut stops: Vec<(FocusHandle, AnyElement)> = (0..values.len())
            .map(|ix| {
                let focus = tab_stop(
                    (self.id.clone(), format!("segment-{ix}")).into(),
                    true,
                    window,
                    cx,
                );
                let reveal = reveal_when_focused(
                    (self.id.clone(), format!("reveal-{ix}")),
                    &scroll,
                    &focus,
                    gpui::Axis::Horizontal,
                    window,
                    cx,
                );
                (focus, reveal.into_any_element())
            })
            .collect();
        // Last frame's strip width decides icons only.
        let strip =
            window.use_keyed_state((self.id.clone(), "strip-width"), cx, |_, _| Pixels::ZERO);
        let shown = *strip.read(cx);
        let compact = self.segments.iter().all(|(_, _, icon)| icon.is_some())
            && shown > Pixels::ZERO
            && shown < natural(&self.segments, self.size, window, cx);
        let theme = cx.theme();
        let colors = &theme.colors;
        let (text, icon_size) = label_size(self.size);
        let each = if compact { 1.0 } else { 2.0 };
        let (least, count) = (theme.control_height(self.size) * each, values.len());
        let duration = motion::duration(motion::SLOW, cx);
        let thumb = marker.map(
            |Marker {
                 from,
                 to,
                 generation,
             }| {
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .debug_selector(|| "segment-thumb".into())
                    .rounded(theme.radius(Radius::Md))
                    .bg(colors.surface)
                    .border_1()
                    .border_color(colors.border)
                    .shadow(theme.elevation(Elevation::Raised))
                    .with_animation(
                        ("segment-thumb", generation),
                        Animation::new(duration),
                        move |thumb, t| {
                            let (left, width) = glide(from, to, t);
                            thumb.left(left).w(width)
                        },
                    )
            },
        );
        let segments = self
            .segments
            .into_iter()
            .enumerate()
            .map(|(ix, (value, label, icon))| {
                let on = Some(ix) == chosen;
                let fg = if on { colors.fg } else { colors.fg_muted };
                let (focus, reveal) = stops.remove(0);
                let (named, labeled) = (value.clone(), value.clone());
                let spoken = if label.is_empty() {
                    value.clone()
                } else {
                    label.clone()
                };
                let (change, measure) = (self.on_change.clone(), state.clone());
                let (context, pressed) = (self.on_context.clone(), value.clone());
                let segment = div()
                    .id(("segment", ix))
                    .role(Role::RadioButton)
                    .aria_toggled(on.into())
                    .aria_label(spoken)
                    .relative()
                    .flex()
                    .flex_auto()
                    .min_w_0()
                    .items_center()
                    .justify_center()
                    .gap_1p5()
                    .h(theme.control_height(self.size))
                    .px(theme.control_padding(self.size))
                    .rounded(theme.radius(Radius::Md))
                    .border_1()
                    .border_color(gpui::transparent_black())
                    .text_size(theme.text_size(text))
                    .font_weight(if on {
                        FontWeight::SEMIBOLD
                    } else {
                        FontWeight::MEDIUM
                    })
                    .text_color(fg)
                    .debug_selector(move || format!("segment {named}"))
                    .cursor_pointer()
                    .track_focus(&focus)
                    .focus_ring(cx)
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    // On the segment itself, so its focus never moves first.
                    .when_some(context, |segment, context| {
                        segment.on_mouse_down(MouseButton::Right, move |event, window, cx| {
                            window.prevent_default();
                            cx.stop_propagation();
                            log::info!("segmented control: context on {pressed}");
                            context(&pressed, event.position, window, cx);
                        })
                    })
                    .on_click(move |_, window, cx| {
                        if on {
                            return;
                        }
                        log::info!("segmented control: {value}");
                        if let Some(change) = &change {
                            change(&value, window, cx);
                        }
                    })
                    .when_some(icon, |segment, icon| {
                        segment.child(Icon::new(icon).size(icon_size).color(fg))
                    })
                    .when(compact, |segment| {
                        segment.tooltip(Tooltip::text(label.clone()))
                    })
                    .when(!label.is_empty() && !compact, |segment| {
                        segment.child(
                            div()
                                .min_w_0()
                                .debug_selector(move || format!("segment-label {labeled}"))
                                .child(Ellipsis::new(label)),
                        )
                    });
                div()
                    .relative()
                    .flex()
                    .flex_auto()
                    .min_w_0()
                    .child(segment)
                    .child(measure_item(measure, ix, Axis::Horizontal))
                    .child(reveal)
            });
        let control = div()
            .id(self.id.clone())
            .relative()
            .flex()
            .flex_grow_1()
            .flex_shrink_1()
            .min_w(least * count as f32)
            .items_center()
            .child(measure_origin(state.clone(), Axis::Horizontal))
            .children(thumb)
            .children(segments);
        on_axis(
            div()
                .id((self.id, "strip"))
                .flex()
                .overflow_x_scroll()
                .track_scroll(&scroll)
                .p_0p5()
                .rounded(theme.radius(Radius::Lg))
                .bg(colors.sunken)
                .border_1()
                .border_color(colors.border),
        )
        .relative()
        .child(measure_width(strip))
        .child(control)
    }
}

/// Every label and icon in full, with padding: the width that cuts none.
fn natural(
    segments: &[(SharedString, SharedString, Option<IconName>)],
    size: ControlSize,
    window: &Window,
    cx: &App,
) -> Pixels {
    let theme = cx.theme();
    let rem = window.rem_size();
    let (text, icon) = label_size(size);
    let pad = (theme.control_padding(size).to_pixels(rem) + theme.hairline()) * 2.0;
    let glyph = theme.icon_size(icon).to_pixels(rem) + rems(0.375).to_pixels(rem);
    let strip = (rems(0.125).to_pixels(rem) + theme.hairline()) * 2.0;
    segments.iter().fold(strip, |sum, (_, label, icon)| {
        let width = text_width(label, text, FontWeight::SEMIBOLD, window, cx);
        sum + pad + width + if icon.is_some() { glyph } else { Pixels::ZERO }
    })
}
