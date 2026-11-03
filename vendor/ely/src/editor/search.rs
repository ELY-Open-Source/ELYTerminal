use std::{ops::Range, rc::Rc};

use gpui::{
    AnyElement, App, ElementId, FontWeight, HighlightStyle, InteractiveElement, IntoElement,
    MouseButton, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled,
    StyledText, Window, div, prelude::*,
};

use crate::{
    primitives::{Disclosure, Icon, IconName},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
    typography::format,
};

/// A line that matched: its number, its text, and where in it the matches lie.
#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    pub line: usize,
    pub text: SharedString,
    pub ranges: Vec<Range<usize>>,
}

/// A file and the lines in it that matched.
#[derive(Clone, Debug, PartialEq)]
pub struct FileHits {
    pub path: SharedString,
    pub hits: Vec<Hit>,
}

/// Characters kept before the first match when a line is long.
const LEAD: usize = 24;

/// A hit's text trimmed to start near its first match, with its ranges moved along.
fn trimmed(hit: &Hit) -> (String, Vec<Range<usize>>) {
    let first = hit.ranges.first().map_or(0, |range| range.start);
    let before = hit.text[..first].chars().count();
    let last = hit.ranges.iter().map(|range| range.end).max().unwrap_or(0);
    let text = &hit.text[..hit.text.trim_end().len().max(last)];
    if before <= LEAD {
        let start = text.len() - text.trim_start().len();
        let start = start.min(first);
        let ranges = hit
            .ranges
            .iter()
            .map(|range| range.start - start..range.end - start)
            .collect();
        return (text[start..].to_string(), ranges);
    }
    let cut = hit.text[..first]
        .char_indices()
        .nth(before - LEAD)
        .map_or(first, |(at, _)| at);
    let shown = format!("…{}", &text[cut..]);
    let shift = '…'.len_utf8();
    let ranges = hit
        .ranges
        .iter()
        .filter(|range| range.start >= cut)
        .map(|range| range.start - cut + shift..range.end - cut + shift)
        .collect();
    (shown, ranges)
}

/// How many places the files name the symbol: every match, two on one line counting twice.
fn references(files: &[FileHits]) -> usize {
    files
        .iter()
        .flat_map(|file| &file.hits)
        .map(|hit| hit.ranges.len())
        .sum()
}

type OnOpen = Rc<dyn Fn(&SharedString, usize, &mut Window, &mut App)>;
type OnPath = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// One matched line: its number, and its text trimmed near the first match with the matches marked; a replacement shows struck out beside the new text.
#[derive(IntoElement)]
pub struct SearchResultItem {
    id: ElementId,
    path: SharedString,
    hit: Hit,
    replacement: Option<SharedString>,
    on_open: Option<OnOpen>,
}

impl SearchResultItem {
    pub fn new(id: impl Into<ElementId>, path: impl Into<SharedString>, hit: Hit) -> Self {
        Self {
            id: id.into(),
            path: path.into(),
            hit,
            replacement: None,
            on_open: None,
        }
    }

    /// Previews replacing each match with this.
    pub fn replacement(mut self, text: impl Into<SharedString>) -> Self {
        self.replacement = Some(text.into());
        self
    }

    pub fn on_open(
        mut self,
        handler: impl Fn(&SharedString, usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SearchResultItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let (text, ranges) = trimmed(&self.hit);
        let (text, styles) = match &self.replacement {
            None => {
                let style = HighlightStyle {
                    background_color: Some(colors.warning.opacity(0.25)),
                    font_weight: Some(FontWeight::SEMIBOLD),
                    ..HighlightStyle::default()
                };
                (
                    text,
                    ranges
                        .into_iter()
                        .map(|range| (range, style))
                        .collect::<Vec<_>>(),
                )
            }
            Some(new) => {
                let (mut out, mut styles, mut last) = (String::new(), Vec::new(), 0);
                let old = HighlightStyle {
                    background_color: Some(colors.danger.opacity(0.18)),
                    strikethrough: Some(gpui::StrikethroughStyle {
                        thickness: theme.underline_thickness(),
                        color: Some(colors.danger),
                    }),
                    ..HighlightStyle::default()
                };
                let added = HighlightStyle {
                    background_color: Some(colors.success.opacity(0.18)),
                    ..HighlightStyle::default()
                };
                for range in ranges {
                    out.push_str(&text[last..range.start]);
                    styles.push((out.len()..out.len() + range.len(), old));
                    out.push_str(&text[range.clone()]);
                    styles.push((out.len()..out.len() + new.len(), added));
                    out.push_str(new);
                    last = range.end;
                }
                out.push_str(&text[last..]);
                (out, styles)
            }
        };
        let (path, line) = (self.path.clone(), self.hit.line);
        div()
            .id(self.id)
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .py_0p5()
            .rounded(theme.radius(Radius::Sm))
            .cursor_pointer()
            .hover(|row| row.bg(colors.hover))
            .text_size(theme.text_size(TextSize::Sm))
            .when_some(self.on_open, |row, open| {
                row.on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .on_click(move |_, window, cx| {
                        log::info!("search result: {path}:{}", line + 1);
                        open(&path, line, window, cx)
                    })
            })
            .child(
                div()
                    .flex_none()
                    .w(theme.label_width() * 0.4)
                    .text_right()
                    .text_color(colors.fg_subtle)
                    .font_family(theme.mono_family.clone())
                    .child((self.hit.line + 1).to_string()),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .font_family(theme.mono_family.clone())
                    .text_color(colors.fg_muted)
                    .child(StyledText::new(text).with_highlights(styles)),
            )
    }
}

/// Results grouped by file, each with its count and a chevron that folds it; the totals lead.
fn results(
    id: &ElementId,
    files: &[FileHits],
    folded: &[SharedString],
    replacement: Option<&SharedString>,
    on_open: Option<OnOpen>,
    on_fold: Option<OnPath>,
    cx: &App,
) -> Vec<AnyElement> {
    let theme = cx.theme();
    let colors = theme.colors.clone();
    let mut out = Vec::new();
    for (file_ix, file) in files.iter().enumerate() {
        let open = !folded.contains(&file.path);
        let (name, dir) = match file.path.rsplit_once('/') {
            Some((dir, name)) => (name.to_string(), dir.to_string()),
            None => (file.path.to_string(), String::new()),
        };
        let fold = on_fold.clone();
        let path = file.path.clone();
        out.push(
            div()
                .id((id.clone(), format!("file-{file_ix}")))
                .flex()
                .items_center()
                .gap_1p5()
                .px_1()
                .py_0p5()
                .rounded(theme.radius(Radius::Sm))
                .cursor_pointer()
                .hover(|row| row.bg(colors.hover))
                .text_size(theme.text_size(TextSize::Sm))
                .when_some(fold, |row, fold| {
                    row.on_click(move |_, window, cx| fold(&path, window, cx))
                })
                .child(Disclosure::new(
                    (id.clone(), format!("fold-{file_ix}")),
                    open,
                ))
                .child(
                    Icon::new(IconName::FileText)
                        .size(IconSize::Sm)
                        .color(colors.fg_muted),
                )
                .child(div().text_color(colors.fg).child(name))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_color(colors.fg_subtle)
                        .child(dir),
                )
                .child(
                    div()
                        .px_1p5()
                        .rounded_full()
                        .bg(colors.hover)
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(colors.fg_muted)
                        .child(file.hits.len().to_string()),
                )
                .into_any_element(),
        );
        if !open {
            continue;
        }
        for (hit_ix, hit) in file.hits.iter().enumerate() {
            let item = SearchResultItem::new(
                (id.clone(), format!("hit-{file_ix}-{hit_ix}")),
                file.path.clone(),
                hit.clone(),
            );
            let item = match replacement {
                Some(text) => item.replacement(text.clone()),
                None => item,
            };
            let item = match &on_open {
                Some(open) => {
                    let open = open.clone();
                    item.on_open(move |path, line, window, cx| open(path, line, window, cx))
                }
                None => item,
            };
            out.push(div().pl_5().child(item).into_any_element());
        }
    }
    out
}

/// Results across files, folding by file, with the totals above; a replacement previews on each line.
#[derive(IntoElement)]
pub struct SearchPanel {
    id: ElementId,
    files: Vec<FileHits>,
    folded: Vec<SharedString>,
    replacement: Option<SharedString>,
    on_open: Option<OnOpen>,
    on_fold: Option<OnPath>,
}

impl SearchPanel {
    pub fn new(id: impl Into<ElementId>, files: impl IntoIterator<Item = FileHits>) -> Self {
        Self {
            id: id.into(),
            files: files.into_iter().collect(),
            folded: Vec::new(),
            replacement: None,
            on_open: None,
            on_fold: None,
        }
    }

    /// Paths whose results are folded away.
    pub fn folded(mut self, paths: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.folded = paths.into_iter().map(Into::into).collect();
        self
    }

    pub fn replacement(mut self, text: impl Into<SharedString>) -> Self {
        self.replacement = Some(text.into());
        self
    }

    pub fn on_open(
        mut self,
        handler: impl Fn(&SharedString, usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }

    /// Gets a path whose chevron was pressed.
    pub fn on_fold(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_fold = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SearchPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let hits: usize = self
            .files
            .iter()
            .map(|file| file.hits.iter().map(|hit| hit.ranges.len()).sum::<usize>())
            .sum();
        let summary = format!("{hits} results in {} files", self.files.len());
        let rows = results(
            &self.id,
            &self.files,
            &self.folded,
            self.replacement.as_ref(),
            self.on_open,
            self.on_fold,
            cx,
        );
        div()
            .flex()
            .flex_col()
            .gap_0p5()
            .child(
                div()
                    .px_1()
                    .pb_1()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(theme.colors.fg_subtle)
                    .child(summary),
            )
            .children(rows)
    }
}

/// Where a symbol is used, grouped by file, under its name and count.
#[derive(IntoElement)]
pub struct ReferencesPanel {
    id: ElementId,
    symbol: SharedString,
    files: Vec<FileHits>,
    on_open: Option<OnOpen>,
}

impl ReferencesPanel {
    pub fn new(
        id: impl Into<ElementId>,
        symbol: impl Into<SharedString>,
        files: impl IntoIterator<Item = FileHits>,
    ) -> Self {
        Self {
            id: id.into(),
            symbol: symbol.into(),
            files: files.into_iter().collect(),
            on_open: None,
        }
    }

    pub fn on_open(
        mut self,
        handler: impl Fn(&SharedString, usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ReferencesPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let count = references(&self.files);
        let rows = results(&self.id, &self.files, &[], None, self.on_open, None, cx);
        div()
            .flex()
            .flex_col()
            .gap_0p5()
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap_2()
                    .px_1()
                    .pb_1()
                    .child(
                        div()
                            .font_family(theme.mono_family.clone())
                            .text_color(colors.fg)
                            .child(self.symbol),
                    )
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg_subtle)
                            .child(format::plural(count as u64, "reference", "references")),
                    ),
            )
            .children(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_match_on_trailing_spaces_keeps_them() {
        let spaces = 6..8;
        let hit = Hit {
            line: 0,
            text: "let a;  ".into(),
            ranges: vec![spaces],
        };
        let (text, ranges) = trimmed(&hit);
        assert_eq!(&text[ranges[0].clone()], "  ");
    }

    #[test]
    fn references_count_every_match_on_a_line() {
        let both = FileHits {
            path: "a.rs".into(),
            hits: vec![Hit {
                line: 0,
                text: "f(f(1))".into(),
                ranges: vec![0..1, 2..3],
            }],
        };
        assert_eq!(references(&[both]), 2);
    }

    #[test]
    fn long_lines_start_near_their_first_match() {
        let needle = 40..46;
        let hit = Hit {
            line: 0,
            text: format!("{}needle end", "x".repeat(40)).into(),
            ranges: vec![needle],
        };
        let (text, ranges) = trimmed(&hit);
        assert!(text.starts_with('…'));
        assert_eq!(&text[ranges[0].clone()], "needle");
        let needle = 8..14;
        let short = Hit {
            line: 0,
            text: "    let needle = 1;".into(),
            ranges: vec![needle],
        };
        let (text, ranges) = trimmed(&short);
        assert_eq!(text, "let needle = 1;", "leading spaces go");
        assert_eq!(&text[ranges[0].clone()], "needle");
    }
}
