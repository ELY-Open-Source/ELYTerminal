use std::{ops::Range, rc::Rc};

use gpui::{
    App, ElementId, FontWeight, HighlightStyle, InteractiveElement, IntoElement, MouseButton,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, StrikethroughStyle,
    Styled, StyledText, Window, div, prelude::*, transparent_black,
};
use jiff::Timestamp;
use similar::{ChangeTag, TextDiff};

use crate::{
    buttons::{Button, ButtonVariant},
    forms::Pick,
    primitives::tab_stop,
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
    typography::{Ellipsis, RelativeTime},
};

/// Runs of text, each kept, taken out or put in, over the byte range it holds.
type Runs = Vec<(ChangeTag, Range<usize>)>;

/// `new` against `old` as one text: each run kept, taken out, or put in, with the byte range it holds; runs of one kind join, and a space parts a run taken out from a run put in where they touch.
pub(crate) fn changes(old: &str, new: &str) -> (String, Runs) {
    let (mut text, mut runs) = (String::new(), Runs::new());
    let mut push = |tag: ChangeTag, part: &str| {
        let start = text.len();
        text.push_str(part);
        match runs.last_mut() {
            Some((last, range)) if *last == tag => range.end = text.len(),
            _ => runs.push((tag, start..text.len())),
        }
    };
    let (mut before, mut blank) = (ChangeTag::Equal, true);
    for change in TextDiff::from_unicode_words(old, new).iter_all_changes() {
        let (tag, part) = (change.tag(), change.value());
        let touching = before != ChangeTag::Equal && tag != ChangeTag::Equal && before != tag;
        if touching && !blank && !part.starts_with(char::is_whitespace) {
            push(ChangeTag::Equal, " ");
        }
        push(tag, part);
        (before, blank) = (tag, part.ends_with(char::is_whitespace));
    }
    (text, runs)
}

/// Two versions of a prompt as one text: words taken out struck through on the danger wash, words put in on the success wash.
#[derive(IntoElement)]
pub struct PromptDiff {
    old: SharedString,
    new: SharedString,
}

impl PromptDiff {
    pub fn new(old: impl Into<SharedString>, new: impl Into<SharedString>) -> Self {
        Self {
            old: old.into(),
            new: new.into(),
        }
    }
}

impl RenderOnce for PromptDiff {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let (text, runs) = changes(&self.old, &self.new);
        let styles = runs.into_iter().filter_map(|(tag, range)| {
            let style = match tag {
                ChangeTag::Equal => return None,
                ChangeTag::Delete => HighlightStyle {
                    background_color: Some(colors.danger_subtle),
                    color: Some(colors.fg_muted),
                    strikethrough: Some(StrikethroughStyle {
                        thickness: theme.underline_thickness(),
                        color: Some(colors.danger),
                    }),
                    ..HighlightStyle::default()
                },
                ChangeTag::Insert => HighlightStyle {
                    background_color: Some(colors.success_subtle),
                    ..HighlightStyle::default()
                },
            };
            Some((range, style))
        });
        div()
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(colors.fg)
            .child(StyledText::new(text).with_highlights(styles))
    }
}

/// What the chosen version is set against: a heading naming the version before it, and that version's text, empty for the first.
pub(crate) fn compared(versions: &[PromptVersion], chosen: usize) -> (SharedString, SharedString) {
    let heading = match versions.len() - chosen {
        1 => "The first version".into(),
        place => format!("Changes from version {}", place - 1).into(),
    };
    let base = versions
        .get(chosen + 1)
        .map(|version| version.text.clone())
        .unwrap_or_default();
    (heading, base)
}

/// A prompt as saved: its key, its words, who saved it, when, and a note on what changed.
#[derive(Clone, Debug, PartialEq)]
pub struct PromptVersion {
    pub key: SharedString,
    pub text: SharedString,
    pub author: SharedString,
    pub at: Timestamp,
    pub note: SharedString,
}

/// A prompt's versions, newest first, and what the chosen one changed from the one before it. A press or Enter chooses a version; Restore brings back an older one.
#[derive(IntoElement)]
pub struct PromptVersionHistory {
    id: ElementId,
    versions: Vec<PromptVersion>,
    chosen: Option<usize>,
    on_choose: Option<Pick>,
    on_restore: Option<Pick>,
}

impl PromptVersionHistory {
    /// `versions` run newest first; `chosen` indexes them.
    pub fn new(
        id: impl Into<ElementId>,
        versions: impl IntoIterator<Item = PromptVersion>,
        chosen: usize,
    ) -> Self {
        let versions: Vec<PromptVersion> = versions.into_iter().collect();
        let held = chosen < versions.len();
        if !held {
            log::error!(
                "prompt versions: version {chosen} of {}; none chosen",
                versions.len()
            );
        }
        Self {
            id: id.into(),
            versions,
            chosen: held.then_some(chosen),
            on_choose: None,
            on_restore: None,
        }
    }

    pub fn on_choose(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_choose = Some(Rc::new(handler));
        self
    }

    /// Gets an older version to bring back.
    pub fn on_restore(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_restore = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for PromptVersionHistory {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let pickable = self.on_choose.is_some();
        let focuses: Vec<_> = (0..self.versions.len())
            .map(|ix| {
                let focus = tab_stop(
                    (self.id.clone(), format!("focus-{ix}")).into(),
                    pickable,
                    window,
                    cx,
                );
                let focused = focus.is_focused(window);
                (focus, focused)
            })
            .collect();
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let chosen = self.chosen;
        let diff = chosen.map(|chosen| {
            let (heading, before) = compared(&self.versions, chosen);
            (heading, before, self.versions[chosen].text.clone())
        });
        let heading: SharedString = match &diff {
            Some((heading, ..)) => heading.clone(),
            None => "No version is chosen".into(),
        };
        let restore = chosen
            .filter(|chosen| *chosen > 0)
            .zip(self.on_restore.clone())
            .map(|(chosen, restore)| {
                Button::new((self.id.clone(), "restore"), "Restore")
                    .variant(ButtonVariant::Secondary)
                    .size(ControlSize::Sm)
                    .on_click(move |_, window, cx| {
                        log::info!("prompt versions: restore {chosen}");
                        restore(chosen, window, cx)
                    })
            });
        let rows = self.versions.iter().zip(focuses).enumerate().map(
            |(ix, (version, (focus, focused)))| {
                let on = chosen == Some(ix);
                let choose = self.on_choose.clone();
                div()
                    .id((self.id.clone(), format!("version-{ix}")))
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .px_2()
                    .py_1p5()
                    .rounded(theme.radius(Radius::Md))
                    .border_1()
                    .border_color(if focused {
                        colors.focus
                    } else {
                        transparent_black()
                    })
                    .when(on, |row| row.bg(colors.active))
                    .when_some(choose, |row, choose| {
                        row.track_focus(&focus)
                            .when(!on, |row| row.hover(|style| style.bg(colors.hover)))
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.prevent_default()
                            })
                            .on_click(move |_, window, cx| {
                                if on {
                                    return;
                                }
                                log::info!("prompt versions: chose {ix}");
                                choose(ix, window, cx)
                            })
                    })
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(colors.fg)
                            .child(Ellipsis::new(version.note.clone())),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg_muted)
                            .child(
                                div()
                                    .min_w_0()
                                    .child(Ellipsis::new(format!("{} ·", version.author))),
                            )
                            .child(div().flex_none().child(RelativeTime::new(
                                (self.id.clone(), format!("when-{ix}")),
                                version.at,
                            ))),
                    )
            },
        );
        div()
            .flex()
            .flex_wrap()
            .items_start()
            .gap_4()
            .child(
                div()
                    .flex_1()
                    .min_w(theme.label_width())
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .children(rows),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(theme.label_width() * 1.5)
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .rounded(theme.radius(Radius::Lg))
                    .border_1()
                    .border_color(colors.border)
                    .bg(colors.surface)
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .text_size(theme.text_size(TextSize::Xs))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(colors.fg_muted)
                                    .child(heading),
                            )
                            .children(restore),
                    )
                    .children(diff.map(|(_, before, text)| PromptDiff::new(before, text))),
            )
    }
}

#[cfg(test)]
mod tests {
    use similar::ChangeTag;

    use jiff::Timestamp;

    use super::{PromptVersion, changes, compared};

    #[test]
    fn a_version_compares_with_the_one_before_it() {
        let version = |n: usize| PromptVersion {
            key: format!("v{n}").into(),
            text: format!("text {n}").into(),
            author: "Mira".into(),
            at: Timestamp::UNIX_EPOCH,
            note: format!("note {n}").into(),
        };
        let versions: Vec<PromptVersion> = (1..=4).rev().map(version).collect();
        assert_eq!(
            compared(&versions, 2),
            ("Changes from version 1".into(), "text 1".into())
        );
        assert_eq!(
            compared(&versions, 3),
            ("The first version".into(), "".into())
        );
        assert_eq!(
            compared(&versions, 0),
            ("Changes from version 3".into(), "text 3".into())
        );
    }

    #[test]
    fn a_change_keeps_words_and_marks_what_went_and_came() {
        let marked = |old: &str, new: &str| {
            let (text, runs) = changes(old, new);
            runs.iter()
                .map(|(tag, range)| (*tag, text[range.clone()].to_string()))
                .collect::<Vec<_>>()
        };
        let went_and_came = marked("a warm tone", "a calm tone");
        assert_eq!(
            went_and_came,
            [
                (ChangeTag::Equal, "a ".into()),
                (ChangeTag::Delete, "warm".into()),
                (ChangeTag::Equal, " ".into()),
                (ChangeTag::Insert, "calm".into()),
                (ChangeTag::Equal, " tone".into())
            ],
            "a space parts a word taken out from the word put in"
        );
        let added = marked("Write short notes.", "Write plain, short notes.");
        assert!(
            added.contains(&(ChangeTag::Insert, "plain, ".into())),
            "{added:?}"
        );
        let text: String = marked("a very warm tone", "a calm tone")
            .into_iter()
            .map(|(_, part)| part)
            .collect();
        assert_eq!(text, "a very warm calm tone", "no doubled space");
        let sentences = [
            "Write short notes.",
            "Write plain, short notes.",
            "a very warm tone",
            "a calmtone",
            "You are a helpful assistant.",
            "You answer support questions. Write plain replies.",
            "Say sorry once. Offer the next step.",
            "Always say sorry.",
            "x y ",
            "x z",
        ];
        for old in sentences {
            for new in sentences {
                let (text, _) = changes(old, new);
                assert!(!text.contains("  "), "{old:?} to {new:?}: {text:?}");
            }
        }
    }
}
