use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div, prelude::*,
};

use crate::{
    buttons::{Button, ButtonVariant},
    data_display::{Badge, Tone},
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
    typography::Ellipsis,
};

/// Which answer won a comparison, or neither.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Side(usize),
    Tie,
}

type OnVerdict = Rc<dyn Fn(Verdict, &mut Window, &mut App)>;

/// A side's heading: its model's name, or a letter while the comparison is blind and undecided.
pub(crate) fn heading(ix: usize, name: &SharedString, blind: bool, decided: bool) -> SharedString {
    match blind && !decided {
        true => format!("Model {}", char::from(b'A' + ix as u8)).into(),
        false => name.clone(),
    }
}

/// Answers from two to four models side by side, each under its model's name, with a way to pick the better or call a tie. Blind, the names stay letters until the verdict; after it, the winner takes accent. Narrow, the sides stack.
#[derive(IntoElement)]
pub struct ABCompareView {
    id: ElementId,
    sides: Vec<(SharedString, AnyElement)>,
    blind: bool,
    verdict: Option<Verdict>,
    on_verdict: Option<OnVerdict>,
}

impl ABCompareView {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            sides: Vec::new(),
            blind: false,
            verdict: None,
            on_verdict: None,
        }
    }

    /// One model's answer under its name.
    pub fn side(mut self, name: impl Into<SharedString>, answer: impl IntoElement) -> Self {
        self.sides.push((name.into(), answer.into_any_element()));
        self
    }

    /// Hides the names until the verdict.
    pub fn blind(mut self) -> Self {
        self.blind = true;
        self
    }

    pub fn verdict(mut self, verdict: Verdict) -> Self {
        self.verdict = Some(verdict);
        self
    }

    pub fn on_verdict(
        mut self,
        handler: impl Fn(Verdict, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_verdict = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ABCompareView {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let count = self.sides.len();
        assert!((2..=4).contains(&count), "a comparison of {count} sides");
        let verdict = match self.verdict {
            Some(Verdict::Side(ix)) if ix >= count => {
                log::error!("comparison: verdict for side {ix} of {count}; none given");
                None
            }
            verdict => verdict,
        };
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let decided = verdict.is_some();
        let judge = |verdict: Verdict, label: SharedString, key: String| {
            self.on_verdict.clone().map(|judge| {
                Button::new((self.id.clone(), key), label)
                    .variant(ButtonVariant::Secondary)
                    .size(ControlSize::Sm)
                    .on_click(move |_, window, cx| {
                        log::info!("comparison: {verdict:?}");
                        judge(verdict, window, cx)
                    })
            })
        };
        let mut titles = Vec::new();
        let sides: Vec<_> = self
            .sides
            .into_iter()
            .enumerate()
            .map(|(ix, (name, answer))| {
                let won = verdict == Some(Verdict::Side(ix));
                let title = heading(ix, &name, self.blind, decided);
                titles.push(title.clone());
                div()
                    .flex_1()
                    .min_w(theme.label_width())
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .rounded(theme.radius(Radius::Lg))
                    .border_1()
                    .border_color(if won { colors.accent } else { colors.border })
                    .bg(colors.surface)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(theme.text_size(TextSize::Sm))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(colors.fg)
                                    .child(Ellipsis::new(title)),
                            )
                            .when(won, |row| {
                                row.child(
                                    div()
                                        .flex_none()
                                        .child(Badge::new("Preferred").tone(Tone::Accent)),
                                )
                            }),
                    )
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(colors.fg)
                            .child(answer),
                    )
            })
            .collect();
        let verdicts = match verdict {
            Some(Verdict::Tie) => vec![div().child("Called a tie").into_any_element()],
            Some(Verdict::Side(_)) => Vec::new(),
            None => titles
                .iter()
                .enumerate()
                .filter_map(|(ix, title)| {
                    judge(
                        Verdict::Side(ix),
                        format!("{title} is better").into(),
                        format!("better-{ix}"),
                    )
                    .map(IntoElement::into_any_element)
                })
                .chain(
                    judge(Verdict::Tie, "Tie".into(), "tie".into())
                        .map(IntoElement::into_any_element),
                )
                .collect(),
        };
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(div().w_full().flex().flex_wrap().gap_3().children(sides))
            .when(!verdicts.is_empty(), |column| {
                column.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_2()
                        .text_size(theme.text_size(TextSize::Sm))
                        .text_color(colors.fg_muted)
                        .children(verdicts),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::heading;

    #[test]
    fn a_blind_side_shows_a_letter_until_the_verdict() {
        let name = "Orchid".into();
        assert_eq!(heading(1, &name, true, false), "Model B");
        assert_eq!(heading(1, &name, true, true), "Orchid");
        assert_eq!(heading(0, &name, false, false), "Orchid");
    }
}
