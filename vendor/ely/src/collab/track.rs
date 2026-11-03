use std::{ops::Range, rc::Rc};

use gpui::{
    App, ElementId, FontWeight, HighlightStyle, IntoElement, ParentElement, RenderOnce,
    SharedString, StrikethroughStyle, Styled, StyledText, UnderlineStyle, Window, div, prelude::*,
};

use similar::{ChangeTag, TextDiff};

use super::peers::Peer;
use crate::{
    buttons::{Button, ButtonVariant, IconButton, SegmentedControl},
    documents::Change,
    primitives::IconName,
    theme::{ActiveTheme, AvatarSize, ControlSize, Radius, TextSize},
    typography::format::plural,
};

/// How strongly an author's color washes the words they would add.
const WASH: f32 = 0.14;

/// How someone works on a shared document: changing it, suggesting changes for others to take, or only reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditMode {
    Editing,
    Suggesting,
    Viewing,
}

const MODES: [(EditMode, &str, &str, IconName); 3] = [
    (EditMode::Editing, "editing", "Editing", IconName::Pencil),
    (
        EditMode::Suggesting,
        "suggesting",
        "Suggesting",
        IconName::MessageSquareDiff,
    ),
    (EditMode::Viewing, "viewing", "Viewing", IconName::Eye),
];

type OnMode = Rc<dyn Fn(EditMode, &mut Window, &mut App)>;

/// Picks how you work on a shared document: edit it, suggest changes for others to take, or only read.
#[derive(IntoElement)]
pub struct SuggestionMode {
    id: ElementId,
    mode: EditMode,
    on_change: OnMode,
}

impl SuggestionMode {
    pub fn new(
        id: impl Into<ElementId>,
        mode: EditMode,
        on_change: impl Fn(EditMode, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            mode,
            on_change: Rc::new(on_change),
        }
    }
}

impl RenderOnce for SuggestionMode {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let on_change = self.on_change;
        let (_, value, _, _) = MODES
            .into_iter()
            .find(|(mode, ..)| *mode == self.mode)
            .expect("every mode is listed");
        MODES
            .into_iter()
            .fold(
                SegmentedControl::new(self.id, value).size(ControlSize::Sm),
                |control, (_, value, label, icon)| control.segment(value, label, Some(icon)),
            )
            .on_change(move |value, window, cx| {
                let (mode, ..) = MODES
                    .into_iter()
                    .find(|(_, named, ..)| *named == value.as_ref())
                    .expect("a listed mode");
                log::info!("suggestion mode: {mode:?}");
                on_change(mode, window, cx)
            })
    }
}

/// A change someone suggests: who, the stretch of the text it replaces, empty to insert, and what goes there, empty to delete.
#[derive(Clone, Debug, PartialEq)]
pub struct Suggestion {
    pub author: Peer,
    pub range: Range<usize>,
    pub text: SharedString,
}

/// What a reviewer decides: take or drop one suggestion, or all of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Accept(usize),
    Reject(usize),
    AcceptAll,
    RejectAll,
}

/// The text with every suggestion shown in place: what each removes kept and marked, what it adds after it, by suggestion.
pub(crate) fn tracked(
    base: &str,
    suggestions: &[Suggestion],
) -> (String, Vec<(Range<usize>, usize, Change)>) {
    let mut text = String::new();
    let mut marks = Vec::new();
    let mut at = 0;
    for (ix, suggestion) in suggestions.iter().enumerate() {
        text.push_str(&base[at..suggestion.range.start]);
        for (part, change) in [
            (&base[suggestion.range.clone()], Change::Removed),
            (suggestion.text.as_ref(), Change::Added),
        ] {
            if !part.is_empty() {
                let start = text.len();
                text.push_str(part);
                marks.push((start..text.len(), ix, change));
            }
        }
        at = suggestion.range.end;
    }
    text.push_str(&base[at..]);
    (text, marks)
}

/// The changes that turn `base` into `edited`, word by word, as `author`'s suggestions.
pub fn suggested(base: &str, edited: &str, author: &Peer) -> Vec<Suggestion> {
    let diff = TextDiff::configure().diff_unicode_words(base, edited);
    let mut found = Vec::new();
    let mut open: Option<Suggestion> = None;
    let mut at = 0;
    for change in diff.iter_all_changes() {
        let value = change.value();
        if change.tag() == ChangeTag::Equal {
            found.extend(open.take());
            at += value.len();
            continue;
        }
        let suggestion = open.get_or_insert_with(|| Suggestion {
            author: author.clone(),
            range: at..at,
            text: SharedString::default(),
        });
        if change.tag() == ChangeTag::Delete {
            at += value.len();
            suggestion.range.end = at;
        } else {
            suggestion.text = format!("{}{value}", suggestion.text).into();
        }
    }
    found.extend(open);
    found
}

/// `base` with suggestion `ix` taken, and the others moved to match; none when `ix` or its range left `base`.
pub fn accept(
    base: &str,
    suggestions: &[Suggestion],
    ix: usize,
) -> Option<(String, Vec<Suggestion>)> {
    let Some(taken) = suggestions.get(ix) else {
        log::error!("track changes: no suggestion {ix} of {}", suggestions.len());
        return None;
    };
    if base.get(taken.range.clone()).is_none() {
        log::error!(
            "track changes: suggestion {:?} lies outside the text",
            taken.range
        );
        return None;
    }
    let text = format!(
        "{}{}{}",
        &base[..taken.range.start],
        taken.text,
        &base[taken.range.end..]
    );
    let shift = taken.text.len() as isize - taken.range.len() as isize;
    let rest = suggestions
        .iter()
        .enumerate()
        .filter(|(at, _)| *at != ix)
        .map(|(at, suggestion)| {
            let mut suggestion = suggestion.clone();
            if at > ix {
                suggestion.range = suggestion.range.start.saturating_add_signed(shift)
                    ..suggestion.range.end.saturating_add_signed(shift);
            }
            suggestion
        })
        .collect();
    Some((text, rest))
}

/// What a suggestion does, in a line: replace, add or delete, with the words.
fn described(base: &str, suggestion: &Suggestion) -> String {
    let (removed, added) = (
        base[suggestion.range.clone()].trim(),
        suggestion.text.trim(),
    );
    match (removed.is_empty(), added.is_empty()) {
        (true, true) => "Change the spacing".to_string(),
        (true, false) => format!("Add “{added}”"),
        (false, true) => format!("Delete “{removed}”"),
        (false, false) => format!("Replace “{removed}” with “{added}”"),
    }
}

type OnDecide = Rc<dyn Fn(Decision, &mut Window, &mut App)>;

/// Suggested changes in place: removed words struck and added ones underlined, each in its author's color, then each change with who made it, to accept or reject, one by one or all at once.
#[derive(IntoElement)]
pub struct TrackChanges {
    id: ElementId,
    base: SharedString,
    suggestions: Vec<Suggestion>,
    kept: Vec<usize>,
    on_decide: Option<OnDecide>,
}

impl TrackChanges {
    /// `suggestions` in order, none overlapping, their ranges in `base`.
    pub fn new(
        id: impl Into<ElementId>,
        base: impl Into<SharedString>,
        suggestions: impl IntoIterator<Item = Suggestion>,
    ) -> Self {
        let base = base.into();
        let (kept, suggestions): (Vec<usize>, Vec<Suggestion>) = suggestions
            .into_iter()
            .enumerate()
            .filter(|(_, suggestion)| {
                let inside = base.get(suggestion.range.clone()).is_some();
                if !inside {
                    log::error!(
                        "track changes: {:?} lies outside the text; left out",
                        suggestion.range
                    );
                }
                inside
            })
            .unzip();
        for suggestion in &suggestions {
            let range = &suggestion.range;
            assert!(
                !(range.is_empty() && suggestion.text.is_empty()),
                "suggestion at {range:?} changes nothing"
            );
        }
        assert!(
            suggestions
                .windows(2)
                .all(|pair| pair[0].range.end <= pair[1].range.start),
            "suggestions come in order without overlapping"
        );
        Self {
            id: id.into(),
            base,
            suggestions,
            kept,
            on_decide: None,
        }
    }

    pub fn on_decide(
        mut self,
        handler: impl Fn(Decision, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_decide = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for TrackChanges {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let thickness = theme.underline_thickness();
        let (text, marks) = tracked(&self.base, &self.suggestions);
        let styles: Vec<(Range<usize>, HighlightStyle)> = marks
            .into_iter()
            .map(|(range, ix, change)| {
                let color = self.suggestions[ix].author.color(cx);
                let style = match change {
                    Change::Removed => HighlightStyle {
                        color: Some(color),
                        strikethrough: Some(StrikethroughStyle {
                            thickness,
                            color: Some(color),
                        }),
                        ..HighlightStyle::default()
                    },
                    Change::Added => HighlightStyle {
                        background_color: Some(color.opacity(WASH)),
                        underline: Some(UnderlineStyle {
                            thickness,
                            color: Some(color),
                            wavy: false,
                        }),
                        ..HighlightStyle::default()
                    },
                };
                (range, style)
            })
            .collect();
        let decide = |decision: Decision| {
            let on_decide = self.on_decide.clone();
            move |_: &gpui::ClickEvent, window: &mut Window, cx: &mut App| {
                log::info!("track changes: {decision:?}");
                if let Some(on_decide) = &on_decide {
                    on_decide(decision, window, cx);
                }
            }
        };
        let pending = !self.suggestions.is_empty() && self.on_decide.is_some();
        let rows = self.suggestions.iter().enumerate().map(|(ix, suggestion)| {
            let at = self.kept[ix];
            let id = |what: &str| (self.id.clone(), format!("{what}-{at}"));
            div()
                .debug_selector(move || format!("suggestion {at}"))
                .flex()
                .items_center()
                .gap_2()
                .py_1p5()
                .child(suggestion.author.avatar(id("author"), AvatarSize::Xs))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .child(suggestion.author.name.clone()),
                        )
                        .child(
                            div()
                                .text_color(colors.fg_muted)
                                .child(described(&self.base, suggestion)),
                        ),
                )
                .when(self.on_decide.is_some(), |row| {
                    row.child(
                        IconButton::new(id("accept"), IconName::Check)
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Sm)
                            .tooltip("Accept")
                            .on_click(decide(Decision::Accept(at))),
                    )
                    .child(
                        IconButton::new(id("reject"), IconName::X)
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Sm)
                            .tooltip("Reject")
                            .on_click(decide(Decision::Reject(at))),
                    )
                })
        });
        div()
            .flex()
            .flex_col()
            .gap_3()
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().flex_1().text_color(colors.fg_muted).child(plural(
                        self.suggestions.len() as u64,
                        "suggestion",
                        "suggestions",
                    )))
                    .when(pending, |bar| {
                        bar.child(
                            Button::new((self.id.clone(), "reject-all"), "Reject all")
                                .variant(ButtonVariant::Ghost)
                                .size(ControlSize::Sm)
                                .on_click(decide(Decision::RejectAll)),
                        )
                        .child(
                            Button::new((self.id.clone(), "accept-all"), "Accept all")
                                .size(ControlSize::Sm)
                                .on_click(decide(Decision::AcceptAll)),
                        )
                    }),
            )
            .child(
                div()
                    .p_3()
                    .rounded(theme.radius(Radius::Md))
                    .border_1()
                    .border_color(colors.border)
                    .text_size(theme.text_size(TextSize::Base))
                    .child(StyledText::new(text).with_highlights(styles)),
            )
            .child(div().flex().flex_col().children(rows))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn suggestion(range: Range<usize>, text: &str) -> Suggestion {
        Suggestion {
            author: Peer::new("ada", "Ada", 0),
            range,
            text: text.to_string().into(),
        }
    }

    #[test]
    fn tracked_text_keeps_both_sides_of_each_change() {
        let base = "A lift blends color toward white.";
        let (text, marks) = tracked(
            base,
            &[
                suggestion(2..6, "tint"),
                suggestion(14..14, "every "),
                suggestion(19..26, ""),
            ],
        );
        assert_eq!(text, "A lifttint blends every color toward white.");
        let shown: Vec<(&str, usize, Change)> = marks
            .iter()
            .map(|(range, ix, change)| (&text[range.clone()], *ix, *change))
            .collect();
        assert_eq!(
            shown,
            [
                ("lift", 0, Change::Removed),
                ("tint", 0, Change::Added),
                ("every ", 1, Change::Added),
                (" toward", 2, Change::Removed),
            ]
        );
    }

    #[test]
    fn edits_become_suggestions_word_by_word() {
        let found = suggested(
            "A lift blends color toward white.",
            "A tint blends every color toward white.",
            &Peer::new("ada", "Ada", 0),
        );
        assert_eq!(
            found,
            [suggestion(2..6, "tint"), suggestion(14..14, "every ")]
        );
        let (text, rest) = accept("A lift blends color toward white.", &found, 0).unwrap();
        assert_eq!(
            accept(&text, &rest, 0).unwrap().0,
            "A tint blends every color toward white."
        );
    }

    #[test]
    fn accepting_one_moves_the_rest() {
        let base = "A lift blends color.";
        let all = [suggestion(2..6, "tint"), suggestion(14..14, "every ")];
        let (text, rest) = accept(base, &all, 0).unwrap();
        assert_eq!(text, "A tint blends color.");
        assert_eq!(
            rest,
            [suggestion(14..14, "every ")],
            "same length, same place"
        );
        let (text, rest) =
            accept(base, &[suggestion(2..6, "shade of"), all[1].clone()], 0).unwrap();
        assert_eq!(text, "A shade of blends color.");
        let (text, _) = accept(&text, &rest, 0).unwrap();
        assert_eq!(text, "A shade of blends every color.");
    }

    #[test]
    fn a_suggestion_gone_or_outside_the_text_is_not_taken() {
        let base = "A lift blends.";
        let inside = suggestion(2..6, "tint");
        assert_eq!(accept(base, std::slice::from_ref(&inside), 1), None);
        assert_eq!(accept("A", std::slice::from_ref(&inside), 0), None);
        let stale = suggestion(0..40, "");
        let track = TrackChanges::new("track", base, [stale, inside.clone()]);
        assert_eq!(track.suggestions, [inside]);
        assert_eq!(track.kept, [1], "decisions name the owner's index");
    }
}
