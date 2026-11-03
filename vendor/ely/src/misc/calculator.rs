use gpui::{
    App, ClickEvent, ElementId, Entity, InteractiveElement, IntoElement, KeyDownEvent, MouseButton,
    ParentElement, RenderOnce, Styled, Window, div, prelude::*,
};

use crate::{
    buttons::{Button, ButtonVariant},
    forms::evaluate,
    primitives::{FocusRing, IconName, tab_stop},
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
    typography::{
        format::{MINUS, Separators, group_digits, significant},
        tabular,
    },
};

/// A key on the pad or the keyboard.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Key {
    Digit(char),
    Point,
    Op(char),
    Sign,
    Back,
    Clear,
    Equals,
}

/// The pad, four keys a row, each with what it shows; zero takes two.
const PAD: [(Key, &str); 19] = [
    (Key::Clear, "AC"),
    (Key::Sign, "±"),
    (Key::Back, "Delete"),
    (Key::Op('/'), "÷"),
    (Key::Digit('7'), "7"),
    (Key::Digit('8'), "8"),
    (Key::Digit('9'), "9"),
    (Key::Op('*'), "×"),
    (Key::Digit('4'), "4"),
    (Key::Digit('5'), "5"),
    (Key::Digit('6'), "6"),
    (Key::Op('-'), "−"),
    (Key::Digit('1'), "1"),
    (Key::Digit('2'), "2"),
    (Key::Digit('3'), "3"),
    (Key::Op('+'), "+"),
    (Key::Digit('0'), "0"),
    (Key::Point, "."),
    (Key::Equals, "="),
];

const OPS: [char; 4] = ['+', '-', '*', '/'];

/// Stands for the last result in the entry, so a sum goes on with its whole value.
const ANSWER: char = 'a';

/// The entry as typed, in ASCII, where `a` is the last result, kept whole in `answer`; the sum that gave it, as shown; and why a sum failed.
#[derive(Default)]
struct Tape {
    entry: String,
    answer: f64,
    worked: Option<String>,
    problem: Option<String>,
}

impl Tape {
    /// Where the number being typed starts, its sign included.
    fn number_start(&self) -> usize {
        let bytes = self.entry.as_bytes();
        let start = self.entry.rfind(OPS).map_or(0, |at| at + 1);
        let signed = start > 0
            && bytes[start - 1] == b'-'
            && (start == 1 || OPS.contains(&(bytes[start - 2] as char)));
        if signed { start - 1 } else { start }
    }

    fn blank(&self) -> bool {
        self.entry.is_empty() && self.worked.is_none() && self.problem.is_none()
    }

    fn press(&mut self, key: Key) {
        if self.problem.take().is_some() {
            self.entry.clear();
        }
        self.worked = None;
        let answered = self.entry.ends_with(ANSWER);
        if answered && matches!(key, Key::Digit(_) | Key::Point) {
            self.entry.clear();
        }
        let start = self.number_start();
        let number = &self.entry[start..];
        match key {
            Key::Digit(digit) => {
                if number.trim_start_matches('-') == "0" {
                    self.entry.pop();
                }
                self.entry.push(digit);
            }
            Key::Point if number.contains('.') => {}
            Key::Point => {
                if number.trim_start_matches('-').is_empty() {
                    self.entry.push('0');
                }
                self.entry.push('.');
            }
            Key::Op(op) => {
                let kept = self.entry.trim_end_matches(OPS).len();
                match kept {
                    0 if self.entry.is_empty() && op == '-' => self.entry.push('-'),
                    0 => {}
                    _ => {
                        self.entry.truncate(kept);
                        self.entry.push(op);
                    }
                }
            }
            Key::Sign if answered => self.answer = -self.answer,
            Key::Sign if number.starts_with('-') => {
                self.entry.remove(start);
            }
            Key::Sign if !number.is_empty() => self.entry.insert(start, '-'),
            Key::Sign => {}
            Key::Back => {
                self.entry.pop();
            }
            Key::Clear => self.entry.clear(),
            Key::Equals if self.entry.is_empty() || self.entry.ends_with(OPS) => {}
            Key::Equals => {
                self.worked = Some(format!("{} =", shown(&self.entry, self.answer)));
                match evaluate(&self.entry, &[(ANSWER.to_string().into(), self.answer)]) {
                    Ok(value) => (self.entry, self.answer) = (ANSWER.to_string(), value),
                    Err(problem) => self.problem = Some(problem),
                }
            }
        }
    }
}

/// The entry as it reads: digits grouped, the last result to twelve significant digits, and ×, ÷ and a true minus between spaced terms.
fn shown(entry: &str, answer: f64) -> String {
    let (mut out, mut number, mut after_op) = (String::new(), String::new(), true);
    let flush = |out: &mut String, number: &mut String| {
        let (whole, part) = number.split_once('.').unwrap_or((number, ""));
        out.push_str(&group_digits(whole, ','));
        if number.contains('.') {
            out.push('.');
            out.push_str(part);
        }
        number.clear();
    };
    for ch in entry.chars() {
        if ch == ANSWER {
            out.push_str(&significant(answer, 12, Separators::EN));
            after_op = false;
            continue;
        }
        if ch.is_ascii_digit() || ch == '.' {
            number.push(ch);
            after_op = false;
            continue;
        }
        flush(&mut out, &mut number);
        match (after_op, ch) {
            (true, _) => out.push(MINUS),
            (_, '+') => out.push_str(" + "),
            (_, '-') => out.push_str(" − "),
            (_, '*') => out.push_str(" × "),
            _ => out.push_str(" ÷ "),
        }
        after_op = true;
    }
    flush(&mut out, &mut number);
    out.trim_end().to_string()
}

fn hit(tape: &Entity<Tape>, key: Key, cx: &mut App) {
    tape.update(cx, |tape, cx| {
        tape.press(key);
        log::info!(
            "calculator: {key:?}, entry {:?}, answer {}",
            tape.entry,
            tape.answer
        );
        cx.notify();
    });
}

/// A key typed on the keyboard; Enter is equals only while the calculator itself has focus, so a focused key presses on its own.
fn typed(event: &KeyDownEvent, own: bool) -> Option<Key> {
    let stroke = &event.keystroke;
    if stroke.modifiers.platform || stroke.modifiers.control {
        return None;
    }
    match (stroke.key.as_str(), stroke.key_char.as_deref()) {
        ("enter", _) if own => Some(Key::Equals),
        ("backspace", _) => Some(Key::Back),
        ("escape", _) => Some(Key::Clear),
        (_, Some("=")) => Some(Key::Equals),
        (_, Some(".")) => Some(Key::Point),
        (_, Some(op @ ("+" | "-" | "*" | "/"))) => op.chars().next().map(Key::Op),
        (_, Some(digit)) if digit.len() == 1 && digit.as_bytes()[0].is_ascii_digit() => {
            digit.chars().next().map(Key::Digit)
        }
        _ => None,
    }
}

/// A calculator: the sum as you type it over its result, and a pad of digits, the four operations, sign, delete and clear. Keys type too: digits, `.`, `+ - * /`, `=` or Enter, Backspace and Escape; Equals waits while the sum ends in an operator. It works in precedence, × and ÷ before + and −, and shows twelve significant digits.
#[derive(IntoElement)]
pub struct Calculator {
    id: ElementId,
}

impl Calculator {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self { id: id.into() }
    }
}

impl RenderOnce for Calculator {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let tape = window.use_keyed_state((id.clone(), "tape"), cx, |_, _| Tape::default());
        let focus = tab_stop((id.clone(), "focus").into(), true, window, cx);
        let theme = cx.theme();
        let colors = &theme.colors;
        let (above, main, failed) = {
            let tape = tape.read(cx);
            let main = match (&tape.problem, tape.entry.is_empty()) {
                (Some(problem), _) => problem.clone(),
                (None, true) => "0".to_string(),
                (None, false) => shown(&tape.entry, tape.answer),
            };
            (tape.worked.clone(), main, tape.problem.is_some())
        };
        let keys = PAD.iter().map(|&(key, label)| {
            let button = match key {
                Key::Back => Button::new((id.clone(), "back"), "").icon(IconName::Delete),
                _ => Button::new((id.clone(), label), label),
            };
            let tape = tape.clone();
            div()
                .debug_selector(|| format!("key-{label}"))
                .when(key == Key::Digit('0'), |cell| cell.col_span(2))
                .child(
                    button
                        .size(ControlSize::Lg)
                        .variant(match key {
                            Key::Equals => ButtonVariant::Primary,
                            Key::Digit(_) | Key::Point => ButtonVariant::Secondary,
                            Key::Op(_) => ButtonVariant::Outline,
                            _ => ButtonVariant::Ghost,
                        })
                        .full_width()
                        .on_click(move |_: &ClickEvent, _, cx| hit(&tape, key, cx)),
                )
        });
        div()
            .id(id.clone())
            .track_focus(&focus)
            .p_3()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .focus_ring(cx)
            .on_mouse_down(MouseButton::Left, {
                let focus = focus.clone();
                move |_, window, cx| window.focus(&focus, cx)
            })
            .on_key_down({
                let tape = tape.clone();
                move |event, window, cx| {
                    let Some(key) = typed(event, focus.is_focused(window)) else {
                        return;
                    };
                    if key == Key::Clear && tape.read(cx).blank() {
                        return;
                    }
                    cx.stop_propagation();
                    hit(&tape, key, cx);
                }
            })
            .child(
                div()
                    .pb_3()
                    .text_right()
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(colors.fg_muted)
                            .child(above.unwrap_or_else(|| "\u{a0}".into())),
                    )
                    .child(
                        tabular(div())
                            .debug_selector(|| format!("calculator-{main}"))
                            .text_size(theme.text_size(TextSize::Xxl))
                            .text_color(if failed { colors.danger } else { colors.fg })
                            .child(main.clone()),
                    ),
            )
            .child(div().grid().grid_cols(4).gap_2().children(keys))
    }
}

#[cfg(test)]
mod tests {
    use super::{Key, Tape, shown};

    fn typed(keys: &[Key]) -> Tape {
        let mut tape = Tape::default();
        keys.iter().for_each(|key| tape.press(*key));
        tape
    }

    fn sum(text: &str) -> Vec<Key> {
        text.chars()
            .map(|ch| match ch {
                '0'..='9' => Key::Digit(ch),
                '.' => Key::Point,
                '=' => Key::Equals,
                '~' => Key::Sign,
                '<' => Key::Back,
                'C' => Key::Clear,
                op => Key::Op(op),
            })
            .collect()
    }

    /// What the calculator's main line reads after `keys`.
    fn reads(keys: &str) -> String {
        let tape = typed(&sum(keys));
        match tape.problem {
            Some(problem) => problem,
            None if tape.entry.is_empty() => "0".into(),
            None => shown(&tape.entry, tape.answer),
        }
    }

    #[test]
    fn it_works_in_precedence_and_goes_on_with_the_whole_result() {
        assert_eq!(
            typed(&sum("12+3*4=")).worked.as_deref(),
            Some("12 + 3 × 4 =")
        );
        assert_eq!(reads("12+3*4="), "24");
        assert_eq!(
            reads("12+3*4=-4="),
            "20",
            "an operator goes on from the result"
        );
        assert_eq!(
            reads("1/3=*3="),
            "1",
            "the whole result, not its twelve digits"
        );
        assert_eq!(reads("12+3*4=7"), "7", "a digit starts afresh");
        assert_eq!(reads("12+3*4=.5"), "0.5", "so does a point");
        assert_eq!(reads("12+3*4=~+4="), "−20", "sign turns the result");
        assert_eq!(reads("12+3*4=<"), "0", "delete takes the whole result");
        assert_eq!(reads(".1+.2="), "0.3", "twelve digits hide binary noise");
        assert_eq!(reads("2/3="), "0.666666666667");
    }

    #[test]
    fn keys_keep_the_entry_a_sum() {
        assert_eq!(typed(&sum("007")).entry, "7", "no leading zeros");
        assert_eq!(typed(&sum("1..5.")).entry, "1.5", "one point a number");
        assert_eq!(
            typed(&sum(".5+.5")).entry,
            "0.5+0.5",
            "a point starts with a zero"
        );
        assert_eq!(
            typed(&sum("5+*")).entry,
            "5*",
            "an operator replaces the last"
        );
        assert_eq!(typed(&sum("+-")).entry, "-", "only minus starts, as a sign");
        assert_eq!(
            typed(&sum("5*3~")).entry,
            "5*-3",
            "sign turns the number typed"
        );
        assert_eq!(typed(&sum("5*3~~")).entry, "5*3");
        assert_eq!(
            typed(&sum("5*3~<+")).entry,
            "5+",
            "a dangling sign goes with the operator"
        );
        assert_eq!(typed(&sum("12+3C")).entry, "");
        assert_eq!(
            reads("1234+5*="),
            "1,234 + 5 ×",
            "Equals waits on a dangling operator"
        );
    }

    #[test]
    fn a_failed_sum_says_why_and_the_next_key_starts_afresh() {
        assert_eq!(reads("5/0="), "division by zero");
        assert_eq!(typed(&sum("5/0=")).worked.as_deref(), Some("5 ÷ 0 ="));
        assert_eq!(typed(&sum("5/0=8")).entry, "8");
        assert_eq!(typed(&sum("5/0=+")).entry, "", "no result to go on from");
    }

    #[test]
    fn the_entry_reads_grouped_with_true_signs() {
        assert_eq!(shown("-1234.5*-6/7-8", 0.0), "−1,234.5 × −6 ÷ 7 − 8");
        assert_eq!(shown("0.", 0.0), "0.");
        assert_eq!(shown("a*2", -1_234.5), "−1,234.5 × 2");
    }
}
