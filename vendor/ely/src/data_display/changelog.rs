use gpui::{
    App, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
    prelude::*,
};

use super::{Badge, Timeline, TimelineItem, Tone};
use crate::theme::ActiveTheme;

/// A kind of change, as Keep a Changelog names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Change {
    Added,
    Changed,
    Deprecated,
    Removed,
    Fixed,
    Security,
}

impl Change {
    fn label(self) -> &'static str {
        match self {
            Self::Added => "Added",
            Self::Changed => "Changed",
            Self::Deprecated => "Deprecated",
            Self::Removed => "Removed",
            Self::Fixed => "Fixed",
            Self::Security => "Security",
        }
    }

    fn tone(self) -> Tone {
        match self {
            Self::Added => Tone::Success,
            Self::Changed => Tone::Info,
            Self::Deprecated => Tone::Warning,
            Self::Removed | Self::Security => Tone::Danger,
            Self::Fixed => Tone::Neutral,
        }
    }
}

/// One version: its date, a line about it, and its changes.
pub struct Release {
    version: SharedString,
    date: SharedString,
    summary: Option<SharedString>,
    changes: Vec<(Change, SharedString)>,
}

impl Release {
    pub fn new(version: impl Into<SharedString>, date: impl Into<SharedString>) -> Self {
        Self {
            version: version.into(),
            date: date.into(),
            summary: None,
            changes: Vec::new(),
        }
    }

    pub fn summary(mut self, summary: impl Into<SharedString>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    pub fn change(mut self, kind: Change, text: impl Into<SharedString>) -> Self {
        self.changes.push((kind, text.into()));
        self
    }
}

/// Changes grouped by kind, kinds in Keep a Changelog order, each kind's changes as given.
fn grouped(changes: Vec<(Change, SharedString)>) -> Vec<(Change, Vec<SharedString>)> {
    let mut groups: Vec<(Change, Vec<SharedString>)> = Vec::new();
    for (kind, text) in changes {
        match groups.iter_mut().find(|(seen, _)| *seen == kind) {
            Some((_, texts)) => texts.push(text),
            None => groups.push((kind, vec![text])),
        }
    }
    groups.sort_by_key(|(kind, _)| *kind);
    groups
}

/// Releases down a rail in the order given; the first reads Latest.
#[derive(IntoElement, Default)]
pub struct Changelog {
    releases: Vec<Release>,
}

impl Changelog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn release(mut self, release: Release) -> Self {
        self.releases.push(release);
        self
    }
}

impl RenderOnce for Changelog {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = &cx.theme().colors;
        let (fg, subtle) = (colors.fg, colors.fg_subtle);
        self.releases
            .into_iter()
            .enumerate()
            .fold(Timeline::new(), |timeline, (ix, release)| {
                let head = div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(release.version)
                    .when(ix == 0, |head| {
                        head.child(Badge::new("Latest").tone(Tone::Accent))
                    });
                let groups = grouped(release.changes).into_iter().map(|(kind, texts)| {
                    div()
                        .flex()
                        .flex_col()
                        .items_start()
                        .gap_1()
                        .mt_3()
                        .child(Badge::new(kind.label()).tone(kind.tone()))
                        .children(texts.into_iter().map(|text| {
                            div()
                                .flex()
                                .gap_2()
                                .text_color(fg)
                                .child(div().text_color(subtle).child("•"))
                                .child(div().flex_1().min_w_0().child(text))
                        }))
                });
                timeline.item(
                    TimelineItem::new(head)
                        .tone(if ix == 0 { Tone::Accent } else { Tone::Neutral })
                        .time(release.date)
                        .children(release.summary)
                        .children(groups),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{Change, grouped};

    #[test]
    fn changes_group_by_kind_in_changelog_order() {
        let changes = vec![
            (Change::Fixed, "a crash".into()),
            (Change::Added, "a timeline".into()),
            (Change::Fixed, "a typo".into()),
            (Change::Removed, "the old feed".into()),
        ];
        let groups: Vec<_> = grouped(changes)
            .into_iter()
            .map(|(kind, texts)| {
                (
                    kind,
                    texts.iter().map(|t| t.to_string()).collect::<Vec<_>>(),
                )
            })
            .collect();
        assert_eq!(
            groups,
            vec![
                (Change::Added, vec!["a timeline".to_string()]),
                (Change::Removed, vec!["the old feed".to_string()]),
                (
                    Change::Fixed,
                    vec!["a crash".to_string(), "a typo".to_string()]
                ),
            ]
        );
    }
}
