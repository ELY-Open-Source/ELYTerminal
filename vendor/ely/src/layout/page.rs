use gpui::{
    AnyElement, App, Axis, ElementId, FontWeight, IntoElement, ParentElement, Pixels, RenderOnce,
    SharedString, Styled, Window, div, prelude::*,
};
use smallvec::SmallVec;

use super::SplitPane;
use crate::theme::{ActiveTheme, TextSize};

/// Page header and content: title, note, actions, then the body.
#[derive(IntoElement)]
pub struct Page {
    title: SharedString,
    subtitle: Option<SharedString>,
    actions: SmallVec<[AnyElement; 2]>,
    body: SmallVec<[AnyElement; 4]>,
}

impl Page {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            actions: SmallVec::new(),
            body: SmallVec::new(),
        }
    }

    pub fn subtitle(mut self, text: impl Into<SharedString>) -> Self {
        self.subtitle = Some(text.into());
        self
    }

    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }
}

impl ParentElement for Page {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl RenderOnce for Page {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .flex()
            .flex_col()
            .gap_8()
            .px_10()
            .py_8()
            .child(
                div()
                    .flex()
                    .items_end()
                    .justify_between()
                    .gap_6()
                    .pb_6()
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(theme.text_size(TextSize::Xxl))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(self.title),
                            )
                            .when_some(self.subtitle, |block, text| {
                                block.child(
                                    div()
                                        .text_size(theme.text_size(TextSize::Md))
                                        .text_color(theme.colors.fg_muted)
                                        .child(text),
                                )
                            }),
                    )
                    .child(div().flex().items_center().gap_2().children(self.actions)),
            )
            .child(div().flex().flex_col().gap_6().children(self.body))
    }
}

/// Window frame: title bar, sidebar, content, status bar.
#[derive(IntoElement)]
pub struct AppShell {
    title_bar: Option<AnyElement>,
    sidebar: Option<AnyElement>,
    status_bar: Option<AnyElement>,
    body: SmallVec<[AnyElement; 2]>,
}

impl AppShell {
    pub fn new() -> Self {
        Self {
            title_bar: None,
            sidebar: None,
            status_bar: None,
            body: SmallVec::new(),
        }
    }

    pub fn title_bar(mut self, bar: impl IntoElement) -> Self {
        self.title_bar = Some(bar.into_any_element());
        self
    }

    pub fn sidebar(mut self, sidebar: impl IntoElement) -> Self {
        self.sidebar = Some(sidebar.into_any_element());
        self
    }

    pub fn status_bar(mut self, bar: impl IntoElement) -> Self {
        self.status_bar = Some(bar.into_any_element());
        self
    }
}

impl Default for AppShell {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for AppShell {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl RenderOnce for AppShell {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.bg)
            .text_color(theme.colors.fg)
            .children(self.title_bar)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .children(self.sidebar)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .children(self.body),
                    ),
            )
            .children(self.status_bar)
    }
}

/// List on the left, the selected item on the right.
#[derive(IntoElement)]
pub struct MasterDetail {
    id: ElementId,
    master: AnyElement,
    detail: AnyElement,
    min: Pixels,
}

impl MasterDetail {
    pub fn new(
        id: impl Into<ElementId>,
        master: impl IntoElement,
        detail: impl IntoElement,
        min: Pixels,
    ) -> Self {
        Self {
            id: id.into(),
            master: master.into_any_element(),
            detail: detail.into_any_element(),
            min,
        }
    }
}

impl RenderOnce for MasterDetail {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        SplitPane::new(self.id, Axis::Horizontal, self.min)
            .sizes(&[0.36, 0.64])
            .pane(self.master)
            .pane(self.detail)
    }
}
