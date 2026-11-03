use std::{ops::Range, rc::Rc, sync::LazyLock};

use gpui::{
    App, ElementId, HighlightStyle, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, StyledText, Window, div,
    prelude::*, uniform_list,
};
use regex::Regex;

use super::OnIndex;
use crate::{
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize, TextSize},
};

/// An instruction: its address, its bytes, its mnemonic and operands, an optional comment, and the source line it starts, when known.
#[derive(Clone, Debug, PartialEq)]
pub struct Instruction {
    pub address: u64,
    pub bytes: Vec<u8>,
    pub mnemonic: SharedString,
    pub operands: SharedString,
    pub comment: Option<SharedString>,
    pub source: Option<SharedString>,
}

/// How a piece of an operand reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Piece {
    Register,
    Number,
    Mark,
}

static PIECES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?P<register>\b(?:[re]?(?:ax|bx|cx|dx|si|di|sp|bp|ip)|r(?:[89]|1[0-5])[dwb]?|[xyz]mm[0-9]{1,2}|[abcd][lh]|[xw](?:[12]?[0-9]|30)|sp|lr|fp|pc|[xw]zr|[vqdsbh][0-9]{1,2})\b)
        |(?P<number>\#?-?(?:0x[0-9a-fA-F]+|\b[0-9]+\b))
        |(?P<mark>[\[\],+*!\-\#])",
    )
    .expect("the operand pattern compiles")
});

/// Registers, numbers and marks in `operands`, in order.
pub(crate) fn pieces(operands: &str) -> Vec<(Range<usize>, Piece)> {
    PIECES
        .captures_iter(operands)
        .map(|found| {
            let (kind, hit) = if let Some(hit) = found.name("register") {
                (Piece::Register, hit)
            } else if let Some(hit) = found.name("number") {
                (Piece::Number, hit)
            } else {
                (
                    Piece::Mark,
                    found.name("mark").expect("a piece is one of three"),
                )
            };
            (hit.range(), kind)
        })
        .collect()
}

/// A row: an instruction, or the source line above one.
#[derive(Clone, Copy)]
enum Line {
    Source(usize),
    Code(usize),
}

/// Machine code as read: address, bytes, the mnemonic and its operands colored, comments, and the source lines they came from. The current instruction is marked; a press in the gutter sets or clears a breakpoint. Long listings draw only the rows in view; it fills its box.
#[derive(IntoElement)]
pub struct Disassembly {
    id: ElementId,
    instructions: Rc<Vec<Instruction>>,
    current: Option<usize>,
    breakpoints: Vec<usize>,
    on_breakpoint: Option<OnIndex>,
}

impl Disassembly {
    pub fn new(id: impl Into<ElementId>, instructions: impl Into<Rc<Vec<Instruction>>>) -> Self {
        Self {
            id: id.into(),
            instructions: instructions.into(),
            current: None,
            breakpoints: Vec::new(),
            on_breakpoint: None,
        }
    }

    pub fn current(mut self, instruction: usize) -> Self {
        self.current = Some(instruction);
        self
    }

    pub fn breakpoints(mut self, instructions: impl IntoIterator<Item = usize>) -> Self {
        self.breakpoints = instructions.into_iter().collect();
        self
    }

    pub fn on_breakpoint(
        mut self,
        handler: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_breakpoint = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Disassembly {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let lines: Rc<Vec<Line>> = Rc::new(
            self.instructions
                .iter()
                .enumerate()
                .flat_map(|(ix, instruction)| {
                    instruction
                        .source
                        .as_ref()
                        .map(|_| Line::Source(ix))
                        .into_iter()
                        .chain([Line::Code(ix)])
                })
                .collect(),
        );
        let widest = self
            .instructions
            .iter()
            .map(|instruction| instruction.bytes.len())
            .max()
            .unwrap_or(0);
        let (instructions, current, breakpoints, on_breakpoint, id) = (
            self.instructions.clone(),
            self.current,
            self.breakpoints.clone(),
            self.on_breakpoint.clone(),
            self.id.clone(),
        );
        let list = uniform_list(
            (self.id.clone(), "rows"),
            lines.len(),
            move |range, _, cx| {
                let colors = cx.theme().colors.clone();
                let syntax = colors.syntax;
                range
                    .map(|row| match lines[row] {
                        Line::Source(ix) => div()
                            .id(row)
                            .w_full()
                            .pl_8()
                            .whitespace_nowrap()
                            .text_color(syntax.comment)
                            .child(
                                instructions[ix]
                                    .source
                                    .clone()
                                    .expect("a source row has its line"),
                            ),
                        Line::Code(ix) => {
                            let instruction = &instructions[ix];
                            let styles: Vec<(Range<usize>, HighlightStyle)> =
                                pieces(&instruction.operands)
                                    .into_iter()
                                    .map(|(range, piece)| {
                                        let color = match piece {
                                            Piece::Register => syntax.variable,
                                            Piece::Number => syntax.number,
                                            Piece::Mark => syntax.punctuation,
                                        };
                                        (range, color.into())
                                    })
                                    .collect();
                            let (marked, is_current) =
                                (breakpoints.contains(&ix), current == Some(ix));
                            let toggle = on_breakpoint.clone();
                            let hex: Vec<String> = instruction
                                .bytes
                                .iter()
                                .map(|byte| format!("{byte:02x}"))
                                .collect();
                            div()
                                .id(row)
                                .w_full()
                                .flex()
                                .gap_3()
                                .whitespace_nowrap()
                                .when(is_current, |line| line.bg(colors.warning.opacity(0.12)))
                                .child(
                                    div()
                                        .id((id.clone(), format!("gutter-{ix}")))
                                        .flex_none()
                                        .w_5()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .cursor_pointer()
                                        .on_mouse_down(MouseButton::Left, |_, window, _| {
                                            window.prevent_default()
                                        })
                                        .when_some(toggle, |gutter, toggle| {
                                            gutter.on_click(move |_, window, cx| {
                                                toggle(ix, window, cx)
                                            })
                                        })
                                        .map(|gutter| {
                                            if is_current {
                                                gutter.child(
                                                    Icon::new(IconName::ArrowRight)
                                                        .size(IconSize::Xs)
                                                        .color(colors.warning),
                                                )
                                            } else if marked {
                                                gutter.child(
                                                    div().size_2().rounded_full().bg(colors.danger),
                                                )
                                            } else {
                                                gutter
                                            }
                                        }),
                                )
                                .child(
                                    div()
                                        .flex_none()
                                        .text_color(colors.fg_subtle)
                                        .child(format!("{:#010x}", instruction.address)),
                                )
                                .child(div().flex_none().text_color(colors.fg_subtle).child(
                                    format!("{:<width$}", hex.join(" "), width = widest * 3),
                                ))
                                .child(
                                    div()
                                        .flex_none()
                                        .w_12()
                                        .text_color(syntax.keyword)
                                        .child(instruction.mnemonic.clone()),
                                )
                                .child(
                                    StyledText::new(instruction.operands.clone())
                                        .with_highlights(styles),
                                )
                                .children(instruction.comment.clone().map(|comment| {
                                    div()
                                        .text_color(syntax.comment)
                                        .child(format!("; {comment}"))
                                }))
                        }
                    })
                    .collect()
            },
        )
        .size_full();
        div()
            .size_full()
            .font_family(theme.mono_family.clone())
            .text_size(theme.text_size(TextSize::Xs))
            .text_color(theme.colors.fg)
            .child(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(operands: &str) -> Vec<(&str, Piece)> {
        pieces(operands)
            .into_iter()
            .map(|(range, piece)| (&operands[range], piece))
            .collect()
    }

    #[test]
    fn operands_split_into_registers_numbers_and_marks() {
        use Piece::*;
        assert_eq!(
            kinds("qword ptr [rbp - 0x18], rax"),
            [
                ("[", Mark),
                ("rbp", Register),
                ("-", Mark),
                ("0x18", Number),
                ("]", Mark),
                (",", Mark),
                ("rax", Register)
            ]
        );
        assert_eq!(
            kinds("x0, [sp, #16]"),
            [
                ("x0", Register),
                (",", Mark),
                ("[", Mark),
                ("sp", Register),
                (",", Mark),
                ("#16", Number),
                ("]", Mark)
            ]
        );
    }
}
