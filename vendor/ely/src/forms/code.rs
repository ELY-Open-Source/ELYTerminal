use std::ops::Range;

use gpui::{
    App, Entity, Hsla, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
    prelude::*,
};

use super::{Highlight, Input, TextInput, syntax::Syntax};
use crate::theme::{ActiveTheme, ControlSize, TextSize};

/// What a span of source is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Comment,
    Text,
    Number,
    Word,
    Mark,
}

const KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "case", "catch", "class", "const", "continue", "def", "do",
    "else", "enum", "export", "fn", "for", "from", "func", "function", "if", "impl", "import",
    "in", "let", "match", "mut", "new", "pub", "return", "static", "struct", "switch", "throw",
    "try", "type", "use", "var", "while", "yield", "SELECT", "FROM", "WHERE", "JOIN", "LEFT",
    "RIGHT", "INNER", "OUTER", "ON", "AND", "OR", "NOT", "AS", "GROUP", "BY", "ORDER", "HAVING",
    "LIMIT", "OFFSET", "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "CREATE", "TABLE",
    "INDEX", "DROP", "ALTER", "DESC", "ASC", "DISTINCT", "UNION", "WITH", "CASE", "WHEN", "THEN",
    "ELSE", "END", "IS", "IN", "LIKE", "BETWEEN", "EXISTS", "INTERVAL",
];
const CONSTANTS: &[&str] = &[
    "false",
    "nil",
    "None",
    "null",
    "self",
    "this",
    "true",
    "undefined",
    "NULL",
    "TRUE",
    "FALSE",
];

/// Every language at once, for code of unknown kind.
const GENERIC: Syntax = Syntax {
    comment: "//",
    word_comment: false,
    keywords: KEYWORDS,
    constants: CONSTANTS,
};

/// Splits source into spans; spaces fall between them.
pub(crate) fn lex(source: &str) -> Vec<(Range<usize>, Kind)> {
    lex_with(source, &GENERIC)
}

/// Splits source as `syntax` reads it.
pub(crate) fn lex_with(source: &str, syntax: &Syntax) -> Vec<(Range<usize>, Kind)> {
    let mut out = Vec::new();
    let mut chars = source.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        let kind = match ch {
            '"' | '\'' | '`' => {
                let mut escaped = false;
                while let Some((_, next)) = chars.next_if(|(_, next)| *next != '\n') {
                    if escaped {
                        escaped = false;
                    } else if next == '\\' {
                        escaped = true;
                    } else if next == ch {
                        break;
                    }
                }
                Kind::Text
            }
            _ if source[start..].starts_with(syntax.comment)
                && (!syntax.word_comment
                    || source[..start]
                        .chars()
                        .next_back()
                        .is_none_or(char::is_whitespace)) =>
            {
                while chars.next_if(|(_, next)| *next != '\n').is_some() {}
                Kind::Comment
            }
            '0'..='9' => {
                while chars
                    .next_if(|(_, next)| next.is_ascii_alphanumeric() || matches!(next, '.' | '_'))
                    .is_some()
                {}
                Kind::Number
            }
            ch if ch.is_alphabetic() || ch == '_' || ch == '$' => {
                while chars
                    .next_if(|(_, next)| next.is_alphanumeric() || matches!(next, '_' | '$'))
                    .is_some()
                {}
                Kind::Word
            }
            ch if ch.is_whitespace() => continue,
            _ => Kind::Mark,
        };
        let end = chars.peek().map_or(source.len(), |(next, _)| *next);
        out.push((start..end, kind));
    }
    out
}

fn painted(spans: impl Iterator<Item = (Range<usize>, Hsla)>) -> Vec<(Range<usize>, Highlight)> {
    spans
        .map(|(range, color)| (range, Highlight::new(color)))
        .collect()
}

/// Colors for one line of code in most languages, from the syntax palette.
pub fn code_highlights(source: &str, cx: &App) -> Vec<(Range<usize>, Highlight)> {
    code_highlights_with(source, &GENERIC, cx)
}

/// Colors for code as `lang` reads it.
pub(crate) fn code_highlights_with(
    source: &str,
    lang: &Syntax,
    cx: &App,
) -> Vec<(Range<usize>, Highlight)> {
    let syntax = &cx.theme().colors.syntax;
    let spans = lex_with(source, lang);
    let next_is = |ix: usize, mark: &str| {
        spans
            .get(ix + 1)
            .is_some_and(|(range, _)| &source[range.clone()] == mark)
    };
    painted(spans.iter().enumerate().map(|(ix, (range, kind))| {
        let text = &source[range.clone()];
        let color = match kind {
            Kind::Comment => syntax.comment,
            Kind::Text => syntax.string,
            Kind::Number => syntax.number,
            Kind::Word if lang.keywords.contains(&text) => syntax.keyword,
            Kind::Word if lang.constants.contains(&text) => syntax.constant,
            Kind::Word if next_is(ix, "(") => syntax.function,
            Kind::Word if text.starts_with(char::is_uppercase) => syntax.type_name,
            Kind::Word => syntax.variable,
            Kind::Mark if "()[]{},.;".contains(text) => syntax.punctuation,
            Kind::Mark => syntax.operator,
        };
        (range.clone(), color)
    }))
}

/// Colors for JSON: keys, strings, numbers and literals.
pub fn json_highlights(source: &str, cx: &App) -> Vec<(Range<usize>, Highlight)> {
    let syntax = &cx.theme().colors.syntax;
    let spans = lex(source);
    let keyed = |ix: usize| {
        spans
            .get(ix + 1)
            .is_some_and(|(range, _)| &source[range.clone()] == ":")
    };
    painted(spans.iter().enumerate().filter_map(|(ix, (range, kind))| {
        let color = match kind {
            Kind::Text if keyed(ix) => syntax.property,
            Kind::Text => syntax.string,
            Kind::Number => syntax.number,
            Kind::Word if matches!(&source[range.clone()], "true" | "false" | "null") => {
                syntax.constant
            }
            Kind::Mark => syntax.punctuation,
            Kind::Word | Kind::Comment => return None,
        };
        Some((range.clone(), color))
    }))
}

/// A one-line code field in the mono face. Give its state `code_highlights`.
#[derive(IntoElement)]
pub struct CodeInput {
    state: Entity<TextInput>,
    size: ControlSize,
}

impl CodeInput {
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

impl RenderOnce for CodeInput {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        assert!(
            !self.state.read(cx).is_multi_line(),
            "a code input is one line; JsonInput takes several"
        );
        div()
            .font_family(cx.theme().mono_family.clone())
            .child(Input::new(&self.state).size(self.size))
    }
}

/// A JSON field, checked as you type. Give its state `json_highlights`.
#[derive(IntoElement)]
pub struct JsonInput {
    state: Entity<TextInput>,
}

impl JsonInput {
    pub fn new(state: &Entity<TextInput>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for JsonInput {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let source = self.state.read(cx).text().to_string();
        let problem = (!source.trim().is_empty())
            .then(|| serde_json::from_str::<serde_json::Value>(&source).err())
            .flatten()
            .map(|error| SharedString::from(error.to_string()));
        let theme = cx.theme();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .font_family(theme.mono_family.clone())
                    .child(Input::new(&self.state).invalid(problem.is_some())),
            )
            .when_some(problem, |field, problem| {
                field.child(
                    div()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(theme.colors.danger)
                        .child(problem),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{Kind, lex};

    fn kinds(source: &str) -> Vec<(&str, Kind)> {
        lex(source)
            .into_iter()
            .map(|(range, kind)| (&source[range], kind))
            .collect()
    }

    #[test]
    fn lexing_splits_strings_numbers_words_and_comments() {
        assert_eq!(
            kinds(r#"let s = "a \"b\""; // done"#),
            [
                ("let", Kind::Word),
                ("s", Kind::Word),
                ("=", Kind::Mark),
                (r#""a \"b\"""#, Kind::Text),
                (";", Kind::Mark),
                ("// done", Kind::Comment),
            ]
        );
        assert_eq!(
            kinds("f(0x1F, 2.5)"),
            [
                ("f", Kind::Word),
                ("(", Kind::Mark),
                ("0x1F", Kind::Number),
                (",", Kind::Mark),
                ("2.5", Kind::Number),
                (")", Kind::Mark),
            ]
        );
    }

    #[test]
    fn an_open_string_stops_at_the_line_end() {
        assert_eq!(kinds("'open"), [("'open", Kind::Text)]);
        assert_eq!(kinds("\"a\n1"), [("\"a", Kind::Text), ("1", Kind::Number)]);
    }
}
