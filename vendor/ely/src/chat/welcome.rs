use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement, IntoElement, MouseButton,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div,
    transparent_black,
};
use smallvec::SmallVec;

use crate::{
    buttons::{Button, ButtonVariant},
    forms::Pick,
    primitives::{FocusRing, Icon, IconName},
    theme::{ActiveTheme, AvatarSize, ContainerSize, IconSize, Radius, TextSize},
};

/// A greeting for the hour of the day, 0 to 23.
pub(crate) fn greeting(hour: u8) -> &'static str {
    match hour {
        5..=11 => "Good morning",
        12..=17 => "Good afternoon",
        _ => "Good evening",
    }
}

/// The first thing a new conversation shows: a greeting by the time of day, then the owner's composer, starters and cards, in a centered column that scrolls when space runs short.
#[derive(IntoElement)]
pub struct WelcomeScreen {
    id: ElementId,
    hour: u8,
    name: Option<SharedString>,
    body: SmallVec<[AnyElement; 3]>,
}

impl WelcomeScreen {
    /// `hour` is the reader's, 0 to 23.
    pub fn new(id: impl Into<ElementId>, hour: u8) -> Self {
        assert!(hour < 24, "an hour of {hour}");
        Self {
            id: id.into(),
            hour,
            name: None,
            body: SmallVec::new(),
        }
    }

    /// The reader's name, for the greeting.
    pub fn name(mut self, name: impl Into<SharedString>) -> Self {
        self.name = Some(name.into());
        self
    }
}

impl ParentElement for WelcomeScreen {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl RenderOnce for WelcomeScreen {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let words = match self.name {
            Some(name) => format!("{}, {name}", greeting(self.hour)),
            None => greeting(self.hour).to_string(),
        };
        div()
            .id(self.id)
            .size_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .items_center()
            .p_6()
            .child(div().flex_1())
            .child(
                div()
                    .flex_none()
                    .w_full()
                    .max_w(theme.container_width(ContainerSize::Md))
                    .flex()
                    .flex_col()
                    .gap_6()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap_3()
                            .child(
                                Icon::new(IconName::Sparkles)
                                    .size(IconSize::Lg)
                                    .color(colors.accent),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .text_size(theme.text_size(TextSize::Xxl))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(colors.fg)
                                    .child(words),
                            ),
                    )
                    .children(self.body),
            )
            .child(div().flex_1())
    }
}

/// Prompts to start from, as chips: each an icon and a few words; a press asks the owner to use it.
#[derive(IntoElement)]
pub struct SuggestionChips {
    id: ElementId,
    prompts: Vec<(SharedString, IconName)>,
    on_pick: Pick,
}

impl SuggestionChips {
    pub fn new(
        id: impl Into<ElementId>,
        prompts: impl IntoIterator<Item = (impl Into<SharedString>, IconName)>,
        on_pick: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            prompts: prompts
                .into_iter()
                .map(|(text, icon)| (text.into(), icon))
                .collect(),
            on_pick: Rc::new(on_pick),
        }
    }
}

impl RenderOnce for SuggestionChips {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().flex().flex_wrap().justify_center().gap_2().children(
            self.prompts
                .into_iter()
                .enumerate()
                .map(|(ix, (text, icon))| {
                    let pick = self.on_pick.clone();
                    Button::new((self.id.clone(), format!("prompt-{ix}")), text)
                        .variant(ButtonVariant::Secondary)
                        .icon(icon)
                        .on_click(move |_, window, cx| {
                            log::info!("starters: picked {ix}");
                            pick(ix, window, cx)
                        })
                }),
        )
    }
}

/// Questions to ask next, under an answer: each a quiet line; a press asks it.
#[derive(IntoElement)]
pub struct FollowUpSuggestions {
    id: ElementId,
    questions: Vec<SharedString>,
    on_pick: Pick,
}

impl FollowUpSuggestions {
    pub fn new(
        id: impl Into<ElementId>,
        questions: impl IntoIterator<Item = impl Into<SharedString>>,
        on_pick: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            questions: questions.into_iter().map(Into::into).collect(),
            on_pick: Rc::new(on_pick),
        }
    }
}

impl RenderOnce for FollowUpSuggestions {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        div()
            .flex()
            .flex_col()
            .children(
                self.questions
                    .into_iter()
                    .enumerate()
                    .map(|(ix, question)| {
                        let pick = self.on_pick.clone();
                        div()
                            .id((self.id.clone(), format!("question-{ix}")))
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_2()
                            .py_1p5()
                            .rounded(theme.radius(Radius::Md))
                            .border_1()
                            .border_color(transparent_black())
                            .tab_index(0)
                            .focus_ring(cx)
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(colors.fg_muted)
                            .cursor_pointer()
                            .hover(|row| row.bg(colors.hover).text_color(colors.fg))
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.prevent_default()
                            })
                            .on_click(move |_, window, cx| {
                                log::info!("follow-ups: picked {ix}");
                                pick(ix, window, cx)
                            })
                            .child(
                                Icon::new(IconName::CornerDownRight)
                                    .size(IconSize::Sm)
                                    .color(colors.fg_subtle),
                            )
                            .child(div().flex_1().min_w_0().child(question))
                    }),
            )
    }
}

/// What the assistant can do, as cards three across: an icon, a name and a line each; a press starts with that.
#[derive(IntoElement)]
pub struct CapabilityCards {
    id: ElementId,
    cards: Vec<(IconName, SharedString, SharedString)>,
    on_pick: Pick,
}

impl CapabilityCards {
    pub fn new(
        id: impl Into<ElementId>,
        cards: impl IntoIterator<Item = (IconName, impl Into<SharedString>, impl Into<SharedString>)>,
        on_pick: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            cards: cards
                .into_iter()
                .map(|(icon, title, line)| (icon, title.into(), line.into()))
                .collect(),
            on_pick: Rc::new(on_pick),
        }
    }
}

impl RenderOnce for CapabilityCards {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        div()
            .grid()
            .grid_cols(3)
            .gap_3()
            .children(
                self.cards
                    .into_iter()
                    .enumerate()
                    .map(|(ix, (icon, title, line))| {
                        let pick = self.on_pick.clone();
                        div()
                            .id((self.id.clone(), format!("card-{ix}")))
                            .p_3()
                            .rounded(theme.radius(Radius::Lg))
                            .border_1()
                            .border_color(colors.border)
                            .tab_index(0)
                            .focus_ring(cx)
                            .bg(colors.surface)
                            .cursor_pointer()
                            .hover(|card| card.bg(colors.hover))
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.prevent_default()
                            })
                            .on_click(move |_, window, cx| {
                                log::info!("capabilities: picked {ix}");
                                pick(ix, window, cx)
                            })
                            .child(
                                div()
                                    .mb_2()
                                    .size(theme.avatar_size(AvatarSize::Sm))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(theme.radius(Radius::Md))
                                    .bg(colors.hover)
                                    .child(
                                        Icon::new(icon).size(IconSize::Sm).color(colors.fg_muted),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(theme.text_size(TextSize::Sm))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(title),
                            )
                            .child(
                                div()
                                    .mt_0p5()
                                    .text_size(theme.text_size(TextSize::Xs))
                                    .text_color(colors.fg_muted)
                                    .child(line),
                            )
                    }),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::greeting;

    #[test]
    fn the_greeting_follows_the_hour() {
        assert_eq!(greeting(5), "Good morning");
        assert_eq!(greeting(11), "Good morning");
        assert_eq!(greeting(12), "Good afternoon");
        assert_eq!(greeting(18), "Good evening");
        assert_eq!(greeting(2), "Good evening");
    }
}
