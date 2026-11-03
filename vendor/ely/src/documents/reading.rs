use std::{collections::BTreeMap, rc::Rc};

use gpui::{
    AnyElement, App, Bounds, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, ScrollHandle, SharedString, Styled, Window, canvas, div, fill, prelude::*, size,
};
use smallvec::SmallVec;

use crate::{
    forms::Run,
    layout::Collapsible,
    overlays::HoverCard,
    primitives::{hand_back, hold_focus},
    theme::{ActiveTheme, ContainerSize, TextSize},
};

/// How far a note's number rides above the line, in its own size.
const RAISE: f32 = 0.4;

/// A note's mark in running text, a small number; resting the pointer on it shows the note.
#[derive(IntoElement)]
pub struct Footnote {
    id: ElementId,
    number: usize,
    note: SharedString,
}

impl Footnote {
    pub fn new(id: impl Into<ElementId>, number: usize, note: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            number,
            note: note.into(),
        }
    }
}

impl RenderOnce for Footnote {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (link, note, number) = (theme.colors.link, self.note, self.number);
        let prose = theme.prose_width();
        HoverCard::new(
            self.id,
            div()
                .relative()
                .bottom(theme.text_size(TextSize::Xs) * RAISE)
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(link)
                .cursor_default()
                .child(format!("{number}")),
            move |_, _| div().max_w(prose).child(format!("{number}. {note}")),
        )
    }
}

/// A term whose meaning is a rest of the pointer away: underlined in dashes, its definition in a card.
#[derive(IntoElement)]
pub struct TooltipTerm {
    id: ElementId,
    term: SharedString,
    meaning: SharedString,
}

impl TooltipTerm {
    pub fn new(
        id: impl Into<ElementId>,
        term: impl Into<SharedString>,
        meaning: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            term: term.into(),
            meaning: meaning.into(),
        }
    }
}

impl RenderOnce for TooltipTerm {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (term, meaning, prose) = (self.term.clone(), self.meaning, theme.prose_width());
        HoverCard::new(
            self.id,
            div()
                .border_b_1()
                .border_dashed()
                .border_color(theme.colors.fg_subtle)
                .child(self.term),
            move |_, cx| {
                div()
                    .max_w(prose)
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().font_weight(FontWeight::MEDIUM).child(term.clone()))
                    .child(
                        div()
                            .text_color(cx.theme().colors.fg_muted)
                            .child(meaning.clone()),
                    )
            },
        )
    }
}

/// Terms and what they mean, in order, under the letter each starts with.
#[derive(IntoElement)]
pub struct Glossary {
    entries: Vec<(SharedString, SharedString)>,
}

impl Glossary {
    pub fn new(
        entries: impl IntoIterator<Item = (impl Into<SharedString>, impl Into<SharedString>)>,
    ) -> Self {
        Self {
            entries: entries
                .into_iter()
                .map(|(term, meaning)| (term.into(), meaning.into()))
                .collect(),
        }
    }
}

/// Entries under their first letter, the letters and each group's terms in order, ignoring case.
pub(crate) fn lettered(
    entries: &[(SharedString, SharedString)],
) -> BTreeMap<char, Vec<(SharedString, SharedString)>> {
    let mut groups: BTreeMap<char, Vec<(SharedString, SharedString)>> = BTreeMap::new();
    for (term, meaning) in entries {
        let letter = term
            .chars()
            .next()
            .expect("a glossary term has a first letter")
            .to_uppercase()
            .next()
            .expect("a letter has an upper case");
        groups
            .entry(letter)
            .or_default()
            .push((term.clone(), meaning.clone()));
    }
    for group in groups.values_mut() {
        group.sort_by_key(|(term, _)| term.to_lowercase());
    }
    groups
}

impl RenderOnce for Glossary {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        div()
            .flex()
            .flex_col()
            .gap_4()
            .children(lettered(&self.entries).into_iter().map(|(letter, group)| {
                div()
                    .flex()
                    .gap_4()
                    .child(
                        div()
                            .flex_none()
                            .w_6()
                            .text_size(theme.text_size(TextSize::Lg))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.fg_subtle)
                            .child(letter.to_string()),
                    )
                    .child(div().flex_1().min_w_0().flex().flex_col().gap_2().children(
                        group.into_iter().map(|(term, meaning)| {
                            div()
                                .flex()
                                .flex_col()
                                .child(div().font_weight(FontWeight::MEDIUM).child(term))
                                .child(div().text_color(colors.fg_muted).child(meaning))
                        }),
                    ))
            }))
    }
}

/// How far a reader is through what `scroll` scrolls, as a thin bar that fills from the left.
#[derive(IntoElement)]
pub struct ReadingProgress {
    scroll: ScrollHandle,
}

impl ReadingProgress {
    pub fn new(scroll: &ScrollHandle) -> Self {
        Self {
            scroll: scroll.clone(),
        }
    }
}

/// The share of `max` that `offset` has scrolled; all of it when nothing scrolls.
pub(crate) fn read_share(offset: f32, max: f32) -> f32 {
    if max <= 0.0 {
        return 1.0;
    }
    (-offset / max).clamp(0.0, 1.0)
}

impl RenderOnce for ReadingProgress {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (track, bar) = (theme.colors.border, theme.colors.accent);
        let scroll = self.scroll;
        div().h(theme.progress_thickness()).bg(track).child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    let share = read_share(
                        f32::from(scroll.offset().y),
                        f32::from(scroll.max_offset().y),
                    );
                    let filled = Bounds::new(
                        bounds.origin,
                        size(bounds.size.width * share, bounds.size.height),
                    );
                    window.paint_quad(fill(filled, bar));
                },
            )
            .size_full(),
        )
    }
}

/// Writing with nothing else in view: while on, the chrome folds away and the text sits in a centered column; Escape leaves.
#[derive(IntoElement)]
pub struct ZenMode {
    id: ElementId,
    on: bool,
    chrome: SmallVec<[AnyElement; 2]>,
    body: SmallVec<[AnyElement; 2]>,
    on_exit: Option<Run>,
}

impl ZenMode {
    pub fn new(id: impl Into<ElementId>, on: bool) -> Self {
        Self {
            id: id.into(),
            on,
            chrome: SmallVec::new(),
            body: SmallVec::new(),
            on_exit: None,
        }
    }

    /// What folds away while on, such as a toolbar.
    pub fn chrome(mut self, element: impl IntoElement) -> Self {
        self.chrome.push(element.into_any_element());
        self
    }

    /// Escape asks the owner to turn it off.
    pub fn on_exit(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_exit = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for ZenMode {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl RenderOnce for ZenMode {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (on, exit) = (self.on, self.on_exit);
        let held = hold_focus((self.id.clone(), "held"), on, window, cx);
        let focus = held.read(cx).focus.clone();
        let theme = cx.theme();
        div()
            .id(self.id.clone())
            .track_focus(&focus)
            .flex()
            .flex_col()
            .on_key_down(move |event, window, cx| {
                if on
                    && event.keystroke.key == "escape"
                    && let Some(exit) = &exit
                {
                    cx.stop_propagation();
                    log::info!("zen mode: leaves");
                    hand_back(&held, window, cx);
                    exit(window, cx);
                }
            })
            .child(Collapsible::new((self.id.clone(), "chrome"), !on).children(self.chrome))
            .child(
                div().w_full().flex().justify_center().child(
                    div()
                        .w_full()
                        .when(on, |column| {
                            column
                                .max_w(theme.container_width(ContainerSize::Sm))
                                .py_8()
                        })
                        .children(self.body),
                ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terms_group_under_their_letter_in_order() {
        let entries: Vec<(SharedString, SharedString)> = [
            ("lift", "a"),
            ("Accent", "b"),
            ("leading", "c"),
            ("alpha", "d"),
        ]
        .into_iter()
        .map(|(term, meaning)| (term.into(), meaning.into()))
        .collect();
        let groups = lettered(&entries);
        let shape: Vec<(char, Vec<&str>)> = groups
            .iter()
            .map(|(letter, group)| {
                (
                    *letter,
                    group.iter().map(|(term, _)| term.as_ref()).collect(),
                )
            })
            .collect();
        assert_eq!(
            shape,
            [
                ('A', vec!["Accent", "alpha"]),
                ('L', vec!["leading", "lift"])
            ]
        );
    }

    #[test]
    fn progress_is_the_share_scrolled() {
        assert_eq!(read_share(-50.0, 200.0), 0.25);
        assert_eq!(read_share(0.0, 0.0), 1.0, "a text that fits is read");
        assert_eq!(read_share(-300.0, 200.0), 1.0);
    }
}
