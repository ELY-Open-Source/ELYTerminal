use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, prelude::*,
};

use crate::{
    buttons::{Button, ButtonVariant, IconButton},
    forms::Run,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, TextSize},
    typography::Ellipsis,
};

/// A channel's head: its mark and name with its topic under them, and beside them its members, its pinned messages and a call, each only with its handler. The actions drop below on a narrow window.
#[derive(IntoElement)]
pub struct ChannelHeader {
    id: ElementId,
    name: SharedString,
    private: bool,
    topic: Option<SharedString>,
    members: Option<(usize, Run)>,
    pins: Option<(usize, Run)>,
    on_call: Option<Run>,
}

impl ChannelHeader {
    pub fn new(id: impl Into<ElementId>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            private: false,
            topic: None,
            members: None,
            pins: None,
            on_call: None,
        }
    }

    pub fn private(mut self) -> Self {
        self.private = true;
        self
    }

    pub fn topic(mut self, topic: impl Into<SharedString>) -> Self {
        self.topic = Some(topic.into());
        self
    }

    /// How many are in it, and what a press on them does.
    pub fn members(
        mut self,
        count: usize,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.members = Some((count, Rc::new(handler)));
        self
    }

    /// How many messages are pinned, and what a press on them does.
    pub fn pins(mut self, count: usize, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.pins = Some((count, Rc::new(handler)));
        self
    }

    pub fn on_call(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_call = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ChannelHeader {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let counted =
            |key: &'static str, icon: IconName, tip: &'static str, what: Option<(usize, Run)>| {
                what.map(|(count, run)| {
                    Button::new((self.id.clone(), key), count.to_string())
                        .icon(icon)
                        .variant(ButtonVariant::Ghost)
                        .size(ControlSize::Sm)
                        .on_click(move |_, window, cx| {
                            log::info!("channel header: {tip}");
                            run(window, cx)
                        })
                })
            };
        let call = self.on_call.map(|run| {
            IconButton::new((self.id.clone(), "call"), IconName::Phone)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip("Start a call")
                .on_click(move |_, window, cx| {
                    log::info!("channel header: call");
                    run(window, cx)
                })
        });
        let mark = if self.private {
            IconName::Lock
        } else {
            IconName::Hash
        };
        div()
            .debug_selector(|| "channel-header".into())
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap_2()
            .child(
                div()
                    .flex_1()
                    .min_w(theme.label_width())
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .child(Icon::new(mark).size(IconSize::Md).color(colors.fg_muted))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(theme.text_size(TextSize::Md))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.fg)
                                    .child(Ellipsis::new(self.name)),
                            ),
                    )
                    .children(self.topic.map(|topic| {
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(colors.fg_muted)
                            .child(Ellipsis::new(topic))
                    })),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap_1()
                    .children(counted("members", IconName::Users, "members", self.members))
                    .children(counted("pins", IconName::Pin, "pinned messages", self.pins))
                    .children(call),
            )
    }
}
