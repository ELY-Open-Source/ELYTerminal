use std::f32::consts::FRAC_PI_2;

use gpui::{
    Animation, AnimationExt, App, ElementId, Hsla, IntoElement, RenderOnce, Window, radians,
};

use super::{Icon, IconName};
use crate::{
    motion,
    theme::{ActiveTheme, IconSize},
};

/// A chevron that points right when shut and down when open, turning on each change and still on first paint.
#[derive(IntoElement)]
pub struct Disclosure {
    id: ElementId,
    open: bool,
    size: IconSize,
    color: Option<Hsla>,
}

impl Disclosure {
    pub fn new(id: impl Into<ElementId>, open: bool) -> Self {
        Self {
            id: id.into(),
            open,
            size: IconSize::Xs,
            color: None,
        }
    }

    pub fn size(mut self, size: IconSize) -> Self {
        self.size = size;
        self
    }

    /// Its color; the subtle foreground unless set.
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

impl RenderOnce for Disclosure {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open = self.open;
        let turns = motion::changes((self.id.clone(), "turns"), open, window, cx);
        let color = self.color.unwrap_or(cx.theme().colors.fg_subtle);
        let turn = |share: f32| radians(share * FRAC_PI_2);
        let icon = Icon::new(IconName::ChevronRight)
            .size(self.size)
            .color(color);
        if turns == 0 {
            return icon
                .rotate(turn(if open { 1.0 } else { 0.0 }))
                .into_any_element();
        }
        icon.with_animation(
            (self.id, format!("turn-{turns}")),
            Animation::new(motion::duration(motion::FAST, cx)).with_easing(motion::ease_out_cubic),
            move |icon, t| icon.rotate(turn(if open { t } else { 1.0 - t })),
        )
        .into_any_element()
    }
}
