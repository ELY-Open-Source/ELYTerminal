use std::{ops::Range, rc::Rc};

use gpui::{
    App, ElementId, FontWeight, HighlightStyle, InteractiveElement, IntoElement, MouseButton,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, StrikethroughStyle,
    Styled, StyledText, Window, div, prelude::*,
};
use jiff::Timestamp;
use similar::{ChangeTag, TextDiff};

use super::{markdown::words, render::MarkdownRenderer};
use crate::{
    buttons::{Button, ButtonVariant, SegmentedControl},
    data_display::Avatar,
    forms::Pick,
    theme::{ActiveTheme, AvatarSize, ControlSize, Radius, TextSize},
    typography::{DateTimeText, format::plural},
};

/// How a stretch of a prose diff changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Change {
    Added,
    Removed,
}

/// `before` and `after` as one text, word by word: every word of both, the added and removed ones marked.
pub(crate) fn prose_diff(before: &str, after: &str) -> (String, Vec<(Range<usize>, Change)>) {
    let diff = TextDiff::configure().diff_unicode_words(before, after);
    let mut text = String::new();
    let mut marks: Vec<(Range<usize>, Change)> = Vec::new();
    for change in diff.iter_all_changes() {
        let start = text.len();
        text.push_str(change.value());
        let kind = match change.tag() {
            ChangeTag::Equal => continue,
            ChangeTag::Insert => Change::Added,
            ChangeTag::Delete => Change::Removed,
        };
        match marks.last_mut() {
            Some((range, last)) if *last == kind && range.end == start => range.end = text.len(),
            _ => marks.push((start..text.len(), kind)),
        }
    }
    (text, marks)
}

/// How many words the stretches marked `kind` hold.
pub(crate) fn changed_words(text: &str, marks: &[(Range<usize>, Change)], kind: Change) -> usize {
    marks
        .iter()
        .filter(|(_, change)| *change == kind)
        .map(|(range, _)| words(&text[range.clone()]))
        .sum()
}

/// What changed between two versions of a page, word by word: added words washed, removed ones struck through.
#[derive(IntoElement)]
pub struct PageHistoryDiff {
    before: SharedString,
    after: SharedString,
}

impl PageHistoryDiff {
    pub fn new(before: impl Into<SharedString>, after: impl Into<SharedString>) -> Self {
        Self {
            before: before.into(),
            after: after.into(),
        }
    }
}

impl RenderOnce for PageHistoryDiff {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let (text, marks) = prose_diff(&self.before, &self.after);
        let (added, removed) = (
            changed_words(&text, &marks, Change::Added),
            changed_words(&text, &marks, Change::Removed),
        );
        let styles = marks.iter().map(|(range, change)| {
            let style = match change {
                Change::Added => HighlightStyle {
                    background_color: Some(colors.success.opacity(0.16)),
                    ..HighlightStyle::default()
                },
                Change::Removed => HighlightStyle {
                    color: Some(colors.danger),
                    strikethrough: Some(StrikethroughStyle {
                        thickness: theme.underline_thickness(),
                        color: Some(colors.danger),
                    }),
                    ..HighlightStyle::default()
                },
            };
            (range.clone(), style)
        });
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .gap_2()
                    .text_size(theme.text_size(TextSize::Sm))
                    .child(
                        div()
                            .text_color(colors.success)
                            .child(format!("+{}", plural(added as u64, "word", "words"))),
                    )
                    .child(
                        div()
                            .text_color(colors.danger)
                            .child(format!("−{}", plural(removed as u64, "word", "words"))),
                    ),
            )
            .child(StyledText::new(text).with_highlights(styles.collect::<Vec<_>>()))
    }
}

/// A saved version of a page: when, by whom, and its text as markdown.
#[derive(Clone, Debug, PartialEq)]
pub struct Version {
    pub at: Timestamp,
    pub author: SharedString,
    pub markdown: SharedString,
}

/// A page's versions, newest first, by time and author: pick one to read it or to see what it changed from the one before, and restore it.
#[derive(IntoElement)]
pub struct VersionHistory {
    id: ElementId,
    versions: Vec<Version>,
    selected: usize,
    on_select: Option<Pick>,
    on_restore: Option<Pick>,
}

impl VersionHistory {
    /// `versions` newest first.
    pub fn new(
        id: impl Into<ElementId>,
        versions: impl IntoIterator<Item = Version>,
        selected: usize,
    ) -> Self {
        let versions: Vec<Version> = versions.into_iter().collect();
        if selected >= versions.len() {
            log::error!(
                "version history: version {selected} of {}; none selected",
                versions.len()
            );
        }
        assert!(
            versions.windows(2).all(|pair| pair[0].at >= pair[1].at),
            "versions come newest first"
        );
        Self {
            id: id.into(),
            versions,
            selected,
            on_select: None,
            on_restore: None,
        }
    }

    pub fn on_select(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    pub fn on_restore(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_restore = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for VersionHistory {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let changes = window.use_keyed_state((self.id.clone(), "changes"), cx, |_, _| false);
        let showing_changes = *changes.read(cx);
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let version = self.versions.get(self.selected);
        let before = version.and_then(|_| self.versions.get(self.selected + 1));
        let list = self.versions.iter().enumerate().map(|(ix, version)| {
            let pick = self.on_select.clone();
            let lit = ix == self.selected;
            div()
                .id((self.id.clone(), format!("version-{ix}")))
                .flex()
                .items_center()
                .gap_2()
                .px_2()
                .py_1p5()
                .rounded(theme.radius(Radius::Sm))
                .cursor_pointer()
                .when(lit, |row| row.bg(colors.active))
                .when(!lit, |row| row.hover(|row| row.bg(colors.hover)))
                .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                .when_some(pick, |row, pick| {
                    row.on_click(move |_, window, cx| pick(ix, window, cx))
                })
                .child(
                    Avatar::new(
                        (self.id.clone(), format!("author-{ix}")),
                        version.author.clone(),
                    )
                    .size(AvatarSize::Xs),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(theme.text_size(TextSize::Sm))
                                .child(DateTimeText::new(version.at).pattern("%b %-d, %H:%M")),
                        )
                        .child(
                            div()
                                .text_size(theme.text_size(TextSize::Xs))
                                .text_color(colors.fg_subtle)
                                .child(if ix == 0 {
                                    format!("{} · current", version.author)
                                } else {
                                    version.author.to_string()
                                }),
                        ),
                )
        });
        let view = SegmentedControl::new(
            (self.id.clone(), "view"),
            if showing_changes { "changes" } else { "read" },
        )
        .size(ControlSize::Sm)
        .segment("read", "Read", None)
        .segment("changes", "Changes", None)
        .on_change({
            let changes = changes.clone();
            move |value, _, cx| {
                changes.update(cx, |changes, cx| {
                    *changes = value.as_ref() == "changes";
                    cx.notify();
                })
            }
        });
        let restore = self
            .on_restore
            .clone()
            .filter(|_| self.selected > 0 && version.is_some())
            .map(|restore| {
                let at = self.selected;
                Button::new((self.id.clone(), "restore"), "Restore this version")
                    .variant(ButtonVariant::Primary)
                    .size(ControlSize::Sm)
                    .on_click(move |_, window, cx| {
                        log::info!("version history: restore {at}");
                        restore(at, window, cx)
                    })
            });
        let body = match (version, showing_changes, before) {
            (None, ..) => div()
                .text_color(colors.fg_subtle)
                .child("No version is selected.")
                .into_any_element(),
            (Some(version), true, Some(before)) => {
                PageHistoryDiff::new(before.markdown.clone(), version.markdown.clone())
                    .into_any_element()
            }
            (Some(_), true, None) => div()
                .text_color(colors.fg_subtle)
                .child("The first version; nothing came before it.")
                .into_any_element(),
            (Some(version), false, _) => MarkdownRenderer::new(
                (self.id.clone(), format!("text-{}", self.selected)),
                version.markdown.clone(),
            )
            .into_any_element(),
        };
        div()
            .flex()
            .size_full()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border)
            .overflow_hidden()
            .child(
                div()
                    .id((self.id.clone(), "versions"))
                    .flex_none()
                    .w(theme.sidebar_width(false))
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .p_2()
                    .border_r_1()
                    .border_color(colors.border)
                    .child(
                        div()
                            .px_2()
                            .pb_1()
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg_subtle)
                            .font_weight(FontWeight::MEDIUM)
                            .child("Versions"),
                    )
                    .children(list),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .px_4()
                            .py_2()
                            .border_b_1()
                            .border_color(colors.border)
                            .child(view)
                            .children(restore),
                    )
                    .child(
                        div()
                            .id((self.id.clone(), "body"))
                            .flex_1()
                            .overflow_y_scroll()
                            .p_4()
                            .child(body),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_prose_diff_keeps_both_sides_and_marks_the_changes() {
        let (text, marks) = prose_diff("A lift blends color.", "A lift blends every color gently.");
        let shown: Vec<(&str, Change)> = marks
            .iter()
            .map(|(range, change)| (&text[range.clone()], *change))
            .collect();
        assert_eq!(text, "A lift blends every color gently.");
        assert_eq!(
            shown,
            [("every ", Change::Added), (" gently", Change::Added)]
        );
        let (text, marks) = prose_diff("keep old words", "keep new words");
        let shown: Vec<(&str, Change)> = marks
            .iter()
            .map(|(range, change)| (&text[range.clone()], *change))
            .collect();
        assert_eq!(shown, [("old", Change::Removed), ("new", Change::Added)]);
    }

    #[test]
    fn punctuation_is_not_a_changed_word() {
        let (text, marks) = prose_diff("white in gamma", "white, gently and in gamma");
        assert_eq!(changed_words(&text, &marks, Change::Added), 2);
        assert_eq!(changed_words(&text, &marks, Change::Removed), 0);
    }
}
