use std::rc::Rc;

use gpui::{
    Animation, AnimationExt, App, ElementId, EntityId, FontWeight, InteractiveElement, IntoElement,
    MouseButton, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled,
    Window, div, prelude::*,
};

use crate::{
    buttons::IconButton,
    motion,
    primitives::{DragGhost, Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
    typography::Ellipsis,
};

/// One window-level tab.
#[derive(Clone)]
pub struct WindowTab {
    id: SharedString,
    title: SharedString,
    icon: Option<IconName>,
}

impl WindowTab {
    pub fn new(id: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            icon: None,
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }
}

struct TabDrag {
    owner: EntityId,
    from: usize,
}

type OnTab = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;
type OnAdd = Rc<dyn Fn(&mut Window, &mut App)>;
type OnReorder = Rc<dyn Fn(usize, usize, &mut Window, &mut App)>;

/// Browser-style tabs across the top of a window.
#[derive(IntoElement)]
pub struct TabBar {
    id: ElementId,
    tabs: Vec<WindowTab>,
    selected: Option<SharedString>,
    on_select: Option<OnTab>,
    on_close: Option<OnTab>,
    on_add: Option<OnAdd>,
    on_reorder: Option<OnReorder>,
}

impl TabBar {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            tabs: Vec::new(),
            selected: None,
            on_select: None,
            on_close: None,
            on_add: None,
            on_reorder: None,
        }
    }

    pub fn tab(mut self, tab: WindowTab) -> Self {
        self.tabs.push(tab);
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

    /// Shows a close button on the selected and hovered tab.
    pub fn on_close(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }

    /// Shows a new-tab button after the tabs.
    pub fn on_add(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_add = Some(Rc::new(handler));
        self
    }

    /// Lets tabs drag; reports `(from, to)` indexes on drop.
    pub fn on_reorder(
        mut self,
        handler: impl Fn(usize, usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_reorder = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for TabBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let owner = window
            .use_keyed_state(self.id.clone(), cx, |_, _| ())
            .entity_id();
        let theme = cx.theme();
        let (narrow, wide) = theme.tab_width();
        let enter = motion::duration(motion::BASE, cx);
        let nudge = motion::NUDGE;
        let tabs = self.tabs.into_iter().enumerate().map(|(ix, tab)| {
            let theme = cx.theme();
            let colors = &theme.colors;
            let selected = self.selected.as_ref() == Some(&tab.id);
            let group = SharedString::from(format!("window-tab-{}", tab.id));
            let (select, close, reorder) = (
                self.on_select.clone(),
                self.on_close.clone(),
                self.on_reorder.clone(),
            );
            let (id, close_id) = (tab.id.clone(), tab.id.clone());
            let (title, icon) = (tab.title.clone(), tab.icon);
            let (lit, edge) = (colors.hover, colors.focus);
            div()
                .id(tab.id.clone())
                .group(group.clone())
                .flex()
                .flex_1()
                .items_center()
                .gap_2()
                .min_w(narrow)
                .max_w(wide)
                .h(theme.control_height(ControlSize::Md))
                .pl_3()
                .pr_1()
                .rounded(theme.radius(Radius::Md))
                .border_1()
                .text_size(theme.text_size(TextSize::Sm))
                .map(|el| {
                    if selected {
                        el.bg(colors.surface)
                            .border_color(colors.border)
                            .text_color(colors.fg)
                            .font_weight(FontWeight::MEDIUM)
                    } else {
                        el.border_color(gpui::transparent_black())
                            .text_color(colors.fg_muted)
                            .hover(|style| style.bg(lit))
                    }
                })
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                .on_click(move |_, window, cx| {
                    log::info!("tab bar: select {id}");
                    if let Some(select) = &select {
                        select(&id, window, cx);
                    }
                })
                .when_some(reorder, |el, reorder| {
                    el.on_drag(TabDrag { owner, from: ix }, move |_, _, _, cx| {
                        DragGhost::new(title.clone(), icon, cx)
                    })
                    .drag_over::<TabDrag>(move |style, drag, _, _| {
                        if drag.owner == owner {
                            style.border_color(edge)
                        } else {
                            style
                        }
                    })
                    .on_drop(move |drag: &TabDrag, window, cx| {
                        if drag.owner == owner && drag.from != ix {
                            log::info!("tab bar: move {} -> {ix}", drag.from);
                            reorder(drag.from, ix, window, cx);
                        }
                    })
                })
                .when_some(tab.icon, |el, icon| {
                    el.child(Icon::new(icon).size(IconSize::Sm).color(if selected {
                        colors.fg
                    } else {
                        colors.fg_subtle
                    }))
                })
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(Ellipsis::new(tab.title.clone())),
                )
                .when_some(close, |el, close| {
                    el.child(
                        div()
                            .when(!selected, |slot| {
                                slot.invisible().group_hover(group, |style| style.visible())
                            })
                            .child(
                                IconButton::new(("close", ix), IconName::X)
                                    .size(ControlSize::Sm)
                                    .on_click(move |_, window, cx| {
                                        cx.stop_propagation();
                                        log::info!("tab bar: close {close_id}");
                                        close(&close_id, window, cx);
                                    }),
                            ),
                    )
                })
                .with_animation(
                    SharedString::from(format!("tab-in-{}", tab.id)),
                    Animation::new(enter).with_easing(motion::ease_out_cubic),
                    move |el, t| el.opacity(t).mt(nudge * (1.0 - t)),
                )
        });
        div()
            .id(self.id)
            .flex()
            .items_center()
            .gap_1()
            .min_w_0()
            .children(tabs)
            .when_some(self.on_add, |bar, add| {
                bar.child(
                    IconButton::new("new-tab", IconName::Plus)
                        .size(ControlSize::Sm)
                        .on_click(move |_, window, cx| {
                            log::info!("tab bar: new tab");
                            add(window, cx)
                        }),
                )
            })
    }
}
