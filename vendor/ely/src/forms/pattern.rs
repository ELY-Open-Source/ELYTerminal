use std::ops::Range;

use gpui::{
    App, Entity, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
    prelude::*,
};
use regex::Regex;

use super::{Highlight, Input, TextInput};
use crate::theme::{ActiveTheme, ControlSize, TextSize};

/// Token spans of a regex, by kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    Escape,
    Class,
    Group,
    Quantifier,
    Anchor,
    Alternation,
}

/// Splits a pattern into colored parts; plain text stays uncolored.
pub(crate) fn parts(pattern: &str) -> Vec<(Range<usize>, Part)> {
    let mut out = Vec::new();
    let mut chars = pattern.char_indices().peekable();
    while let Some((ix, ch)) = chars.next() {
        let span = |end: usize| ix..end;
        match ch {
            '\\' => {
                let end = chars
                    .next()
                    .map_or(pattern.len(), |(next, ch)| next + ch.len_utf8());
                out.push((span(end), Part::Escape));
            }
            '[' => {
                let mut end = pattern.len();
                while let Some((next, ch)) = chars.next() {
                    if ch == '\\' {
                        chars.next();
                    } else if ch == ']' {
                        end = next + 1;
                        break;
                    }
                }
                out.push((span(end), Part::Class));
            }
            '(' | ')' => {
                let mut end = ix + 1;
                if ch == '(' && chars.peek().is_some_and(|(_, next)| *next == '?') {
                    chars.next();
                    end += 1;
                    if let Some((next, ch)) =
                        chars.next_if(|(_, ch)| matches!(ch, ':' | '=' | '!' | '<'))
                    {
                        end = next + ch.len_utf8();
                    }
                }
                out.push((span(end), Part::Group));
            }
            '*' | '+' | '?' => out.push((span(ix + 1), Part::Quantifier)),
            '{' => {
                let end = pattern[ix..]
                    .find('}')
                    .map_or(pattern.len(), |close| ix + close + 1);
                while chars.peek().is_some_and(|(next, _)| *next < end) {
                    chars.next();
                }
                out.push((span(end), Part::Quantifier));
            }
            '^' | '$' => out.push((span(ix + 1), Part::Anchor)),
            '|' => out.push((span(ix + 1), Part::Alternation)),
            _ => {}
        }
    }
    out
}

/// Colors for a regex, from the syntax palette.
pub fn regex_highlights(pattern: &str, cx: &App) -> Vec<(Range<usize>, Highlight)> {
    let syntax = &cx.theme().colors.syntax;
    parts(pattern)
        .into_iter()
        .map(|(range, part)| {
            let color = match part {
                Part::Escape => syntax.keyword,
                Part::Class => syntax.string,
                Part::Group => syntax.function,
                Part::Quantifier => syntax.number,
                Part::Anchor => syntax.constant,
                Part::Alternation => syntax.operator,
            };
            (range, Highlight::new(color))
        })
        .collect()
}

/// A regular expression field: colored as you type, checked as you type.
#[derive(IntoElement)]
pub struct RegexInput {
    state: Entity<TextInput>,
    size: ControlSize,
}

impl RegexInput {
    /// Give the state `regex_highlights` as its highlighter.
    pub fn new(state: &Entity<TextInput>) -> Self {
        Self {
            state: state.clone(),
            size: ControlSize::default(),
        }
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }
}

impl RenderOnce for RegexInput {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let pattern = self.state.read(cx).text().to_string();
        let error = (!pattern.is_empty())
            .then(|| Regex::new(&pattern).err())
            .flatten()
            .map(|error| {
                let text = error.to_string();
                let last = text.lines().last().unwrap_or_default().trim().to_string();
                SharedString::from(last)
            });
        let theme = cx.theme();
        let (subtle, danger) = (theme.colors.fg_subtle, theme.colors.danger);
        let mono = theme.mono_family.clone();
        let slash = || div().text_color(subtle).child("/");
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div().font_family(mono).child(
                    Input::new(&self.state)
                        .size(self.size)
                        .invalid(error.is_some())
                        .prefix(slash())
                        .suffix(slash()),
                ),
            )
            .when_some(error, |field, error| {
                field.child(
                    div()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(danger)
                        .child(error),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{Part, parts};

    #[test]
    fn parts_cover_escapes_classes_groups_quantifiers() {
        let found = parts(r"^(?:\d+)[a-z\]]{2,3}|x$");
        let kinds: Vec<Part> = found.iter().map(|(_, part)| *part).collect();
        assert_eq!(
            kinds,
            vec![
                Part::Anchor,
                Part::Group,
                Part::Escape,
                Part::Quantifier,
                Part::Group,
                Part::Class,
                Part::Quantifier,
                Part::Alternation,
                Part::Anchor,
            ]
        );
        assert_eq!(found[1].0, 1..4);
        assert_eq!(found[5].0, 8..15);
    }
}
