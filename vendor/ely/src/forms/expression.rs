use std::ops::Range;

use gpui::{
    App, Entity, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
    prelude::*,
};

use super::{Highlight, Input, TextInput};
use crate::{
    theme::{ActiveTheme, ControlSize, TextSize},
    typography::format::{self, Separators},
};

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(f64),
    Name(String),
    Op(char),
    Open,
    Close,
    Comma,
}

fn tokens(source: &str) -> Result<Vec<(Range<usize>, Token)>, String> {
    let mut out = Vec::new();
    let mut chars = source.char_indices().peekable();
    while let Some(&(ix, ch)) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }
        if ch.is_ascii_digit() || ch == '.' {
            let mut end = ix;
            while let Some(&(at, ch)) = chars.peek() {
                if !(ch.is_ascii_digit() || ch == '.') {
                    break;
                }
                end = at + 1;
                chars.next();
            }
            let number = source[ix..end]
                .parse::<f64>()
                .ok()
                .filter(|number| number.is_finite())
                .ok_or_else(|| format!("{} is not a number", &source[ix..end]))?;
            out.push((ix..end, Token::Number(number)));
            continue;
        }
        if ch.is_alphabetic() || ch == '_' {
            let mut end = ix;
            while let Some(&(at, ch)) = chars.peek() {
                if !(ch.is_alphanumeric() || ch == '_') {
                    break;
                }
                end = at + ch.len_utf8();
                chars.next();
            }
            out.push((ix..end, Token::Name(source[ix..end].to_string())));
            continue;
        }
        chars.next();
        let token = match ch {
            '+' | '-' | '*' | '/' | '%' | '^' => Token::Op(ch),
            '(' => Token::Open,
            ')' => Token::Close,
            ',' => Token::Comma,
            other => return Err(format!("unexpected {other}")),
        };
        out.push((ix..ix + ch.len_utf8(), token));
    }
    Ok(out)
}

/// Passes `value` on, or names the step that left the finite numbers.
fn finite(value: f64, step: &str) -> Result<f64, String> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!("{step} has no finite value"))
    }
}

struct Parser<'a> {
    tokens: Vec<Token>,
    at: usize,
    variables: &'a [(SharedString, f64)],
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.at)
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.at).cloned();
        self.at += 1;
        token
    }

    fn sum(&mut self) -> Result<f64, String> {
        let mut value = self.product()?;
        while let Some(Token::Op(op @ ('+' | '-'))) = self.peek().cloned() {
            self.next();
            let rhs = self.product()?;
            value = finite(if op == '+' { value + rhs } else { value - rhs }, "+")?;
        }
        Ok(value)
    }

    fn product(&mut self) -> Result<f64, String> {
        let mut value = self.power()?;
        while let Some(Token::Op(op @ ('*' | '/' | '%'))) = self.peek().cloned() {
            self.next();
            let rhs = self.power()?;
            if op != '*' && rhs == 0.0 {
                return Err("division by zero".into());
            }
            value = match op {
                '*' => finite(value * rhs, "*")?,
                '/' => finite(value / rhs, "/")?,
                _ => value % rhs,
            };
        }
        Ok(value)
    }

    fn power(&mut self) -> Result<f64, String> {
        let base = self.unary()?;
        if self.peek() == Some(&Token::Op('^')) {
            self.next();
            return finite(base.powf(self.power()?), "^");
        }
        Ok(base)
    }

    fn unary(&mut self) -> Result<f64, String> {
        if self.peek() == Some(&Token::Op('-')) {
            self.next();
            return Ok(-self.unary()?);
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<f64, String> {
        match self.next() {
            Some(Token::Number(number)) => Ok(number),
            Some(Token::Open) => {
                let value = self.sum()?;
                match self.next() {
                    Some(Token::Close) => Ok(value),
                    _ => Err("a ( is never closed".into()),
                }
            }
            Some(Token::Name(name)) if self.peek() == Some(&Token::Open) => {
                self.next();
                let mut args = Vec::new();
                if self.peek() != Some(&Token::Close) {
                    loop {
                        args.push(self.sum()?);
                        match self.next() {
                            Some(Token::Comma) => continue,
                            Some(Token::Close) => break,
                            _ => return Err(format!("{name}( is never closed")),
                        }
                    }
                } else {
                    self.next();
                }
                finite(call(&name, &args)?, &format!("{name}(…)"))
            }
            Some(Token::Name(name)) => {
                let (_, value) = self
                    .variables
                    .iter()
                    .find(|(known, _)| known.as_ref() == name)
                    .ok_or_else(|| format!("{name} is not defined"))?;
                finite(*value, &name)
            }
            Some(other) => Err(format!("unexpected {other:?}")),
            None => Err("the formula ends early".into()),
        }
    }
}

fn call(name: &str, args: &[f64]) -> Result<f64, String> {
    let one = |f: fn(f64) -> f64| match args {
        [x] => Ok(f(*x)),
        _ => Err(format!("{name} takes one value")),
    };
    match name {
        "abs" => one(f64::abs),
        "sqrt" => one(f64::sqrt),
        "round" => one(f64::round),
        "floor" => one(f64::floor),
        "ceil" => one(f64::ceil),
        "min" | "max" if args.is_empty() => Err(format!("{name} needs values")),
        "min" => Ok(args.iter().copied().fold(f64::INFINITY, f64::min)),
        "max" => Ok(args.iter().copied().fold(f64::NEG_INFINITY, f64::max)),
        _ => Err(format!("{name} is not a function")),
    }
}

/// Evaluates a formula with the given variables.
pub fn evaluate(source: &str, variables: &[(SharedString, f64)]) -> Result<f64, String> {
    let tokens = tokens(source)?
        .into_iter()
        .map(|(_, token)| token)
        .collect();
    let mut parser = Parser {
        tokens,
        at: 0,
        variables,
    };
    let value = parser.sum()?;
    match parser.peek() {
        None => Ok(value),
        Some(extra) => Err(format!("unexpected {extra:?}")),
    }
}

/// Colors for a formula: numbers, names, operators.
pub fn expression_highlights(source: &str, cx: &App) -> Vec<(Range<usize>, Highlight)> {
    let syntax = &cx.theme().colors.syntax;
    let Ok(found) = tokens(source) else {
        return Vec::new();
    };
    found
        .iter()
        .enumerate()
        .map(|(ix, (range, token))| {
            let color = match token {
                Token::Number(_) => syntax.number,
                Token::Name(_) if matches!(found.get(ix + 1), Some((_, Token::Open))) => {
                    syntax.function
                }
                Token::Name(_) => syntax.property,
                Token::Op(_) => syntax.operator,
                Token::Open | Token::Close | Token::Comma => syntax.punctuation,
            };
            (range.clone(), Highlight::new(color))
        })
        .collect()
}

/// A formula field: colored as you type, with its value or its problem below.
#[derive(IntoElement)]
pub struct ExpressionInput {
    state: Entity<TextInput>,
    variables: Vec<(SharedString, f64)>,
}

impl ExpressionInput {
    /// Give the state `expression_highlights` as its highlighter.
    pub fn new(state: &Entity<TextInput>) -> Self {
        Self {
            state: state.clone(),
            variables: Vec::new(),
        }
    }

    pub fn variable(mut self, name: impl Into<SharedString>, value: f64) -> Self {
        self.variables.push((name.into(), value));
        self
    }
}

impl RenderOnce for ExpressionInput {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let source = self.state.read(cx).text().to_string();
        let result = (!source.trim().is_empty()).then(|| evaluate(&source, &self.variables));
        let theme = cx.theme();
        let colors = &theme.colors;
        let mono = theme.mono_family.clone();
        let invalid = matches!(result, Some(Err(_)));
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div().font_family(mono).child(
                    Input::new(&self.state)
                        .size(ControlSize::Md)
                        .invalid(invalid)
                        .prefix(div().text_color(colors.fg_subtle).child("=")),
                ),
            )
            .when_some(result, |field, result| {
                let (text, color) = match result {
                    Ok(value) => (
                        format!(
                            "= {}",
                            format::number(value, 4, Separators::EN)
                                .trim_end_matches('0')
                                .trim_end_matches('.')
                        ),
                        colors.fg_muted,
                    ),
                    Err(problem) => (problem, colors.danger),
                };
                field.child(
                    div()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(color)
                        .child(SharedString::from(text)),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use gpui::SharedString;

    use super::evaluate;

    #[test]
    fn precedence_functions_and_variables() {
        let vars = [
            (SharedString::from("price"), 12.5),
            (SharedString::from("qty"), 3.0),
        ];
        assert_eq!(evaluate("1 + 2 * 3", &vars), Ok(7.0));
        assert_eq!(evaluate("2 ^ 3 ^ 2", &vars), Ok(512.0));
        assert_eq!(evaluate("-(price * qty) + max(1, 4, 2)", &vars), Ok(-33.5));
        assert_eq!(evaluate("round(sqrt(16.4))", &vars), Ok(4.0));
    }

    #[test]
    fn problems_are_named() {
        let vars = [];
        assert_eq!(evaluate("1 / 0", &vars), Err("division by zero".into()));
        assert_eq!(evaluate("tax * 2", &vars), Err("tax is not defined".into()));
        assert_eq!(evaluate("(1 + 2", &vars), Err("a ( is never closed".into()));
        assert_eq!(
            evaluate("abs(1, 2)", &vars),
            Err("abs takes one value".into())
        );
        assert!(evaluate("1 $ 2", &vars).is_err());
    }

    #[test]
    fn a_step_without_a_finite_value_fails_at_once() {
        assert_eq!(
            evaluate("min(sqrt(-1), 2)", &[]),
            Err("sqrt(…) has no finite value".into())
        );
        assert_eq!(
            evaluate("max(10 ^ 400, 1)", &[]),
            Err("^ has no finite value".into())
        );
    }
}
