use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement, IntoElement, MouseButton,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::*,
};
use smallvec::SmallVec;

use crate::{
    forms::{OnFlag, Run},
    layout::Collapsible,
    primitives::{FocusRing, Icon, IconName, Tooltip},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
};

/// One place in a side navigation. A folded sidebar shows the icon alone and names it on hover.
#[derive(IntoElement)]
pub struct NavItem {
    id: ElementId,
    label: SharedString,
    icon: IconName,
    count: Option<SharedString>,
    active: bool,
    folded: bool,
    on_click: Option<Run>,
}

impl NavItem {
    pub fn new(id: impl Into<ElementId>, icon: IconName, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
            count: None,
            active: false,
            folded: false,
            on_click: None,
        }
    }

    /// Quiet text at the end, such as an unread count.
    pub fn count(mut self, count: impl Into<SharedString>) -> Self {
        self.count = Some(count.into());
        self
    }

    /// Marks the place you are.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// The icon alone, for a sidebar folded to a rail.
    pub fn folded(mut self, folded: bool) -> Self {
        self.folded = folded;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for NavItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let fg = if self.active {
            colors.fg
        } else {
            colors.fg_muted
        };
        let (id, label) = (self.id.clone(), self.label.clone());
        let item = div()
            .id(self.id)
            .relative()
            .flex()
            .items_center()
            .gap_2()
            .h(theme.control_height(ControlSize::Md))
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(gpui::transparent_black())
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(fg)
            .cursor_pointer()
            .tab_index(0)
            .focus_ring(cx)
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .map(|item| {
                if self.active {
                    item.bg(colors.hover).font_weight(FontWeight::MEDIUM).child(
                        div()
                            .absolute()
                            .left_0()
                            .top_1p5()
                            .bottom_1p5()
                            .w(theme.tab_indicator())
                            .rounded_full()
                            .bg(colors.accent),
                    )
                } else {
                    item.hover(|style| style.bg(colors.hover).text_color(colors.fg))
                }
            })
            .when_some(self.on_click, |item, click| {
                item.on_click(move |_, window, cx| {
                    log::info!("nav item {id:?}: open");
                    click(window, cx)
                })
            })
            .child(Icon::new(self.icon).size(IconSize::Sm).color(fg));
        if self.folded {
            return item
                .justify_center()
                .w(theme.control_height(ControlSize::Md))
                .tooltip(Tooltip::text(label));
        }
        item.px_2()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(label),
            )
            .when_some(self.count, |item, count| {
                item.child(div().text_color(colors.fg_subtle).child(count))
            })
    }
}

/// A titled run of nav items. The title folds it; a folded sidebar drops the title.
#[derive(IntoElement)]
pub struct NavGroup {
    id: ElementId,
    title: SharedString,
    open: bool,
    folded: bool,
    items: SmallVec<[AnyElement; 8]>,
    on_toggle: Option<OnFlag>,
}

impl NavGroup {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>, open: bool) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            open,
            folded: false,
            items: SmallVec::new(),
            on_toggle: None,
        }
    }

    /// Items only, for a sidebar folded to a rail.
    pub fn folded(mut self, folded: bool) -> Self {
        self.folded = folded;
        self
    }

    /// Runs with the state the title asks for.
    pub fn on_toggle(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for NavGroup {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.items.extend(elements);
    }
}

impl RenderOnce for NavGroup {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let items = div().flex().flex_col().gap_0p5().children(self.items);
        if self.folded {
            return div().id(self.id).child(items);
        }
        let theme = cx.theme();
        let colors = &theme.colors;
        let (open, id) = (self.open, self.id.clone());
        let toggle = self.on_toggle;
        div()
            .id(self.id.clone())
            .flex()
            .flex_col()
            .gap_0p5()
            .child(
                div()
                    .id((self.id.clone(), "title"))
                    .flex()
                    .items_center()
                    .gap_1()
                    .h(theme.control_height(ControlSize::Sm))
                    .px_2()
                    .rounded(theme.radius(Radius::Md))
                    .border_1()
                    .border_color(gpui::transparent_black())
                    .text_size(theme.text_size(TextSize::Xs))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.fg_subtle)
                    .cursor_pointer()
                    .tab_index(0)
                    .focus_ring(cx)
                    .hover(|style| style.text_color(colors.fg_muted))
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .on_click(move |_, window, cx| {
                        log::info!("nav group {id:?}: open {}", !open);
                        if let Some(toggle) = &toggle {
                            toggle(!open, window, cx);
                        }
                    })
                    .child(
                        Icon::new(if open {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        })
                        .size(IconSize::Xs)
                        .color(colors.fg_subtle),
                    )
                    .child(self.title),
            )
            .child(Collapsible::new((self.id, "items"), open).child(items))
    }
}
