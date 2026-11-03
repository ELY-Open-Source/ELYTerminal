use std::time::Duration;

use gpui::{
    Animation, AnimationExt, App, ClipboardItem, ElementId, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Task, Window, div,
    prelude::*,
};

use super::IconButton;
use crate::{
    motion,
    primitives::{Icon, IconName, Tooltip},
    theme::{ActiveTheme, ControlSize, IconSize},
};

const CONFIRM: Duration = Duration::from_millis(1400);

#[derive(Default)]
struct Copied {
    count: u32,
    _reset: Option<Task<()>>,
}

/// Copies text, then shows a check for a moment.
#[derive(IntoElement)]
pub struct CopyButton {
    id: ElementId,
    text: SharedString,
}

impl CopyButton {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
        }
    }
}

impl RenderOnce for CopyButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| Copied::default());
        let count = state.read(cx).count;
        let copied = state.read(cx)._reset.is_some();
        let theme = cx.theme();
        let success = theme.colors.success;
        let text = self.text;
        let button = IconButton::new("copy", IconName::Copy)
            .size(ControlSize::Sm)
            .on_click(move |_, window, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(text.to_string()));
                log::info!("copy button: {} characters copied", text.chars().count());
                let weak = state.downgrade();
                let reset = window.spawn(cx, async move |cx| {
                    cx.background_executor().timer(CONFIRM).await;
                    let cleared = cx.update(|_, cx| {
                        weak.update(cx, |copied, cx| {
                            copied._reset = None;
                            cx.notify();
                        })
                    });
                    if let Err(error) = cleared.and_then(|inner| inner) {
                        log::error!("copy button: reset lost its state: {error:#}");
                    }
                });
                state.update(cx, |copied, cx| {
                    copied.count += 1;
                    copied._reset = Some(reset);
                    cx.notify();
                });
            });
        let check = Icon::new(IconName::Check)
            .size(IconSize::Sm)
            .color(success)
            .with_animation(
                ("copied", count),
                Animation::new(motion::duration(motion::FAST, cx))
                    .with_easing(motion::ease_out_cubic),
                |icon, t| icon.rotate(gpui::radians((1.0 - t) * -0.6)),
            );
        div()
            .id(self.id)
            .flex_none()
            .size(theme.control_height(ControlSize::Sm))
            .flex()
            .items_center()
            .justify_center()
            .tooltip(Tooltip::text(if copied { "Copied" } else { "Copy" }))
            .map(|slot| {
                if copied {
                    slot.child(check)
                } else {
                    slot.child(button)
                }
            })
    }
}
