use std::rc::Rc;

use gpui::{App, ElementId, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div};

use crate::{
    buttons::{ToggleButton, ToggleItem},
    forms::{Input, TextInput},
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, TextSize},
};

/// Whether a path passes comma-separated globs: some include pattern when there are any, and no exclude one.
pub fn passes(path: &str, include: &str, exclude: &str) -> bool {
    let patterns = |list: &str| -> Vec<Option<regex::Regex>> {
        list.split(',')
            .map(str::trim)
            .filter(|glob| !glob.is_empty())
            .map(|glob| {
                let mut pattern = String::from("(^|/)");
                let mut chars = glob.chars().peekable();
                while let Some(ch) = chars.next() {
                    match ch {
                        '*' if chars.peek() == Some(&'*') => {
                            chars.next();
                            if chars.next_if_eq(&'/').is_some() {
                                pattern.push_str("(?:.*/)?");
                            } else {
                                pattern.push_str(".*");
                            }
                        }
                        '*' => pattern.push_str("[^/]*"),
                        '?' => pattern.push_str("[^/]"),
                        other => pattern.push_str(&regex::escape(&other.to_string())),
                    }
                }
                pattern.push('$');
                regex::Regex::new(&pattern)
                    .inspect_err(|error| {
                        log::error!(
                            "search filter: a glob of {} bytes matches nothing: {error}",
                            pattern.len()
                        )
                    })
                    .ok()
            })
            .collect()
    };
    let (include, exclude) = (patterns(include), patterns(exclude));
    (include.is_empty() || include.iter().flatten().any(|glob| glob.is_match(path)))
        && !exclude.iter().flatten().any(|glob| glob.is_match(path))
}

type OnBool = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// Which files a search reads: globs to include and to exclude, and whether ignored files stay out.
#[derive(IntoElement)]
pub struct SearchFilters {
    id: ElementId,
    include: Entity<TextInput>,
    exclude: Entity<TextInput>,
    ignore: bool,
    on_ignore: Option<OnBool>,
}

impl SearchFilters {
    pub fn new(
        id: impl Into<ElementId>,
        include: &Entity<TextInput>,
        exclude: &Entity<TextInput>,
    ) -> Self {
        Self {
            id: id.into(),
            include: include.clone(),
            exclude: exclude.clone(),
            ignore: true,
            on_ignore: None,
        }
    }

    /// Whether files the project ignores stay out.
    pub fn ignore(mut self, ignore: bool) -> Self {
        self.ignore = ignore;
        self
    }

    pub fn on_ignore(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_ignore = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SearchFilters {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let label = |words: &'static str| {
            div()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(colors.fg_subtle)
                .child(words)
        };
        let ignore = ToggleButton::new(
            (self.id.clone(), "ignore"),
            ToggleItem::new("ignore")
                .icon(IconName::EyeOff)
                .tooltip("Leave out ignored files"),
            self.ignore,
        )
        .size(ControlSize::Sm);
        let ignore = match self.on_ignore {
            Some(on_ignore) => ignore.on_toggle(move |on, window, cx| on_ignore(on, window, cx)),
            None => ignore,
        };
        div()
            .flex()
            .flex_col()
            .gap_1p5()
            .child(label("Files to include"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .flex_1()
                            .child(Input::new(&self.include).size(ControlSize::Sm)),
                    )
                    .child(ignore),
            )
            .child(label("Files to exclude"))
            .child(Input::new(&self.exclude).size(ControlSize::Sm))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs_take_includes_then_drop_excludes() {
        assert!(passes("src/editor/view.rs", "src/**/*.rs", ""));
        assert!(
            passes("src/main.rs", "src/**/*.rs", ""),
            "** takes no folders too"
        );
        assert!(!passes("examples/main.rs", "src/**", ""));
        assert!(!passes(
            "src/editor/tests.rs",
            "*.rs",
            "tests.rs, target/**"
        ));
        assert!(passes("README.md", "", "*.rs"));
        assert!(!passes("src/a.rs", "src/?.md", ""));
    }
}
