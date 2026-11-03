use gpui::{
    Animation, AnimationExt, App, ElementId, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div,
};

use crate::{
    buttons::IconButton,
    motion,
    primitives::{FocusRing, IconName, tab_stop},
    theme::{ActiveTheme, Radius, TextSize},
    typography::tabular,
};

/// Where a deck stands: the card shown, and whether its back faces up.
#[derive(Default)]
struct Deck {
    at: usize,
    back: bool,
}

impl Deck {
    fn turn(&mut self) {
        self.back = !self.back;
    }

    /// Goes `by` cards round a deck of `count`, front up.
    fn go(&mut self, by: isize, count: usize) {
        self.at = (self.at as isize + by).rem_euclid(count as isize) as usize;
        self.back = false;
    }
}

/// Cards with a question on the front and its answer on the back, one at a time: a press on the card turns it, and Previous and Next go round the deck, front up. Each face fades in.
#[derive(IntoElement)]
pub struct Flashcards {
    id: ElementId,
    cards: Vec<(SharedString, SharedString)>,
}

impl Flashcards {
    pub fn new(
        id: impl Into<ElementId>,
        cards: impl IntoIterator<Item = (impl Into<SharedString>, impl Into<SharedString>)>,
    ) -> Self {
        Self {
            id: id.into(),
            cards: cards
                .into_iter()
                .map(|(front, back)| (front.into(), back.into()))
                .collect(),
        }
    }
}

impl RenderOnce for Flashcards {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let count = self.cards.len();
        assert!(count > 0, "flashcards {id:?} has no cards");
        let deck = window.use_keyed_state((id.clone(), "deck"), cx, |_, _| Deck::default());
        if deck.read(cx).at >= count {
            log::error!(
                "flashcards {id:?}: card {} of {count}; the deck starts over",
                deck.read(cx).at
            );
            deck.update(cx, |deck, _| *deck = Deck::default());
        }
        let card = tab_stop((id.clone(), "card").into(), true, window, cx);
        let (at, back) = (deck.read(cx).at, deck.read(cx).back);
        let (front_words, back_words) = self.cards[at].clone();
        let turn = motion::changes((id.clone(), "face"), (at, back), window, cx);
        let theme = cx.theme();
        let colors = &theme.colors;
        let (key, caption, words) = match back {
            false => ("front", "Question", front_words),
            true => ("back", "Answer", back_words),
        };
        let face = div()
            .debug_selector(|| format!("flashcard-{key}-{words}"))
            .flex()
            .flex_1()
            .flex_col()
            .gap_2()
            .p_5()
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.fg_muted)
                    .child(caption),
            )
            .child(
                div().flex().flex_1().items_center().child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_center()
                        .text_size(theme.text_size(TextSize::Lg))
                        .text_color(colors.fg)
                        .child(words.clone()),
                ),
            )
            .with_animation(
                (id.clone(), format!("face-{turn}")),
                Animation::new(motion::duration(motion::BASE, cx))
                    .with_easing(motion::ease_out_cubic),
                move |face, t| if turn == 0 { face } else { face.opacity(t) },
            );
        let go = |by: isize| {
            let deck = deck.clone();
            move |_: &gpui::ClickEvent, _: &mut Window, cx: &mut App| {
                deck.update(cx, |deck, cx| {
                    deck.go(by, count);
                    log::info!("flashcards: card {} of {count}", deck.at + 1);
                    cx.notify();
                })
            }
        };
        let turned = deck.clone();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .id((id.clone(), "card-face"))
                    .track_focus(&card)
                    .flex()
                    .flex_col()
                    .min_h(theme.misc().card)
                    .rounded(theme.radius(Radius::Lg))
                    .border_1()
                    .border_color(colors.border)
                    .bg(colors.surface)
                    .focus_ring(cx)
                    .cursor_pointer()
                    .on_click(move |_, _, cx| {
                        turned.update(cx, |deck, cx| {
                            deck.turn();
                            log::info!(
                                "flashcards: turned to the {}",
                                if deck.back { "back" } else { "front" }
                            );
                            cx.notify();
                        })
                    })
                    .child(face),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        IconButton::new((id.clone(), "previous"), IconName::ChevronLeft)
                            .tooltip("Previous card")
                            .on_click(go(-1)),
                    )
                    .child(
                        tabular(div())
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(colors.fg_muted)
                            .child(format!("{} of {count}", at + 1)),
                    )
                    .child(
                        IconButton::new((id, "next"), IconName::ChevronRight)
                            .tooltip("Next card")
                            .on_click(go(1)),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::Deck;

    #[test]
    fn a_deck_turns_and_goes_round_front_up() {
        let mut deck = Deck::default();
        deck.turn();
        deck.turn();
        assert!(!deck.back, "a second press turns it back");
        deck.turn();
        deck.go(-1, 3);
        assert_eq!((deck.at, deck.back), (2, false));
        deck.go(1, 3);
        assert_eq!(deck.at, 0);
    }
}
