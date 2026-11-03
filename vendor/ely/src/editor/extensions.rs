use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, prelude::*,
};

use crate::{
    buttons::{Button, ButtonVariant},
    charts::compact,
    data_display::Avatar,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, AvatarSize, ControlSize, IconSize, Radius, TextSize},
    typography::Ellipsis,
};

/// Where an extension stands here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtensionState {
    Available,
    Installing,
    Installed,
    Disabled,
    /// Installed, with a newer version out.
    Outdated,
}

/// An extension in the marketplace: who makes it, what it does, how many use it and how they rate it.
#[derive(Clone, Debug, PartialEq)]
pub struct Extension {
    pub id: SharedString,
    pub name: SharedString,
    pub publisher: SharedString,
    pub description: SharedString,
    pub version: SharedString,
    pub installs: u64,
    pub rating: f32,
    pub state: ExtensionState,
}

/// What a press on an extension's button asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtensionAction {
    Install,
    Uninstall,
    Enable,
    Update,
}

type OnAction = Rc<dyn Fn(&SharedString, ExtensionAction, &mut Window, &mut App)>;

/// Extensions, each with its mark, name, maker, what it does, its reach and rating, and the one action it needs next.
#[derive(IntoElement)]
pub struct ExtensionsPanel {
    id: ElementId,
    extensions: Vec<Extension>,
    on_action: Option<OnAction>,
}

impl ExtensionsPanel {
    pub fn new(id: impl Into<ElementId>, extensions: impl IntoIterator<Item = Extension>) -> Self {
        Self {
            id: id.into(),
            extensions: extensions.into_iter().collect(),
            on_action: None,
        }
    }

    pub fn on_action(
        mut self,
        handler: impl Fn(&SharedString, ExtensionAction, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_action = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ExtensionsPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let rows = self.extensions.iter().enumerate().map(|(ix, extension)| {
            let action = match extension.state {
                ExtensionState::Available => {
                    Some((ExtensionAction::Install, "Install", ButtonVariant::Primary))
                }
                ExtensionState::Installing => None,
                ExtensionState::Installed => Some((
                    ExtensionAction::Uninstall,
                    "Uninstall",
                    ButtonVariant::Ghost,
                )),
                ExtensionState::Disabled => {
                    Some((ExtensionAction::Enable, "Enable", ButtonVariant::Secondary))
                }
                ExtensionState::Outdated => {
                    Some((ExtensionAction::Update, "Update", ButtonVariant::Primary))
                }
            };
            let button = match action {
                Some((action, label, variant)) => {
                    let (on_action, id) = (self.on_action.clone(), extension.id.clone());
                    Button::new((self.id.clone(), format!("action-{ix}")), label)
                        .variant(variant)
                        .size(ControlSize::Sm)
                        .on_click(move |_, window, cx| {
                            log::info!("extensions: {action:?} {id}");
                            if let Some(on_action) = &on_action {
                                on_action(&id, action, window, cx);
                            }
                        })
                }
                None => Button::new((self.id.clone(), format!("action-{ix}")), "Installing")
                    .variant(ButtonVariant::Secondary)
                    .size(ControlSize::Sm)
                    .loading(true),
            };
            let muted = extension.state == ExtensionState::Disabled;
            div()
                .flex()
                .items_start()
                .gap_3()
                .px_2()
                .py_2p5()
                .border_b_1()
                .border_color(colors.border.opacity(0.6))
                .when(muted, |row| row.opacity(0.6))
                .child(
                    Avatar::new(
                        (self.id.clone(), format!("avatar-{ix}")),
                        extension.name.clone(),
                    )
                    .size(AvatarSize::Lg),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .child(
                            div()
                                .flex()
                                .items_baseline()
                                .gap_2()
                                .child(
                                    div()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(colors.fg)
                                        .child(extension.name.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(theme.text_size(TextSize::Xs))
                                        .text_color(colors.fg_subtle)
                                        .child(format!("v{}", extension.version)),
                                ),
                        )
                        .child(
                            div()
                                .text_color(colors.fg_muted)
                                .child(Ellipsis::new(extension.description.clone())),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_3()
                                .text_size(theme.text_size(TextSize::Xs))
                                .text_color(colors.fg_subtle)
                                .child(extension.publisher.clone())
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(
                                            Icon::new(IconName::Download)
                                                .size(IconSize::Xs)
                                                .color(colors.fg_subtle),
                                        )
                                        .child(compact(extension.installs as f64)),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(
                                            Icon::new(IconName::Star)
                                                .size(IconSize::Xs)
                                                .color(colors.warning),
                                        )
                                        .child(format!("{:.1}", extension.rating)),
                                ),
                        ),
                )
                .child(button)
        });
        div()
            .flex()
            .flex_col()
            .rounded(theme.radius(Radius::Md))
            .text_size(theme.text_size(TextSize::Sm))
            .children(rows)
    }
}
