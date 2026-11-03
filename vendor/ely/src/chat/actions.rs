use std::rc::Rc;

use gpui::{
    App, ElementId, Entity, Focusable, FontWeight, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, Styled, Window, div, prelude::*,
};

use crate::{
    buttons::{Button, ButtonVariant, CopyButton, IconButton, ToggleButton, ToggleItem},
    forms::{Input, Pick, Run, Submit, TextInput},
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, TextSize},
    typography::{Ellipsis, tabular},
};

type OnRate = Rc<dyn Fn(Option<bool>, &mut Window, &mut App)>;

/// Asks for another answer to the same message.
pub fn regenerate_button(id: impl Into<ElementId>) -> Button {
    Button::new(id, "Regenerate")
        .variant(ButtonVariant::Ghost)
        .size(ControlSize::Sm)
        .icon(IconName::RefreshCw)
}

/// Stops an answer while it streams.
pub fn stop_button(id: impl Into<ElementId>) -> Button {
    Button::new(id, "Stop")
        .variant(ButtonVariant::Secondary)
        .size(ControlSize::Sm)
        .icon(IconName::CircleStop)
}

/// Picks an answer up where it was cut off.
pub fn continue_button(id: impl Into<ElementId>) -> Button {
    Button::new(id, "Continue")
        .variant(ButtonVariant::Secondary)
        .size(ControlSize::Sm)
        .icon(IconName::StepForward)
}

/// A message quoted in a reply: a rule, who wrote it and the start of what they said; in the composer, a way to take it off.
#[derive(IntoElement)]
pub struct QuoteReply {
    id: ElementId,
    author: SharedString,
    text: SharedString,
    on_remove: Option<Run>,
}

impl QuoteReply {
    pub fn new(
        id: impl Into<ElementId>,
        author: impl Into<SharedString>,
        text: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            author: author.into(),
            text: text.into(),
            on_remove: None,
        }
    }

    pub fn on_remove(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_remove = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for QuoteReply {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let line = self.text.lines().next().unwrap_or_default().to_string();
        div()
            .flex()
            .items_center()
            .gap_2()
            .pl_3()
            .py_1()
            .border_l_2()
            .border_color(colors.border_strong)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Xs))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.fg_muted)
                            .child(self.author),
                    )
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(colors.fg_muted)
                            .child(Ellipsis::new(line)),
                    ),
            )
            .children(self.on_remove.map(|remove| {
                IconButton::new((self.id.clone(), "remove"), IconName::X)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .tooltip("Remove quote")
                    .on_click(move |_, window, cx| remove(window, cx))
            }))
    }
}

/// What can be done with a message, in a quiet row: copy, edit, regenerate, and a good or bad mark; each shows once its handler is set, copy always.
#[derive(IntoElement)]
pub struct MessageActions {
    id: ElementId,
    text: SharedString,
    rating: Option<bool>,
    on_edit: Option<Run>,
    on_regenerate: Option<Run>,
    on_rate: Option<OnRate>,
}

impl MessageActions {
    /// `text` is what copy takes.
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            rating: None,
            on_edit: None,
            on_regenerate: None,
            on_rate: None,
        }
    }

    pub fn on_edit(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_edit = Some(Rc::new(handler));
        self
    }

    pub fn on_regenerate(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_regenerate = Some(Rc::new(handler));
        self
    }

    /// The mark given, good or bad, and what a press asks: the other mark, or none to take it back.
    pub fn rating(
        mut self,
        rating: Option<bool>,
        on_rate: impl Fn(Option<bool>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.rating = rating;
        self.on_rate = Some(Rc::new(on_rate));
        self
    }
}

impl RenderOnce for MessageActions {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let button = |key: &str, icon: IconName, tip: &'static str, run: Option<Run>| {
            run.map(|run| {
                IconButton::new((self.id.clone(), key.to_string()), icon)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .tooltip(tip)
                    .on_click(move |_, window, cx| run(window, cx))
            })
        };
        let mark = |good: bool| {
            self.on_rate.clone().map(|rate| {
                let on = self.rating == Some(good);
                let (key, icon, tip) = if good {
                    ("good", IconName::ThumbsUp, "Good answer")
                } else {
                    ("bad", IconName::ThumbsDown, "Bad answer")
                };
                ToggleButton::new(
                    (self.id.clone(), key),
                    ToggleItem::new(key).icon(icon).tooltip(tip),
                    on,
                )
                .size(ControlSize::Sm)
                .on_toggle(move |now, window, cx| {
                    let next = now.then_some(good);
                    log::info!("message actions: rated {next:?}");
                    rate(next, window, cx)
                })
            })
        };
        div()
            .flex()
            .items_center()
            .gap_0p5()
            .child(CopyButton::new(
                (self.id.clone(), "copy"),
                self.text.clone(),
            ))
            .children(button(
                "edit",
                IconName::Pencil,
                "Edit",
                self.on_edit.clone(),
            ))
            .children(button(
                "regenerate",
                IconName::RefreshCw,
                "Regenerate",
                self.on_regenerate.clone(),
            ))
            .children(mark(true))
            .children(mark(false))
    }
}

/// A sent message opened to change: the field, focused as it opens, then cancel and save. Escape cancels; the submit key saves once the words changed.
#[derive(IntoElement)]
pub struct MessageEditor {
    id: ElementId,
    field: Entity<TextInput>,
    original: SharedString,
    on_save: Run,
    on_cancel: Run,
}

impl MessageEditor {
    /// `field` holds the words being changed, `original` what was sent.
    pub fn new(
        id: impl Into<ElementId>,
        field: &Entity<TextInput>,
        original: impl Into<SharedString>,
        on_save: impl Fn(&mut Window, &mut App) + 'static,
        on_cancel: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            field: field.clone(),
            original: original.into(),
            on_save: Rc::new(on_save),
            on_cancel: Rc::new(on_cancel),
        }
    }
}

impl RenderOnce for MessageEditor {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let opened = window.use_keyed_state((self.id.clone(), "opened"), cx, |_, _| false);
        if !*opened.read(cx) {
            log::info!("message editor: takes focus");
            window.focus(&self.field.focus_handle(cx), cx);
            opened.update(cx, |opened, _| *opened = true);
        }
        let text = self.field.read(cx).text().trim().to_string();
        let changed = !text.is_empty() && text != self.original.trim();
        let (save, submit, cancel, escape) = (
            self.on_save.clone(),
            self.on_save,
            self.on_cancel.clone(),
            self.on_cancel,
        );
        div()
            .flex()
            .flex_col()
            .gap_2()
            .capture_action(move |_: &Submit, window, cx| {
                cx.stop_propagation();
                if changed {
                    log::info!("message editor: saved");
                    submit(window, cx);
                }
            })
            .on_key_down(move |event, window, cx| {
                if event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    log::info!("message editor: cancelled");
                    escape(window, cx);
                }
            })
            .child(Input::new(&self.field))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new((self.id.clone(), "cancel"), "Cancel")
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Sm)
                            .on_click(move |_, window, cx| cancel(window, cx)),
                    )
                    .child(
                        Button::new((self.id.clone(), "save"), "Save")
                            .variant(ButtonVariant::Primary)
                            .size(ControlSize::Sm)
                            .shortcut("secondary-enter")
                            .disabled(!changed)
                            .on_click(move |_, window, cx| {
                                log::info!("message editor: saved");
                                save(window, cx)
                            }),
                    ),
            )
    }
}

/// Which version of a message shows, among all it has: back, where it stands, forward.
#[derive(IntoElement)]
pub struct BranchNavigator {
    id: ElementId,
    current: Option<usize>,
    total: usize,
    on_change: Pick,
}

impl BranchNavigator {
    pub fn new(
        id: impl Into<ElementId>,
        current: usize,
        total: usize,
        on_change: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        if current >= total {
            log::error!("branch navigator: version {current} of {total}; none shown");
        }
        Self {
            id: id.into(),
            current: (current < total).then_some(current),
            total,
            on_change: Rc::new(on_change),
        }
    }
}

impl RenderOnce for BranchNavigator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (current, total) = (self.current, self.total);
        let step = |key: &str, icon: IconName, tip: &'static str, to: Option<usize>| {
            let change = self.on_change.clone();
            IconButton::new((self.id.clone(), key.to_string()), icon)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip(tip)
                .disabled(to.is_none())
                .when_some(to, |button, to| {
                    button.on_click(move |_, window, cx| {
                        log::info!("branch navigator: version {to}");
                        change(to, window, cx)
                    })
                })
        };
        div()
            .flex()
            .items_center()
            .child(step(
                "back",
                IconName::ChevronLeft,
                "Previous version",
                current.and_then(|current| current.checked_sub(1)),
            ))
            .child(
                tabular(
                    div()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(theme.colors.fg_muted),
                )
                .child(match current {
                    Some(current) => format!("{} / {total}", current + 1),
                    None => format!("– / {total}"),
                }),
            )
            .child(step(
                "forward",
                IconName::ChevronRight,
                "Next version",
                current.and_then(|current| (current + 1 < total).then_some(current + 1)),
            ))
    }
}

#[cfg(test)]
mod tests {
    use super::BranchNavigator;

    #[test]
    fn a_version_past_the_branches_shows_none() {
        let navigator =
            |current, total| BranchNavigator::new("branch", current, total, |_, _, _| {});
        assert_eq!(navigator(1, 3).current, Some(1));
        assert_eq!(navigator(3, 3).current, None);
        assert_eq!(navigator(0, 0).current, None);
    }
}
