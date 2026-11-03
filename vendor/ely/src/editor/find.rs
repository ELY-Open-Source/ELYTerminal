use std::{ops::Range, rc::Rc};

use gpui::{
    App, ElementId, Entity, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window,
    div, prelude::*,
};

use crate::i18n;
use crate::{
    buttons::{Button, ButtonVariant, IconButton, ToggleButton, ToggleItem},
    forms::{Input, TextInput},
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, Elevation, Radius, TextSize},
};

/// How a search matches: case exactly, whole words only, the query as a regular expression.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FindOptions {
    pub case: bool,
    pub word: bool,
    pub regex: bool,
}

/// Where `query` appears in `text` under `options`; a pattern that does not compile is an error to show.
pub fn find_all(
    text: &str,
    query: &str,
    options: FindOptions,
) -> Result<Vec<Range<usize>>, String> {
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let pattern = if options.regex {
        query.to_string()
    } else {
        regex::escape(query)
    };
    let pattern = if options.word {
        format!(r"\b(?:{pattern})\b")
    } else {
        pattern
    };
    let found = regex::RegexBuilder::new(&pattern)
        .case_insensitive(!options.case)
        .multi_line(true)
        .build()
        .map_err(|error| error.to_string())?;
    Ok(found
        .find_iter(text)
        .filter(|hit| !hit.is_empty())
        .map(|hit| hit.range())
        .collect())
}

type OnOptions = Rc<dyn Fn(FindOptions, &mut Window, &mut App)>;
type OnBool = Rc<dyn Fn(bool, &mut Window, &mut App)>;
type OnClose = Rc<dyn Fn(&mut Window, &mut App)>;

/// A find box: the query, its options when the owner takes them, where the current match stands among all, steps between them, and a replace row that folds away when the owner offers one.
#[derive(IntoElement)]
pub struct FindWidget {
    id: ElementId,
    find: Entity<TextInput>,
    replace: Option<Entity<TextInput>>,
    options: FindOptions,
    matches: Result<(Option<usize>, usize), SharedString>,
    on_options: Option<OnOptions>,
    on_step: Option<OnBool>,
    on_replace: Option<OnBool>,
    on_toggle_replace: Option<OnBool>,
    on_close: Option<OnClose>,
}

impl FindWidget {
    /// `find` holds the query; `current` is the chosen match among `total`.
    pub fn new(
        id: impl Into<ElementId>,
        find: &Entity<TextInput>,
        current: Option<usize>,
        total: usize,
    ) -> Self {
        if current.is_some_and(|current| current >= total) {
            log::error!("find: match {current:?} of {total}; none current");
        }
        let current = current.filter(|current| *current < total);
        Self {
            id: id.into(),
            find: find.clone(),
            replace: None,
            options: FindOptions::default(),
            matches: Ok((current, total)),
            on_options: None,
            on_step: None,
            on_replace: None,
            on_toggle_replace: None,
            on_close: None,
        }
    }

    /// The query failed to compile; its message shows in place of a count.
    pub fn error(mut self, message: impl Into<SharedString>) -> Self {
        self.matches = Err(message.into());
        self
    }

    pub fn options(mut self, options: FindOptions) -> Self {
        self.options = options;
        self
    }

    /// Shows the replace row, its text in `field`.
    pub fn replace(mut self, field: &Entity<TextInput>) -> Self {
        self.replace = Some(field.clone());
        self
    }

    pub fn on_options(
        mut self,
        handler: impl Fn(FindOptions, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_options = Some(Rc::new(handler));
        self
    }

    /// Gets `true` for the next match, `false` for the previous.
    pub fn on_step(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_step = Some(Rc::new(handler));
        self
    }

    /// Gets `true` to replace every match, `false` for the current one.
    pub fn on_replace(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_replace = Some(Rc::new(handler));
        self
    }

    /// Gets whether the replace row should show.
    pub fn on_toggle_replace(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle_replace = Some(Rc::new(handler));
        self
    }

    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }
}

/// The count shown, and whether it reads as a miss.
pub(crate) fn count(
    matches: &Result<(Option<usize>, usize), SharedString>,
    cx: &App,
) -> (SharedString, bool) {
    match matches {
        Ok((_, 0)) => (i18n::text(cx, "find.none", &[]), true),
        Ok((Some(current), total)) => {
            let (at, total) = ((current + 1).to_string(), total.to_string());
            let args = [("at", at.as_str()), ("total", total.as_str())];
            (i18n::text(cx, "find.at", &args), false)
        }
        Ok((None, total)) => {
            let total = total.to_string();
            (i18n::text(cx, "find.found", &[("total", &total)]), false)
        }
        Err(message) => (message.clone(), true),
    }
}

impl RenderOnce for FindWidget {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let options = self.options;
        let toggle = |key: &'static str,
                      icon: IconName,
                      words: SharedString,
                      on: bool,
                      set: fn(&mut FindOptions, bool)| {
            let on_options = self.on_options.clone();
            ToggleButton::new(
                (self.id.clone(), key),
                ToggleItem::new(key).icon(icon).tooltip(words),
                on,
            )
            .size(ControlSize::Sm)
            .on_toggle(move |on, window, cx| {
                let mut next = options;
                set(&mut next, on);
                log::info!("find: {next:?}");
                if let Some(on_options) = &on_options {
                    on_options(next, window, cx);
                }
            })
        };
        let (count, missing) = count(&self.matches, cx);
        let found = matches!(self.matches, Ok((_, total)) if total > 0);
        let icon_button = |key: &'static str,
                           icon: IconName,
                           words: SharedString,
                           enabled: bool,
                           action: Option<OnBool>,
                           value: bool| {
            IconButton::new((self.id.clone(), key), icon)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip(words)
                .disabled(!enabled)
                .when_some(action, move |button, action| {
                    button.on_click(move |_, window, cx| action(value, window, cx))
                })
        };
        let showing = self.replace.is_some();
        let row = || div().flex().items_center().gap_1();
        div()
            .w(theme.label_width() * 2.75)
            .flex()
            .flex_col()
            .gap_1()
            .p_1()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.border)
            .bg(colors.overlay)
            .shadow(theme.elevation(Elevation::Floating))
            .text_size(theme.text_size(TextSize::Xs))
            // Escape from the query closes the box.
            .when_some(self.on_close.clone(), |el, close| {
                el.on_key_down(move |event, window, cx| {
                    if event.keystroke.key == "escape" {
                        cx.stop_propagation();
                        close(window, cx);
                    }
                })
            })
            .child(
                row()
                    .children(self.on_toggle_replace.clone().map(|fold| {
                        icon_button(
                            "fold",
                            if showing {
                                IconName::ChevronDown
                            } else {
                                IconName::ChevronRight
                            },
                            i18n::text(cx, "find.fold", &[]),
                            true,
                            Some(fold),
                            !showing,
                        )
                    }))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(Input::new(&self.find).size(ControlSize::Sm)),
                    )
                    .when(self.on_options.is_some(), |row| {
                        row.child(toggle(
                            "case",
                            IconName::CaseSensitive,
                            i18n::text(cx, "find.case", &[]),
                            options.case,
                            |next, on| next.case = on,
                        ))
                        .child(toggle(
                            "word",
                            IconName::WholeWord,
                            i18n::text(cx, "find.word", &[]),
                            options.word,
                            |next, on| next.word = on,
                        ))
                        .child(toggle(
                            "regex",
                            IconName::Regex,
                            i18n::text(cx, "find.regex", &[]),
                            options.regex,
                            |next, on| next.regex = on,
                        ))
                    })
                    .child(
                        div()
                            .w(theme.label_width() * 0.5)
                            .text_center()
                            .text_color(if missing {
                                colors.danger
                            } else {
                                colors.fg_muted
                            })
                            .child(count),
                    )
                    .child(icon_button(
                        "previous",
                        IconName::ArrowUp,
                        i18n::text(cx, "find.previous", &[]),
                        found,
                        self.on_step.clone(),
                        false,
                    ))
                    .child(icon_button(
                        "next",
                        IconName::ArrowDown,
                        i18n::text(cx, "find.next", &[]),
                        found,
                        self.on_step.clone(),
                        true,
                    ))
                    .children(self.on_close.clone().map(|close| {
                        IconButton::new((self.id.clone(), "close"), IconName::X)
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Sm)
                            .tooltip(i18n::text(cx, "dialog.close", &[]))
                            .on_click(move |_, window, cx| close(window, cx))
                    })),
            )
            .children(self.replace.as_ref().map(|field| {
                let (one, all) = (self.on_replace.clone(), self.on_replace.clone());
                row()
                    .when(self.on_toggle_replace.is_some(), |row| {
                        row.pl(theme.control_height(ControlSize::Sm))
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(Input::new(field).size(ControlSize::Sm)),
                    )
                    .child(
                        Button::new(
                            (self.id.clone(), "replace"),
                            i18n::text(cx, "find.replace", &[]),
                        )
                        .variant(ButtonVariant::Ghost)
                        .size(ControlSize::Sm)
                        .disabled(!found)
                        .when_some(one, |button, one| {
                            button.on_click(move |_, window, cx| one(false, window, cx))
                        }),
                    )
                    .child(
                        Button::new(
                            (self.id.clone(), "all"),
                            i18n::text(cx, "find.replace_all", &[]),
                        )
                        .variant(ButtonVariant::Ghost)
                        .size(ControlSize::Sm)
                        .disabled(!found)
                        .when_some(all, |button, all| {
                            button.on_click(move |_, window, cx| all(true, window, cx))
                        }),
                    )
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_by_case_word_and_pattern() {
        let text = "Let total = total_cost + TOTAL;";
        let plain = FindOptions::default();
        assert_eq!(
            find_all(text, "total", plain).map(|found| found.len()),
            Ok(3)
        );
        let case = FindOptions {
            case: true,
            ..plain
        };
        assert_eq!(
            find_all(text, "total", case).map(|found| found.len()),
            Ok(2)
        );
        let word = FindOptions {
            word: true,
            ..plain
        };
        assert_eq!(find_all(text, "total", word), Ok(vec![4..9, 25..30]));
        let pattern = FindOptions {
            regex: true,
            ..plain
        };
        let word = 12..22;
        assert_eq!(find_all(text, r"total_\w+", pattern), Ok(vec![word]));
        assert!(
            find_all(text, "(", pattern).is_err(),
            "a broken pattern says so"
        );
        assert_eq!(
            find_all(text, "(", plain),
            Ok(Vec::new()),
            "and plain text is escaped"
        );
    }
}
