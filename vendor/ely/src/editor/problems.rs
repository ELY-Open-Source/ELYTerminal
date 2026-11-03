use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use crate::{
    buttons::{ToggleButton, ToggleItem},
    primitives::{Icon, IconName, Severity},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
    typography::{Ellipsis, format},
};

/// A problem the tools found: where, how bad, what, and which tool said so.
#[derive(Clone, Debug, PartialEq)]
pub struct Problem {
    pub path: SharedString,
    pub line: usize,
    pub column: usize,
    pub severity: Severity,
    pub message: SharedString,
    pub source: Option<SharedString>,
}

/// How bad a severity is, worst highest.
pub(crate) fn weight(severity: Severity) -> u8 {
    match severity {
        Severity::Danger => 3,
        Severity::Warning => 2,
        Severity::Info => 1,
        Severity::Success => 0,
    }
}

/// Problems by file, worst first within each, and each file in the order its worst problem ranks.
pub(crate) fn grouped(
    problems: &[Problem],
    shown: &[Severity],
) -> Vec<(SharedString, Vec<Problem>)> {
    let mut files: Vec<(SharedString, Vec<Problem>)> = Vec::new();
    for problem in problems
        .iter()
        .filter(|problem| shown.contains(&problem.severity))
    {
        match files.iter_mut().find(|(path, _)| *path == problem.path) {
            Some((_, list)) => list.push(problem.clone()),
            None => files.push((problem.path.clone(), vec![problem.clone()])),
        }
    }
    for (_, list) in &mut files {
        list.sort_by(|a, b| {
            weight(b.severity)
                .cmp(&weight(a.severity))
                .then(a.line.cmp(&b.line))
        });
    }
    files.sort_by_key(|(_, list)| {
        std::cmp::Reverse(list.first().map_or(0, |problem| weight(problem.severity)))
    });
    files
}

type OnOpen = Rc<dyn Fn(&Problem, &mut Window, &mut App)>;
type OnShown = Rc<dyn Fn(Vec<Severity>, &mut Window, &mut App)>;

/// Every problem in the workspace, grouped by file: toggles by severity with their counts, and a press opens one.
#[derive(IntoElement)]
pub struct ProblemsPanel {
    id: ElementId,
    problems: Vec<Problem>,
    shown: Vec<Severity>,
    on_open: Option<OnOpen>,
    on_shown: Option<OnShown>,
}

impl ProblemsPanel {
    pub fn new(id: impl Into<ElementId>, problems: impl IntoIterator<Item = Problem>) -> Self {
        Self {
            id: id.into(),
            problems: problems.into_iter().collect(),
            shown: vec![Severity::Danger, Severity::Warning, Severity::Info],
            on_open: None,
            on_shown: None,
        }
    }

    /// The severities listed; the rest are counted but hidden.
    pub fn shown(mut self, severities: impl IntoIterator<Item = Severity>) -> Self {
        self.shown = severities.into_iter().collect();
        self
    }

    pub fn on_open(mut self, handler: impl Fn(&Problem, &mut Window, &mut App) + 'static) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }

    pub fn on_shown(
        mut self,
        handler: impl Fn(Vec<Severity>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_shown = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ProblemsPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let count = |severity: Severity| {
            self.problems
                .iter()
                .filter(|problem| problem.severity == severity)
                .count()
        };
        let filters = [
            (Severity::Danger, "errors", ("Error", "Errors")),
            (Severity::Warning, "warnings", ("Warning", "Warnings")),
            (Severity::Info, "notes", ("Note", "Notes")),
        ]
        .map(|(severity, key, (one, other))| {
            let (shown, on_shown) = (self.shown.clone(), self.on_shown.clone());
            let label = format::plural(count(severity) as u64, one, other);
            ToggleButton::new(
                (self.id.clone(), key),
                ToggleItem::new(key).icon(severity.icon()).label(label),
                self.shown.contains(&severity),
            )
            .size(ControlSize::Sm)
            .on_toggle(move |on, window, cx| {
                let mut next: Vec<Severity> = shown
                    .iter()
                    .copied()
                    .filter(|kept| *kept != severity)
                    .collect();
                if on {
                    next.push(severity);
                }
                log::info!("problems: showing {next:?}");
                if let Some(on_shown) = &on_shown {
                    on_shown(next, window, cx);
                }
            })
        });
        let files = grouped(&self.problems, &self.shown);
        let mut rows = Vec::new();
        for (file_ix, (path, list)) in files.iter().enumerate() {
            let (name, dir) = path
                .rsplit_once('/')
                .map_or((path.to_string(), String::new()), |(dir, name)| {
                    (name.to_string(), dir.to_string())
                });
            rows.push(
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .px_1()
                    .pt_1()
                    .text_size(theme.text_size(TextSize::Sm))
                    .child(
                        Icon::new(IconName::FileText)
                            .size(IconSize::Sm)
                            .color(colors.fg_muted),
                    )
                    .child(div().text_color(colors.fg).child(name))
                    .child(div().text_color(colors.fg_subtle).child(dir))
                    .child(
                        div()
                            .px_1p5()
                            .rounded_full()
                            .bg(colors.hover)
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg_muted)
                            .child(list.len().to_string()),
                    )
                    .into_any_element(),
            );
            for (ix, problem) in list.iter().enumerate() {
                let (open, picked) = (self.on_open.clone(), problem.clone());
                rows.push(
                    div()
                        .id((self.id.clone(), format!("problem-{file_ix}-{ix}")))
                        .flex()
                        .items_center()
                        .gap_2()
                        .pl_6()
                        .pr_2()
                        .py_0p5()
                        .rounded(theme.radius(Radius::Sm))
                        .cursor_pointer()
                        .hover(|row| row.bg(colors.hover))
                        .text_size(theme.text_size(TextSize::Sm))
                        .when_some(open, |row, open| {
                            row.on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.prevent_default()
                            })
                            .on_click(move |_, window, cx| {
                                log::info!("problems: open {}:{}", picked.path, picked.line + 1);
                                open(&picked, window, cx)
                            })
                        })
                        .child(
                            Icon::new(problem.severity.icon())
                                .size(IconSize::Sm)
                                .color(problem.severity.color(&colors)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_color(colors.fg)
                                .child(Ellipsis::new(problem.message.clone())),
                        )
                        .children(
                            problem
                                .source
                                .clone()
                                .map(|source| div().text_color(colors.fg_subtle).child(source)),
                        )
                        .child(
                            div()
                                .text_color(colors.fg_subtle)
                                .font_family(theme.mono_family.clone())
                                .child(format!("{}:{}", problem.line + 1, problem.column + 1)),
                        )
                        .into_any_element(),
                );
            }
        }
        let empty = rows.is_empty();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().flex().gap_1().children(filters))
            .when(empty, |panel| {
                panel.child(
                    div()
                        .py_4()
                        .text_center()
                        .text_size(theme.text_size(TextSize::Sm))
                        .text_color(colors.fg_subtle)
                        .child("No problems shown."),
                )
            })
            .children(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem(path: &str, line: usize, severity: Severity) -> Problem {
        Problem {
            path: path.to_string().into(),
            line,
            column: 0,
            severity,
            message: "m".into(),
            source: None,
        }
    }

    #[test]
    fn files_with_worse_problems_lead_and_filters_hide() {
        let problems = [
            problem("a.rs", 9, Severity::Warning),
            problem("b.rs", 3, Severity::Danger),
            problem("a.rs", 2, Severity::Info),
        ];
        let all = grouped(
            &problems,
            &[Severity::Danger, Severity::Warning, Severity::Info],
        );
        assert_eq!(all[0].0.as_ref(), "b.rs");
        assert_eq!(all[1].1[0].line, 9, "the warning before the note");
        let only = grouped(&problems, &[Severity::Info]);
        assert_eq!(only.len(), 1);
        assert_eq!(only[0].1.len(), 1);
    }
}
