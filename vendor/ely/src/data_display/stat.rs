use gpui::{
    App, Div, ElementId, FontWeight, IntoElement, ParentElement, Refineable, RenderOnce,
    SharedString, StyleRefinement, Styled, Window, div, prelude::*,
};

use crate::{
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
    typography::{AnimatedNumber, format},
};

/// Whether a move is good news; `None` when nothing moved.
fn good(change: f64, down_is_good: bool) -> Option<bool> {
    (change != 0.0).then_some((change > 0.0) != down_is_good)
}

/// Which way a figure moved, as a signed share, and whether down is the good way.
#[derive(IntoElement, Clone, Copy)]
pub struct TrendIndicator {
    change: f64,
    down_is_good: bool,
}

impl TrendIndicator {
    /// `change` is a share: 0.124 reads +12.4%.
    pub fn new(change: f64) -> Self {
        assert!(
            change.is_finite(),
            "a trend needs a finite change, got {change}"
        );
        Self {
            change,
            down_is_good: false,
        }
    }

    /// For costs and wait times, where falling is better.
    pub fn down_is_good(mut self) -> Self {
        self.down_is_good = true;
        self
    }
}

impl RenderOnce for TrendIndicator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = &cx.theme().colors;
        let icon = match self.change {
            change if change > 0.0 => IconName::TrendingUp,
            change if change < 0.0 => IconName::TrendingDown,
            _ => IconName::Minus,
        };
        let tone = match good(self.change, self.down_is_good) {
            None => colors.fg_muted,
            Some(true) => colors.success,
            Some(false) => colors.danger,
        };
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap_1()
            .text_size(cx.theme().text_size(TextSize::Sm))
            .font_weight(FontWeight::MEDIUM)
            .text_color(tone)
            .child(Icon::new(icon).size(IconSize::Sm).color(tone))
            .child(format::percent(self.change, 1, true))
    }
}

/// A figure that matters: its label, the number rolling to each new value, units around it, and how it moved.
#[derive(IntoElement)]
pub struct Statistic {
    id: ElementId,
    label: SharedString,
    value: f64,
    decimals: usize,
    prefix: Option<SharedString>,
    suffix: Option<SharedString>,
    trend: Option<TrendIndicator>,
    size: TextSize,
}

impl Statistic {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>, value: f64) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value,
            decimals: 0,
            prefix: None,
            suffix: None,
            trend: None,
            size: TextSize::Xxl,
        }
    }

    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    /// Before the number, as a currency sign.
    pub fn prefix(mut self, text: impl Into<SharedString>) -> Self {
        self.prefix = Some(text.into());
        self
    }

    /// After the number, as a unit.
    pub fn suffix(mut self, text: impl Into<SharedString>) -> Self {
        self.suffix = Some(text.into());
        self
    }

    pub fn trend(mut self, trend: TrendIndicator) -> Self {
        self.trend = Some(trend);
        self
    }

    /// The number's size; a compact metric takes a smaller one.
    pub fn size(mut self, size: TextSize) -> Self {
        self.size = size;
        self
    }
}

impl RenderOnce for Statistic {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let id = &self.id;
        let unit = |part: &str, text: SharedString| {
            div()
                .debug_selector(|| format!("statistic-{part} {id}"))
                .text_size(theme.text_size(TextSize::Md))
                .text_color(theme.colors.fg_muted)
                .child(text)
        };
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(theme.colors.fg_muted)
                    .child(self.label),
            )
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap_1()
                    .children(self.prefix.map(|text| unit("prefix", text)))
                    .child(
                        AnimatedNumber::new(self.id.clone(), self.value)
                            .decimals(self.decimals)
                            .size(self.size),
                    )
                    .children(self.suffix.map(|text| unit("suffix", text))),
            )
            .children(self.trend)
    }
}

/// A statistic on a quiet card, with an icon and a line of context.
#[derive(IntoElement)]
pub struct KpiCard {
    base: Div,
    statistic: Statistic,
    icon: Option<IconName>,
    caption: Option<SharedString>,
}

impl KpiCard {
    pub fn new(statistic: Statistic) -> Self {
        Self {
            base: div(),
            statistic,
            icon: None,
            caption: None,
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn caption(mut self, text: impl Into<SharedString>) -> Self {
        self.caption = Some(text.into());
        self
    }
}

impl Styled for KpiCard {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for KpiCard {
    fn render(mut self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let mut card = div();
        card.style().refine(self.base.style());
        card.relative()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .when_some(self.icon, |card, icon| {
                card.child(
                    div()
                        .absolute()
                        .top_4()
                        .right_4()
                        .child(Icon::new(icon).size(IconSize::Md).color(colors.fg_subtle)),
                )
            })
            .child(self.statistic)
            .when_some(self.caption, |card, caption| {
                card.child(
                    div()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(colors.fg_subtle)
                        .child(caption),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::good;

    #[test]
    fn a_trend_is_good_when_it_goes_the_right_way() {
        assert_eq!(good(0.12, false), Some(true));
        assert_eq!(good(-0.12, false), Some(false));
        assert_eq!(good(-0.08, true), Some(true));
        assert_eq!(good(0.08, true), Some(false));
        assert_eq!(good(0.0, true), None);
    }
}
