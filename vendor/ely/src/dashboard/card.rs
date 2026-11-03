use gpui::{
    AnyElement, App, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div,
};

use crate::{
    theme::{ActiveTheme, Radius, TextSize},
    typography::Ellipsis,
};

/// A widget's frame: its title with a line under it, actions at the end of the title's line, its body, and an optional footer. It fills the box it sits in, as a dashboard tile.
#[derive(IntoElement)]
pub struct DashboardCard {
    title: SharedString,
    subtitle: Option<SharedString>,
    actions: Vec<AnyElement>,
    body: Option<AnyElement>,
    footer: Option<AnyElement>,
}

impl DashboardCard {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            actions: Vec::new(),
            body: None,
            footer: None,
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

    pub fn body(mut self, body: impl IntoElement) -> Self {
        self.body = Some(body.into_any_element());
        self
    }

    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }
}

impl RenderOnce for DashboardCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .h_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.surface)
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(Ellipsis::new(self.title)),
                            )
                            .children(self.subtitle.map(|text| {
                                div()
                                    .text_size(theme.text_size(TextSize::Xs))
                                    .text_color(theme.colors.fg_muted)
                                    .child(Ellipsis::new(text))
                            })),
                    )
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap_1()
                            .children(self.actions),
                    ),
            )
            .child(div().flex_1().min_h_0().children(self.body))
            .children(self.footer.map(|footer| {
                div()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(theme.colors.fg_subtle)
                    .child(footer)
            }))
    }
}
