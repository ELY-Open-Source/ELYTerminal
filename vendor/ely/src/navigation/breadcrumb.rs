use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use crate::{
    forms::{Choice, Listing, listing},
    primitives::{Icon, IconName, tab_stop},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
};

/// One level of a path, and the siblings a click on it lists.
#[derive(Clone, Debug)]
pub struct Crumb {
    value: SharedString,
    label: SharedString,
    icon: Option<IconName>,
    siblings: Vec<Choice>,
}

impl Crumb {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            icon: None,
            siblings: Vec::new(),
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// The level's neighbors, itself among them; a click lists them.
    pub fn siblings(mut self, siblings: impl IntoIterator<Item = Choice>) -> Self {
        self.siblings = siblings.into_iter().collect();
        if !self
            .siblings
            .iter()
            .any(|sibling| sibling.value == self.value)
        {
            log::error!(
                "crumb {} is not among its siblings; none marked",
                self.value
            );
        }
        self
    }
}

type OnLevel = Rc<dyn Fn(usize, &SharedString, &mut Window, &mut App)>;

/// A path of levels. A level with siblings lists them on click; an earlier level goes to itself.
#[derive(IntoElement)]
pub struct Breadcrumb {
    id: ElementId,
    crumbs: Vec<Crumb>,
    on_select: Option<OnLevel>,
}

impl Breadcrumb {
    pub fn new(id: impl Into<ElementId>, crumbs: impl IntoIterator<Item = Crumb>) -> Self {
        Self {
            id: id.into(),
            crumbs: crumbs.into_iter().collect(),
            on_select: None,
        }
    }

    /// Runs with the level and the value chosen there.
    pub fn on_select(
        mut self,
        handler: impl Fn(usize, &SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Breadcrumb {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        assert!(
            !self.crumbs.is_empty(),
            "breadcrumb {:?} has no levels",
            self.id
        );
        let last = self.crumbs.len() - 1;
        let mut levels: Vec<AnyElement> = Vec::new();
        for (level, crumb) in self.crumbs.into_iter().enumerate() {
            let here: ElementId = (self.id.clone(), format!("crumb-{level}")).into();
            let listed = !crumb.siblings.is_empty();
            let active = listed || level < last;
            let focus = tab_stop((here.clone(), "focus").into(), active, window, cx);
            let focused = focus.is_focused(window);
            let theme = cx.theme();
            let colors = &theme.colors;
            let fg = if level == last {
                colors.fg
            } else {
                colors.fg_muted
            };
            let trigger = div()
                .id(here.clone())
                .track_focus(&focus)
                .relative()
                .flex()
                .items_center()
                .gap_1()
                .h(theme.control_height(ControlSize::Sm))
                .px_1p5()
                .rounded(theme.radius(Radius::Md))
                .border_1()
                .border_color(if focused {
                    colors.focus
                } else {
                    gpui::transparent_black()
                })
                .text_size(theme.text_size(TextSize::Sm))
                .font_weight(if level == last {
                    FontWeight::MEDIUM
                } else {
                    FontWeight::NORMAL
                })
                .text_color(fg)
                .when(active, |crumb| {
                    crumb
                        .cursor_pointer()
                        .hover(|style| style.bg(colors.hover).text_color(colors.fg))
                })
                .when_some(crumb.icon, |el, icon| {
                    el.child(Icon::new(icon).size(IconSize::Sm).color(fg))
                })
                .child(crumb.label.clone())
                .when(listed, |el| {
                    el.child(
                        Icon::new(IconName::ChevronDown)
                            .size(IconSize::Xs)
                            .color(colors.fg_subtle),
                    )
                });
            let on_select = self.on_select.clone();
            let (id, value) = (self.id.clone(), crumb.value.clone());
            let go = move |value: &SharedString, window: &mut Window, cx: &mut App| {
                log::info!("breadcrumb {id:?}: level {level} -> {value}");
                if let Some(on_select) = &on_select {
                    on_select(level, value, window, cx);
                }
            };
            let element = if listed {
                let list = Listing {
                    id: &here,
                    rows: Rc::new(crumb.siblings),
                    selected: Some(&crumb.value),
                    focused,
                };
                listing(list, trigger, go, window, cx).into_any_element()
            } else if active {
                trigger
                    .on_click(move |_, window, cx| go(&value, window, cx))
                    .into_any_element()
            } else {
                trigger.into_any_element()
            };
            if level > 0 {
                levels.push(
                    Icon::new(IconName::ChevronRight)
                        .size(IconSize::Xs)
                        .color(cx.theme().colors.fg_subtle)
                        .into_any_element(),
                );
            }
            levels.push(element);
        }
        div()
            .id(self.id)
            .flex()
            .flex_wrap()
            .items_center()
            .gap_0p5()
            .children(levels)
    }
}
