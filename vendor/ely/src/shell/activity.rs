use std::rc::Rc;

use gpui::{
    Animation, AnimationExt, App, ElementId, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use crate::{
    data_display::CountBadge,
    motion,
    primitives::{Icon, IconName, Tooltip},
    theme::{ActiveTheme, IconSize},
};

/// One destination on an activity bar.
#[derive(Clone)]
pub struct ActivityItem {
    id: SharedString,
    icon: IconName,
    label: SharedString,
    badge: Option<u32>,
}

impl ActivityItem {
    pub fn new(
        id: impl Into<SharedString>,
        icon: IconName,
        label: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            icon,
            label: label.into(),
            badge: None,
        }
    }

    /// A count; zero hides it.
    pub fn badge(mut self, count: u32) -> Self {
        self.badge = (count > 0).then_some(count);
        self
    }
}

type OnSelect = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// Icon rail at a window's edge. Main items top, footer items bottom.
#[derive(IntoElement)]
pub struct ActivityBar {
    id: ElementId,
    items: Vec<ActivityItem>,
    footer: Vec<ActivityItem>,
    selected: Option<SharedString>,
    on_select: Option<OnSelect>,
}

impl ActivityBar {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
            footer: Vec::new(),
            selected: None,
            on_select: None,
        }
    }

    pub fn item(mut self, item: ActivityItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn footer(mut self, item: ActivityItem) -> Self {
        self.footer.push(item);
        self
    }

    pub fn selected(mut self, id: impl Into<SharedString>) -> Self {
        self.selected = Some(id.into());
        self
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ActivityBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let side = theme.rail_width();
        let mark = theme.icon_size(IconSize::Lg);
        let grow = motion::duration(motion::BASE, cx);
        let entry = |item: ActivityItem, cx: &App| {
            let theme = cx.theme();
            let colors = &theme.colors;
            let selected = self.selected.as_ref() == Some(&item.id);
            let group = SharedString::from(format!("activity-{}", item.id));
            let on_select = self.on_select.clone();
            let id = item.id.clone();
            div()
                .id(item.id.clone())
                .group(group.clone())
                .relative()
                .flex()
                .items_center()
                .justify_center()
                .size(side)
                .cursor_pointer()
                .tooltip(Tooltip::text(item.label.clone()))
                .on_click(move |_, window, cx| {
                    log::info!("activity bar: {id}");
                    if let Some(select) = &on_select {
                        select(&id, window, cx);
                    }
                })
                .child(
                    Icon::new(item.icon)
                        .size(IconSize::Lg)
                        .color(if selected {
                            colors.fg
                        } else {
                            colors.fg_subtle
                        })
                        .group_hover_color(group, colors.fg),
                )
                .when(selected, |entry| {
                    entry.child(
                        div()
                            .absolute()
                            .left_0()
                            .top_0()
                            .bottom_0()
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .h(mark)
                                    .border_l_2()
                                    .border_color(colors.fg)
                                    .with_animation(
                                        SharedString::from(format!("activity-mark-{}", item.id)),
                                        Animation::new(grow).with_easing(motion::ease_out_cubic),
                                        move |mark_line, t| mark_line.h(mark * t),
                                    ),
                            ),
                    )
                })
                .when_some(item.badge, |entry, count| {
                    entry.child(div().absolute().top_1().right_1().child(CountBadge::new(
                        SharedString::from(format!("activity-badge-{}", item.id)),
                        count as usize,
                    )))
                })
        };
        let column = || div().flex().flex_col();
        div()
            .id(self.id)
            .flex()
            .flex_col()
            .flex_none()
            .justify_between()
            .w(side)
            .h_full()
            .py_1()
            .bg(theme.colors.surface)
            .border_r_1()
            .border_color(theme.colors.border)
            .child(column().children(self.items.clone().into_iter().map(|item| entry(item, cx))))
            .child(column().children(self.footer.clone().into_iter().map(|item| entry(item, cx))))
    }
}
