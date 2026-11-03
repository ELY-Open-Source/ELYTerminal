use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled,
    Window, div,
};
use smallvec::SmallVec;

use crate::{
    buttons::{ButtonVariant, IconButton},
    layout::Collapsible,
    primitives::IconName,
    theme::{ActiveTheme, ControlSize},
};

/// Words quoted from an earlier message, folded behind "···" until it is pressed; open, they sit behind a rule. The fold is the text's own, logged.
#[derive(IntoElement)]
pub struct QuotedText {
    id: ElementId,
    body: SmallVec<[AnyElement; 2]>,
}

impl QuotedText {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            body: SmallVec::new(),
        }
    }
}

impl ParentElement for QuotedText {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl RenderOnce for QuotedText {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open = window.use_keyed_state((self.id.clone(), "open"), cx, |_, _| false);
        let shown = *open.read(cx);
        let theme = cx.theme();
        let (id, named) = (self.id.clone(), self.id.clone());
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div().flex().child(
                    IconButton::new((self.id.clone(), "toggle"), IconName::Ellipsis)
                        .variant(ButtonVariant::Subtle)
                        .size(ControlSize::Sm)
                        .tooltip(if shown {
                            "Hide quoted text"
                        } else {
                            "Show quoted text"
                        })
                        .on_click(move |_, _, cx| {
                            log::info!("quoted text {id}: {}", if shown { "hide" } else { "show" });
                            open.update(cx, |open, cx| {
                                *open = !*open;
                                cx.notify();
                            })
                        }),
                ),
            )
            .child(
                Collapsible::new((self.id.clone(), "fold"), shown).child(
                    div()
                        .debug_selector(move || format!("quoted {named}"))
                        .pl_3()
                        .border_l_2()
                        .border_color(theme.colors.border_strong)
                        .text_color(theme.colors.fg_muted)
                        .flex()
                        .flex_col()
                        .gap_2()
                        .children(self.body),
                ),
            )
    }
}
