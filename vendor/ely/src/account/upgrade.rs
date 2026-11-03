use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div,
};

use super::login::Run;
use crate::{
    buttons::{Button, ButtonVariant},
    layout::Card,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize, TextSize},
};

/// A way past a limit: why, what the next plan brings, and Upgrade. With benefits it stands as a paywall; without, as one line.
#[derive(IntoElement)]
pub struct UpgradePrompt {
    id: ElementId,
    title: SharedString,
    body: Option<SharedString>,
    benefits: Vec<SharedString>,
    on_upgrade: Run,
    on_dismiss: Option<Run>,
}

impl UpgradePrompt {
    pub fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        on_upgrade: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            body: None,
            benefits: Vec::new(),
            on_upgrade: Rc::new(on_upgrade),
            on_dismiss: None,
        }
    }

    pub fn body(mut self, body: impl Into<SharedString>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Something the next plan brings; any makes this a paywall.
    pub fn benefit(mut self, benefit: impl Into<SharedString>) -> Self {
        self.benefits.push(benefit.into());
        self
    }

    /// Shows "Not now".
    pub fn on_dismiss(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for UpgradePrompt {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let upgrade = self.on_upgrade;
        let theme = cx.theme();
        let wall = !self.benefits.is_empty();
        let actions = div()
            .flex()
            .flex_wrap()
            .gap_2()
            .child(
                Button::new((id.clone(), "upgrade"), "Upgrade")
                    .variant(ButtonVariant::Primary)
                    .on_click(move |_, window, cx| {
                        log::info!("upgrade prompt: upgrade");
                        upgrade(window, cx);
                    }),
            )
            .children(self.on_dismiss.map(|run| {
                Button::new((id.clone(), "dismiss"), "Not now")
                    .variant(ButtonVariant::Ghost)
                    .on_click(move |_, window, cx| run(window, cx))
            }));
        let words = div()
            .flex_1()
            .min_w(theme.label_width())
            .child(div().font_weight(FontWeight::SEMIBOLD).child(self.title))
            .children(
                self.body
                    .map(|body| div().text_color(theme.colors.fg_muted).child(body)),
            );
        let benefits = self.benefits.into_iter().map(|benefit| {
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    Icon::new(IconName::Check)
                        .size(IconSize::Sm)
                        .color(theme.colors.success),
                )
                .child(div().flex_1().min_w_0().child(benefit))
        });
        let body = match wall {
            true => div()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    div()
                        .flex()
                        .items_start()
                        .gap_3()
                        .child(
                            Icon::new(IconName::Lock)
                                .size(IconSize::Md)
                                .color(theme.colors.fg_muted),
                        )
                        .child(words),
                )
                .child(
                    div()
                        .debug_selector(|| "paywall-benefits".into())
                        .flex()
                        .flex_col()
                        .gap_1p5()
                        .children(benefits),
                )
                .child(actions),
            false => div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_3()
                .child(words)
                .child(actions),
        };
        Card::new().child(div().text_size(theme.text_size(TextSize::Sm)).child(body))
    }
}
