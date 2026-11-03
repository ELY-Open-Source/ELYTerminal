use std::{cell::Cell, rc::Rc};

use gpui::{
    App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, prelude::*,
};

use crate::{
    buttons::{Button, ButtonVariant},
    chat::CodeBlock,
    forms::OnFlag,
    overlays::Dialog,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
};

/// Asks before a tool runs: what it will do and its arguments, then Deny or Run. Escape, the scrim and Deny refuse it. Render it while the call waits.
#[derive(IntoElement)]
pub struct ToolApprovalDialog {
    id: ElementId,
    tool: SharedString,
    arguments: SharedString,
    detail: Option<SharedString>,
    on_answer: OnFlag,
}

impl ToolApprovalDialog {
    /// `arguments` as JSON; `on_answer` gets whether the call may run.
    pub fn new(
        id: impl Into<ElementId>,
        tool: impl Into<SharedString>,
        arguments: impl Into<SharedString>,
        on_answer: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            tool: tool.into(),
            arguments: arguments.into(),
            detail: None,
            on_answer: Rc::new(on_answer),
        }
    }

    /// A line on what the call will do.
    pub fn detail(mut self, text: impl Into<SharedString>) -> Self {
        self.detail = Some(text.into());
        self
    }
}

impl RenderOnce for ToolApprovalDialog {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let approved = Rc::new(Cell::new(false));
        let (answer, tool, chosen) = (self.on_answer, self.tool.clone(), approved.clone());
        let (deny, run) = ((self.id.clone(), "deny"), (self.id.clone(), "run"));
        Dialog::new(
            self.id.clone(),
            format!("Run {}?", self.tool),
            move |window, cx| {
                let approved = chosen.get();
                log::info!(
                    "tool approval: {tool} {}",
                    if approved { "runs" } else { "denied" }
                );
                answer(approved, window, cx)
            },
        )
        .when_some(self.detail, |dialog, detail| dialog.detail(detail))
        .child(CodeBlock::new((self.id.clone(), "arguments"), self.arguments).language("json"))
        .action(move |close| {
            Button::new(deny, "Deny")
                .variant(ButtonVariant::Secondary)
                .on_click(move |_, window, cx| close(window, cx))
        })
        .action(move |close| {
            Button::new(run, "Run")
                .variant(ButtonVariant::Primary)
                .on_click(move |_, window, cx| {
                    approved.set(true);
                    close(window, cx)
                })
        })
    }
}

/// An answer to a request for permission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permission {
    Once,
    Always,
    Deny,
}

type OnPermission = Rc<dyn Fn(Permission, &mut Window, &mut App)>;

/// Asks in place whether the agent may do something: allow once, always allow, or deny.
#[derive(IntoElement)]
pub struct PermissionPrompt {
    id: ElementId,
    title: SharedString,
    detail: Option<SharedString>,
    on_answer: OnPermission,
}

impl PermissionPrompt {
    pub fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        on_answer: impl Fn(Permission, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            detail: None,
            on_answer: Rc::new(on_answer),
        }
    }

    /// What the permission covers, such as the command or the folder.
    pub fn detail(mut self, text: impl Into<SharedString>) -> Self {
        self.detail = Some(text.into());
        self
    }
}

impl RenderOnce for PermissionPrompt {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let choice = |key: &'static str, label: &'static str, variant, answer: Permission| {
            let on_answer = self.on_answer.clone();
            Button::new((self.id.clone(), key), label)
                .variant(variant)
                .on_click(move |_, window, cx| {
                    log::info!("permission: {answer:?}");
                    on_answer(answer, window, cx)
                })
        };
        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Icon::new(IconName::Shield)
                            .size(IconSize::Sm)
                            .color(colors.fg_muted),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.fg)
                            .child(self.title.clone()),
                    ),
            )
            .children(self.detail.clone().map(|detail| {
                div().w_full().flex().child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .px_2()
                        .py_1p5()
                        .rounded(theme.radius(Radius::Md))
                        .bg(colors.sunken)
                        .font_family(theme.mono_family.clone())
                        .text_color(colors.fg_muted)
                        .child(detail),
                )
            }))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .justify_end()
                    .gap_2()
                    .child(choice(
                        "deny",
                        "Deny",
                        ButtonVariant::Ghost,
                        Permission::Deny,
                    ))
                    .child(choice(
                        "always",
                        "Always allow",
                        ButtonVariant::Secondary,
                        Permission::Always,
                    ))
                    .child(choice(
                        "once",
                        "Allow once",
                        ButtonVariant::Primary,
                        Permission::Once,
                    )),
            )
    }
}
