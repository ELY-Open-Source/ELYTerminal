use std::ops::Range;

use gpui::{
    App, AppContext as _, ElementId, Entity, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, Styled, Window, div,
};
use regex::RegexBuilder;

use crate::{
    buttons::{ToggleGroup, ToggleItem},
    feedback::InlineMessage,
    forms::{Highlight, Input, RegexInput, TextInput, regex_highlights},
    primitives::Severity,
    theme::{ActiveTheme, TextSize},
};

/// A flag a pattern reads with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegexFlag {
    /// `i`: letters match either case.
    IgnoreCase,
    /// `m`: `^` and `$` match at each line.
    MultiLine,
    /// `s`: `.` matches a line break too.
    DotAll,
    /// `x`: spaces and `#` comments in the pattern are ignored.
    Verbose,
}

impl RegexFlag {
    pub const ALL: [RegexFlag; 4] = [
        RegexFlag::IgnoreCase,
        RegexFlag::MultiLine,
        RegexFlag::DotAll,
        RegexFlag::Verbose,
    ];

    pub fn letter(self) -> &'static str {
        match self {
            RegexFlag::IgnoreCase => "i",
            RegexFlag::MultiLine => "m",
            RegexFlag::DotAll => "s",
            RegexFlag::Verbose => "x",
        }
    }

    fn words(self) -> &'static str {
        match self {
            RegexFlag::IgnoreCase => "Ignore case",
            RegexFlag::MultiLine => "^ and $ at each line",
            RegexFlag::DotAll => ". matches a line break",
            RegexFlag::Verbose => "Spaces and comments ignored",
        }
    }
}

/// A match: where it lies in the text, and each group's name or number with what it caught.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub range: Range<usize>,
    pub groups: Vec<(String, Option<String>)>,
}

/// Every match of `pattern` in `text` with `flags`, or why the pattern does not compile.
pub fn find(pattern: &str, flags: &[RegexFlag], text: &str) -> Result<Vec<Found>, String> {
    let regex = RegexBuilder::new(pattern)
        .case_insensitive(flags.contains(&RegexFlag::IgnoreCase))
        .multi_line(flags.contains(&RegexFlag::MultiLine))
        .dot_matches_new_line(flags.contains(&RegexFlag::DotAll))
        .ignore_whitespace(flags.contains(&RegexFlag::Verbose))
        .build()
        .map_err(|error| error.to_string())?;
    let names: Vec<String> = regex
        .capture_names()
        .enumerate()
        .skip(1)
        .map(|(ix, name)| name.map_or(ix.to_string(), str::to_string))
        .collect();
    Ok(regex
        .captures_iter(text)
        .map(|captures| {
            let whole = captures.get(0).expect("a match holds itself");
            Found {
                range: whole.range(),
                groups: names
                    .iter()
                    .enumerate()
                    .map(|(ix, name)| {
                        (
                            name.clone(),
                            captures.get(ix + 1).map(|group| group.as_str().to_string()),
                        )
                    })
                    .collect(),
            }
        })
        .collect())
}

/// The tester's own fields and flags.
struct Bench {
    pattern: Entity<TextInput>,
    sample: Entity<TextInput>,
    flags: Entity<Vec<RegexFlag>>,
}

/// A pattern to try against a sample: the pattern colored as it is read, its flags as toggles, the sample with each match washed, and each match with its groups; a pattern that does not compile says why.
#[derive(IntoElement)]
pub struct RegexTester {
    id: ElementId,
    pattern: SharedString,
    sample: SharedString,
}

impl RegexTester {
    /// The pattern and the sample it starts with.
    pub fn new(
        id: impl Into<ElementId>,
        pattern: impl Into<SharedString>,
        sample: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            pattern: pattern.into(),
            sample: sample.into(),
        }
    }
}

impl RenderOnce for RegexTester {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (pattern, sample) = (self.pattern.to_string(), self.sample.to_string());
        let bench = window.use_keyed_state((self.id.clone(), "bench"), cx, move |window, cx| {
            let flags = cx.new(|_| Vec::new());
            let pattern = cx.new(|cx| {
                let mut input = TextInput::new(window, cx).highlighter(regex_highlights);
                input.set_text(pattern, cx);
                input
            });
            let (read, read_flags) = (pattern.clone(), flags.clone());
            let sample = cx.new(|cx| {
                let mut input = TextInput::new(window, cx).multi_line(4, 10).highlighter(
                    move |text: &str, cx: &App| {
                        let wash = cx.theme().colors.accent.alpha(0.18);
                        let (pattern, flags) = (
                            read.read(cx).text().to_string(),
                            read_flags.read(cx).clone(),
                        );
                        find(&pattern, &flags, text)
                            .unwrap_or_default()
                            .into_iter()
                            .map(|found| {
                                let mut highlight = Highlight::new(cx.theme().colors.fg);
                                highlight.background = Some(wash);
                                (found.range, highlight)
                            })
                            .collect()
                    },
                );
                input.set_text(sample, cx);
                input
            });
            Bench {
                pattern,
                sample,
                flags,
            }
        });
        let (pattern, sample, flags) = {
            let bench = bench.read(cx);
            (
                bench.pattern.clone(),
                bench.sample.clone(),
                bench.flags.clone(),
            )
        };
        let on: Vec<RegexFlag> = flags.read(cx).clone();
        let found = find(pattern.read(cx).text(), &on, sample.read(cx).text());
        let theme = cx.theme();
        let text = sample.read(cx).text().to_string();
        let summary = match &found {
            Err(why) => InlineMessage::new(
                Severity::Danger,
                why.lines().last().unwrap_or(why).to_string(),
            )
            .into_any_element(),
            Ok(matches) => {
                let rows = matches.iter().enumerate().map(|(ix, found)| {
                    let groups: Vec<String> = found
                        .groups
                        .iter()
                        .map(|(name, caught)| {
                            format!("{name}: {}", caught.as_deref().unwrap_or("none"))
                        })
                        .collect();
                    div()
                        .py_1()
                        .border_b_1()
                        .border_color(theme.colors.border)
                        .child(
                            div()
                                .font_family(theme.mono_family.clone())
                                .text_size(theme.text_size(TextSize::Sm))
                                .child(format!(
                                    "{} · bytes {}..{} · {:?}",
                                    ix + 1,
                                    found.range.start,
                                    found.range.end,
                                    &text[found.range.clone()]
                                )),
                        )
                        .children((!groups.is_empty()).then(|| {
                            div()
                                .text_size(theme.text_size(TextSize::Xs))
                                .text_color(theme.colors.fg_muted)
                                .child(groups.join(" · "))
                        }))
                });
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .debug_selector({
                                let count = matches.len();
                                move || format!("regex-matches-{count}")
                            })
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(theme.colors.fg_muted)
                            .child(match matches.len() {
                                1 => "1 match".to_string(),
                                count => format!("{count} matches"),
                            }),
                    )
                    .children(rows)
                    .into_any_element()
            }
        };
        let flip = flags.clone();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w(theme.label_width())
                            .child(RegexInput::new(&pattern)),
                    )
                    .child(
                        RegexFlag::ALL
                            .iter()
                            .fold(
                                ToggleGroup::new((self.id, "flags")).multiple(),
                                |group, flag| {
                                    group.item(
                                        ToggleItem::new(flag.letter())
                                            .label(flag.letter())
                                            .tooltip(flag.words()),
                                    )
                                },
                            )
                            .selected(on.iter().map(|flag| flag.letter()))
                            .on_change(move |letters, _, cx| {
                                flip.update(cx, |flags, cx| {
                                    *flags = RegexFlag::ALL
                                        .into_iter()
                                        .filter(|flag| {
                                            letters.iter().any(|letter| letter == flag.letter())
                                        })
                                        .collect();
                                    cx.notify();
                                })
                            }),
                    ),
            )
            .child(Input::new(&sample))
            .child(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::{RegexFlag, find};

    #[test]
    fn matches_carry_their_groups_and_flags_change_what_matches() {
        let found = find(
            r"(?P<user>\w+)@(\w+)\.com",
            &[],
            "ada@example.com, bob@test.com",
        )
        .expect("a pattern");
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].range, 0..15);
        assert_eq!(
            found[0].groups,
            [
                ("user".to_string(), Some("ada".to_string())),
                ("2".to_string(), Some("example".to_string()))
            ]
        );
        assert_eq!(find("hello", &[], "Hello").expect("a pattern").len(), 0);
        assert_eq!(
            find("hello", &[RegexFlag::IgnoreCase], "Hello")
                .expect("a pattern")
                .len(),
            1
        );
        assert!(find("(", &[], "x").is_err());
    }
}
