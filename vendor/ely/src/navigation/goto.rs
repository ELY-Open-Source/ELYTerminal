use std::rc::Rc;

use gpui::{
    App, ElementId, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div,
};

use crate::{
    forms::{Enter, Input, TextInput},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize, TextSize},
};

/// Where `text` points in a file of `lines` lines, as "line" or "line:column"; `None` while empty.
pub(crate) fn target(text: &str, lines: usize) -> Result<Option<(usize, Option<usize>)>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let number = |part: &str| part.trim().parse::<usize>().ok().filter(|n| *n >= 1);
    let (line, column) = match text.split_once(':') {
        Some((line, column)) => (number(line), number(column).map(Some)),
        None => (number(text), Some(None)),
    };
    let (Some(line), Some(column)) = (line, column) else {
        return Err("Type a line number, or line:column".into());
    };
    if line > lines {
        return Err(format!("Line {line} is past the last line, {lines}"));
    }
    Ok(Some((line, column)))
}

type OnJump = Rc<dyn Fn(usize, Option<usize>, &mut Window, &mut App)>;

/// A field that jumps to "line" or "line:column". Enter goes; a note says where, or why not.
#[derive(IntoElement)]
pub struct GoToLine {
    id: ElementId,
    state: Entity<TextInput>,
    lines: usize,
    on_jump: Option<OnJump>,
}

impl GoToLine {
    pub fn new(id: impl Into<ElementId>, state: &Entity<TextInput>, lines: usize) -> Self {
        assert!(lines >= 1, "a file to jump in has a line");
        Self {
            id: id.into(),
            state: state.clone(),
            lines,
            on_jump: None,
        }
    }

    pub fn on_jump(
        mut self,
        handler: impl Fn(usize, Option<usize>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_jump = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for GoToLine {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let lines = self.lines;
        let aim = target(self.state.read(cx).text(), lines);
        let theme = cx.theme();
        let colors = &theme.colors;
        let (note, color): (SharedString, _) = match &aim {
            Ok(None) => (format!("Lines 1 to {lines}").into(), colors.fg_subtle),
            Ok(Some((line, None))) => (format!("Go to line {line}").into(), colors.fg_muted),
            Ok(Some((line, Some(column)))) => (
                format!("Go to line {line}, column {column}").into(),
                colors.fg_muted,
            ),
            Err(problem) => (problem.clone().into(), colors.danger),
        };
        let (id, on_jump, state) = (self.id.clone(), self.on_jump, self.state.clone());
        div()
            .id(self.id)
            .flex()
            .flex_col()
            .gap_1()
            .capture_action(move |_: &Enter, window, cx| {
                cx.stop_propagation();
                match target(state.read(cx).text(), lines) {
                    Ok(Some((line, column))) => {
                        log::info!("go to line {id:?}: {line}:{column:?}");
                        if let Some(on_jump) = &on_jump {
                            on_jump(line, column, window, cx);
                        }
                    }
                    Ok(None) => {}
                    Err(problem) => log::info!("go to line {id:?}: {problem}"),
                }
            })
            .child(
                Input::new(&self.state).invalid(aim.is_err()).prefix(
                    Icon::new(IconName::Hash)
                        .size(IconSize::Sm)
                        .color(colors.fg_subtle),
                ),
            )
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(color)
                    .child(note),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::target;

    #[test]
    fn targets_read_line_and_column_and_refuse_the_rest() {
        assert_eq!(target("", 100), Ok(None));
        assert_eq!(target(" 12 ", 100), Ok(Some((12, None))));
        assert_eq!(target("12:4", 100), Ok(Some((12, Some(4)))));
        assert!(target("0", 100).is_err());
        assert!(target("12:", 100).is_err());
        assert!(target("abc", 100).is_err());
        assert_eq!(
            target("300", 240),
            Err("Line 300 is past the last line, 240".to_string())
        );
    }
}
