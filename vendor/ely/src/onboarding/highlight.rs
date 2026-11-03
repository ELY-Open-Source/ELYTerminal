use std::rc::Rc;

use gpui::{
    Animation, AnimationExt, App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div,
};

use crate::{
    buttons::{Button, ButtonVariant},
    data_display::{Badge, Tone},
    forms::Run,
    layout::Card,
    motion,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
};

/// Word of something new: its icon, a New badge over its name, a line on what it does, then Try it and Not now. It rises in when it appears.
#[derive(IntoElement)]
pub struct FeatureHighlight {
    id: ElementId,
    icon: IconName,
    title: SharedString,
    body: SharedString,
    on_try: Run,
    on_dismiss: Run,
}

impl FeatureHighlight {
    pub fn new(
        id: impl Into<ElementId>,
        icon: IconName,
        title: impl Into<SharedString>,
        body: impl Into<SharedString>,
        on_try: impl Fn(&mut Window, &mut App) + 'static,
        on_dismiss: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            icon,
            title: title.into(),
            body: body.into(),
            on_try: Rc::new(on_try),
            on_dismiss: Rc::new(on_dismiss),
        }
    }
}

impl RenderOnce for FeatureHighlight {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let try_it = self.on_try;
        let dismiss = self.on_dismiss;
        let theme = cx.theme();
        let title = self.title.clone();
        let actions = div()
            .flex()
            .flex_wrap()
            .gap_2()
            .mt_3()
            .child(
                Button::new((id.clone(), "try"), "Try it")
                    .variant(ButtonVariant::Primary)
                    .size(ControlSize::Sm)
                    .on_click(move |_, window, cx| {
                        log::info!("feature highlight: try {title}");
                        try_it(window, cx)
                    }),
            )
            .child(
                Button::new((id.clone(), "dismiss"), "Not now")
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .on_click(move |_, window, cx| dismiss(window, cx)),
            );
        let words = div()
            .flex_1()
            .min_w_0()
            .child(div().flex().child(Badge::new("New").tone(Tone::Accent)))
            .child(
                div()
                    .mt_2()
                    .text_size(theme.text_size(TextSize::Base))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(self.title),
            )
            .child(
                div()
                    .mt_1()
                    .text_color(theme.colors.fg_muted)
                    .child(self.body),
            )
            .child(actions);
        let mark = div()
            .flex_none()
            .p_2()
            .rounded(theme.radius(Radius::Md))
            .bg(theme.colors.sunken)
            .child(
                Icon::new(self.icon)
                    .size(IconSize::Md)
                    .color(theme.colors.fg_muted),
            );
        Card::new()
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_3()
                    .text_size(theme.text_size(TextSize::Sm))
                    .child(mark)
                    .child(words),
            )
            .with_animation(
                (id, "in"),
                Animation::new(motion::duration(motion::SLOW, cx))
                    .with_easing(motion::ease_out_cubic),
                |card, t| card.opacity(t).mt(motion::NUDGE * 2.0 * (1.0 - t)),
            )
    }
}
