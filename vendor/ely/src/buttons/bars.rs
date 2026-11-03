use std::rc::Rc;

use gpui::{
    Animation, AnimationExt, AnyElement, App, Div, ElementId, IntoElement, ParentElement,
    RenderOnce, StyleRefinement, Styled, Window, div, prelude::*,
};
use smallvec::SmallVec;

use super::IconButton;
use crate::{
    motion,
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, Elevation, TextSize},
    typography::AnimatedNumber,
};

/// Actions in a row at a region's foot: docked, or floating as a pill.
#[derive(IntoElement)]
pub struct ActionBar {
    base: Div,
    floating: bool,
    children: SmallVec<[AnyElement; 4]>,
}

impl ActionBar {
    pub fn new() -> Self {
        Self {
            base: div(),
            floating: false,
            children: SmallVec::new(),
        }
    }

    /// A raised pill instead of a full-width strip.
    pub fn floating(mut self) -> Self {
        self.floating = true;
        self
    }
}

impl Default for ActionBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for ActionBar {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for ActionBar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ActionBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let bar = self.base.flex().items_center().gap_2();
        let bar = if self.floating {
            bar.p_1p5()
                .rounded_full()
                .bg(colors.overlay)
                .border_1()
                .border_color(colors.border)
                .shadow(theme.elevation(Elevation::Floating))
        } else {
            bar.justify_end()
                .px_4()
                .py_3()
                .bg(colors.surface)
                .border_t_1()
                .border_color(colors.border)
        };
        bar.children(self.children).with_animation(
            "action-bar-in",
            Animation::new(motion::duration(motion::BASE, cx)).with_easing(motion::ease_out_cubic),
            |bar, t| bar.opacity(t).mt(motion::NUDGE * 2.0 * (1.0 - t)),
        )
    }
}

type OnClear = Rc<dyn Fn(&mut Window, &mut App)>;

/// Rises when items are selected: the count, bulk actions, and a clear button.
#[derive(IntoElement)]
pub struct BulkActionBar {
    id: ElementId,
    count: usize,
    actions: SmallVec<[AnyElement; 4]>,
    on_clear: Option<OnClear>,
}

impl BulkActionBar {
    pub fn new(id: impl Into<ElementId>, count: usize) -> Self {
        Self {
            id: id.into(),
            count,
            actions: SmallVec::new(),
            on_clear: None,
        }
    }

    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }

    pub fn on_clear(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_clear = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for BulkActionBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        if self.count == 0 {
            return div().into_any_element();
        }
        let theme = cx.theme();
        let colors = &theme.colors;
        ActionBar::new()
            .floating()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .pl_2()
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(colors.fg)
                    .child(
                        AnimatedNumber::new(self.id.clone(), self.count as f64).size(TextSize::Sm),
                    )
                    .child("selected"),
            )
            .child(
                div()
                    .h(theme.control_height(ControlSize::Sm))
                    .border_l_1()
                    .border_color(colors.border),
            )
            .children(self.actions)
            .when_some(self.on_clear, |bar, clear| {
                bar.child(
                    IconButton::new("bulk-clear", IconName::X)
                        .size(ControlSize::Sm)
                        .tooltip("Clear selection")
                        .on_click(move |_, window, cx| {
                            log::info!("bulk action bar: cleared");
                            clear(window, cx)
                        }),
                )
            })
            .into_any_element()
    }
}
