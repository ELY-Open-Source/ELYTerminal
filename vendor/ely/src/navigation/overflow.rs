use std::rc::Rc;

use gpui::{
    App, Div, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    Stateful, Styled, Window, div,
};

use crate::{
    forms::{Choice, Listing, OnValue, listing},
    primitives::{Icon, IconName, tab_stop},
    theme::{ActiveTheme, ControlSize, IconSize, Radius},
};

/// A small icon button that lists `rows`; picking one runs `on_pick`.
pub(crate) fn list_button(
    id: &ElementId,
    icon: IconName,
    rows: Rc<Vec<Choice>>,
    selected: Option<&SharedString>,
    on_pick: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    window: &mut Window,
    cx: &mut App,
) -> Stateful<Div> {
    let focus = tab_stop((id.clone(), "focus").into(), true, window, cx);
    let focused = focus.is_focused(window);
    let theme = cx.theme();
    let colors = &theme.colors;
    let trigger = div()
        .id((id.clone(), "button"))
        .track_focus(&focus)
        .relative()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(theme.control_height(ControlSize::Sm))
        .rounded(theme.radius(Radius::Md))
        .border_1()
        .border_color(if focused {
            colors.focus
        } else {
            gpui::transparent_black()
        })
        .cursor_pointer()
        .hover(|style| style.bg(colors.hover))
        .child(Icon::new(icon).size(IconSize::Sm).color(colors.fg_muted));
    let list = Listing {
        id,
        rows,
        selected,
        focused,
    };
    listing(list, trigger, on_pick, window, cx)
}

/// A button that lists every tab, for a strip too narrow to show them all.
#[derive(IntoElement)]
pub struct TabOverflowMenu {
    id: ElementId,
    tabs: Vec<Choice>,
    selected: Option<SharedString>,
    on_select: Option<OnValue>,
}

impl TabOverflowMenu {
    pub fn new(id: impl Into<ElementId>, tabs: impl IntoIterator<Item = Choice>) -> Self {
        Self {
            id: id.into(),
            tabs: tabs.into_iter().collect(),
            selected: None,
            on_select: None,
        }
    }

    pub fn selected(mut self, value: impl Into<SharedString>) -> Self {
        self.selected = Some(value.into());
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

impl RenderOnce for TabOverflowMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        assert!(
            !self.tabs.is_empty(),
            "tab overflow menu {:?} has no tabs",
            self.id
        );
        let (id, on_select) = (self.id.clone(), self.on_select);
        list_button(
            &self.id,
            IconName::ChevronDown,
            Rc::new(self.tabs),
            self.selected.as_ref(),
            move |value, window, cx| {
                log::info!("tab overflow menu {id:?}: {value}");
                if let Some(on_select) = &on_select {
                    on_select(value, window, cx);
                }
            },
            window,
            cx,
        )
    }
}
