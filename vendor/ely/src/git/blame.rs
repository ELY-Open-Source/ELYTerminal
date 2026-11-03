use std::{ops::Range, rc::Rc};

use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce, Role,
    SharedString, StatefulInteractiveElement, Styled, StyledText, Window, div, prelude::*,
    relative, uniform_list,
};

use crate::{
    data_display::Avatar,
    editor::code_colors,
    primitives::FocusRing,
    theme::{ActiveTheme, AvatarSize, TextSize},
    typography::{Ellipsis, LEADING},
};

/// Who last changed lines; age 0 newest, 1 oldest.
#[derive(Clone, Debug, PartialEq)]
pub struct Blame {
    pub commit: SharedString,
    pub author: SharedString,
    pub when: SharedString,
    pub subject: SharedString,
    pub age: f32,
}

/// Consecutive lines from one change, with that change's index.
pub(crate) fn runs(owners: &[Option<usize>]) -> Vec<(Option<usize>, Range<usize>)> {
    let mut out: Vec<(Option<usize>, Range<usize>)> = Vec::new();
    for (line, owner) in owners.iter().enumerate() {
        match out.last_mut() {
            Some((last, lines)) if last == owner => lines.end = line + 1,
            _ => out.push((*owner, line..line + 1)),
        }
    }
    out
}

/// Who last changed a line, quietly after its code.
#[derive(IntoElement)]
pub struct GitBlameAnnotation {
    blame: Blame,
}

impl GitBlameAnnotation {
    pub fn new(blame: Blame) -> Self {
        Self { blame }
    }
}

impl RenderOnce for GitBlameAnnotation {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors.clone();
        div()
            .flex_none()
            .pl_6()
            .whitespace_nowrap()
            .text_color(colors.fg_subtle)
            .child(format!(
                "{}, {} · {}",
                self.blame.author, self.blame.when, self.blame.subject
            ))
    }
}

type OnCommit = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// Code beside each run's last author; a press picks it.
#[derive(IntoElement)]
pub struct BlameView {
    id: ElementId,
    code: SharedString,
    owners: Vec<Option<usize>>,
    blames: Vec<Blame>,
    current: Option<usize>,
    on_commit: Option<OnCommit>,
}

impl BlameView {
    /// `owners`: each line's entry in `blames`.
    pub fn new(
        id: impl Into<ElementId>,
        code: impl Into<SharedString>,
        owners: impl IntoIterator<Item = usize>,
        blames: impl IntoIterator<Item = Blame>,
    ) -> Self {
        let code: SharedString = code.into();
        let owners: Vec<usize> = owners.into_iter().collect();
        let blames: Vec<Blame> = blames.into_iter().collect();
        let lines = code.lines().count();
        if owners.len() != lines || owners.iter().any(|owner| *owner >= blames.len()) {
            log::error!(
                "blame view: {} owners for {lines} lines and {} blames; lines without one draw bare",
                owners.len(),
                blames.len()
            );
        }
        let owners: Vec<Option<usize>> = (0..lines)
            .map(|line| {
                owners
                    .get(line)
                    .copied()
                    .filter(|owner| *owner < blames.len())
            })
            .collect();
        Self {
            id: id.into(),
            code,
            owners,
            blames,
            current: None,
            on_commit: None,
        }
    }

    /// The line with the cursor, from zero.
    pub fn current(mut self, line: usize) -> Self {
        self.current = Some(line);
        self
    }

    pub fn on_commit(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_commit = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for BlameView {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let frame = div()
            .size_full()
            .font_family(theme.mono_family.clone())
            .text_size(theme.text_size(TextSize::Xs))
            .text_color(theme.colors.fg);
        let Self {
            id,
            code,
            owners,
            blames,
            current,
            on_commit,
        } = self;
        let lines: Rc<Vec<SharedString>> =
            Rc::new(code.lines().map(|line| line.to_string().into()).collect());
        let digits = lines.len().to_string().len();
        // Each line's owner, and whether it starts a run.
        let heads: Rc<Vec<(Option<usize>, bool)>> = Rc::new(
            runs(&owners)
                .into_iter()
                .flat_map(|(owner, span)| span.clone().map(move |line| (owner, line == span.start)))
                .collect(),
        );
        let blames = Rc::new(blames);
        let list = uniform_list((id.clone(), "lines"), lines.len(), move |range, _, cx| {
            let theme = cx.theme();
            let colors = theme.colors.clone();
            range
                .map(|line| {
                    let (owner, head) = heads[line];
                    let blame = owner.map(|owner| blames[owner].clone());
                    let fade = blame
                        .as_ref()
                        .map(|blame| 1.0 - blame.age.clamp(0.0, 1.0) * 0.8);
                    let gutter = div()
                        .id((id.clone(), format!("gutter-{line}")))
                        .flex_none()
                        .w_80()
                        .h_full()
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_2()
                        .border_l_2()
                        .border_color(
                            fade.map_or(colors.border, |fade| colors.accent.opacity(fade)),
                        )
                        .font_family(theme.font_family.clone())
                        .text_size(theme.text_size(TextSize::Xs));
                    let gutter = match blame.clone().filter(|_| head) {
                        Some(blame) => {
                            // Empty parts, as for uncommitted lines, are left out.
                            let said: SharedString = [&blame.author, &blame.when, &blame.subject]
                                .into_iter()
                                .filter(|part| !part.is_empty())
                                .map(|part| part.as_ref())
                                .collect::<Vec<&str>>()
                                .join(", ")
                                .into();
                            let pick = on_commit.clone();
                            let commit = blame.commit.clone();
                            gutter
                                .when_some(pick, |gutter, pick| {
                                    gutter
                                        .role(Role::Button)
                                        .aria_label(said)
                                        .tab_index(0)
                                        .border_1()
                                        .border_color(gpui::transparent_black())
                                        .focus_ring(cx)
                                        .cursor_pointer()
                                        .hover(|gutter| gutter.bg(colors.hover))
                                        .on_click(move |_, window, cx| pick(&commit, window, cx))
                                })
                                .child(
                                    Avatar::new(
                                        (id.clone(), format!("author-{line}")),
                                        blame.author.clone(),
                                    )
                                    .size(AvatarSize::Xs),
                                )
                                .child(
                                    div()
                                        .flex_none()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(colors.fg)
                                        .child(blame.author.clone()),
                                )
                                .child(
                                    div()
                                        .flex_none()
                                        .text_color(colors.fg_subtle)
                                        .child(blame.when.clone()),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .text_color(colors.fg_muted)
                                        .child(Ellipsis::new(blame.subject.clone())),
                                )
                        }
                        None => gutter,
                    };
                    let text = lines[line].clone();
                    let styles = code_colors(&text, cx);
                    div()
                        .id((id.clone(), format!("line-{line}")))
                        .role(Role::Label)
                        .aria_label(SharedString::from(format!("{} {text}", line + 1)))
                        .w_full()
                        .flex()
                        .items_center()
                        .gap_3()
                        .border_t_1()
                        .border_color(if head && line > 0 {
                            colors.border
                        } else {
                            gpui::transparent_black()
                        })
                        .whitespace_nowrap()
                        .line_height(relative(LEADING))
                        .when(current == Some(line), |row| row.bg(colors.hover))
                        .child(gutter)
                        .child(
                            div()
                                .flex_none()
                                .w(theme.text_size(TextSize::Xs) * (digits as f32 * 0.62))
                                .text_right()
                                .text_color(colors.fg_subtle)
                                .child((line + 1).to_string()),
                        )
                        .child(StyledText::new(text).with_highlights(styles))
                        .when_some(blame.filter(|_| current == Some(line)), |row, blame| {
                            row.child(GitBlameAnnotation::new(blame))
                        })
                })
                .collect()
        })
        .size_full();
        frame.child(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_from_one_change_run_together() {
        let owned = [Some(0), Some(0), Some(1), None, None, Some(0)];
        assert_eq!(
            runs(&owned),
            [
                (Some(0), 0..2),
                (Some(1), 2..3),
                (None, 3..5),
                (Some(0), 5..6)
            ]
        );
        assert!(runs(&[]).is_empty());
    }
}
