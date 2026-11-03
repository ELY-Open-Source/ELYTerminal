use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled, Window,
    div, prelude::*,
};

use crate::{
    data_display::Badge,
    lists::GitStatus,
    primitives::Tooltip,
    theme::{ActiveTheme, Radius, TextSize},
    typography::tabular,
};

/// A file's git status as its letter in its tone; the word shows on hover.
#[derive(IntoElement)]
pub struct GitStatusBadge {
    id: ElementId,
    status: GitStatus,
}

impl GitStatusBadge {
    pub fn new(id: impl Into<ElementId>, status: GitStatus) -> Self {
        Self {
            id: id.into(),
            status,
        }
    }
}

impl RenderOnce for GitStatusBadge {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .id(self.id)
            .tooltip(Tooltip::text(self.status.word()))
            .child(Badge::new(self.status.letter()).tone(self.status.tone()))
    }
}

/// Dots a diff stat shows.
const SQUARES: usize = 5;

/// How many of the dots read as added, the rest as removed, when anything changed.
pub(crate) fn split(added: usize, removed: usize) -> Option<usize> {
    let total = added + removed;
    (total > 0).then(|| (added * SQUARES + total / 2) / total)
}

/// Lines added and removed: the two counts, and five dots shared between them.
#[derive(IntoElement)]
pub struct DiffStat {
    added: usize,
    removed: usize,
}

impl DiffStat {
    pub fn new(added: usize, removed: usize) -> Self {
        Self { added, removed }
    }
}

impl RenderOnce for DiffStat {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let green = split(self.added, self.removed);
        let square = |ix: usize| {
            let color = match green {
                Some(green) if ix < green => colors.success,
                Some(_) => colors.danger,
                None => colors.border,
            };
            div().size_2().rounded(theme.radius(Radius::Sm)).bg(color)
        };
        div()
            .flex()
            .items_center()
            .gap_1p5()
            .text_size(theme.text_size(TextSize::Sm))
            .child(tabular(
                div()
                    .text_color(colors.success)
                    .child(format!("+{}", self.added)),
            ))
            .child(tabular(
                div()
                    .text_color(colors.danger)
                    .child(format!("−{}", self.removed)),
            ))
            .child(div().flex().gap_0p5().children((0..SQUARES).map(square)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn squares_split_by_share() {
        assert_eq!(split(0, 0), None);
        assert_eq!(split(10, 0), Some(5));
        assert_eq!(split(120, 45), Some(4));
        assert_eq!(split(1, 9), Some(1), "a little still shows");
    }
}
