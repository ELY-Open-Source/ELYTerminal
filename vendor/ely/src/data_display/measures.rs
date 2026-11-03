use std::{
    f32::consts::{FRAC_PI_4, TAU},
    rc::Rc,
};

use gpui::{
    Animation, AnimationExt, App, ElementId, FontWeight, Hsla, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, Styled, Window, canvas, div, prelude::*, relative,
};

use crate::{
    motion,
    theme::{ActiveTheme, Palette, TextSize},
    typography::{AnimatedNumber, Ellipsis, format, tabular},
};

/// Where a filled share stands against a meter's thresholds; it indexes `tones`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Level {
    Calm = 0,
    Warn = 1,
    Danger = 2,
}

impl Level {
    fn of(share: f32, (warn, danger): (f32, f32)) -> Self {
        if share >= danger {
            Self::Danger
        } else if share >= warn {
            Self::Warn
        } else {
            Self::Calm
        }
    }
}

/// A fill's color for each level: plain, then amber, then red.
fn tones(colors: &Palette) -> [Hsla; 3] {
    [colors.fg, colors.warning, colors.danger]
}

/// A share's color against its limits: plain, amber past the first, red past the second.
pub(crate) fn tone(share: f32, limits: (f32, f32), colors: &Palette) -> Hsla {
    tones(colors)[Level::of(share, thresholds(limits.0, limits.1)) as usize]
}

fn thresholds(warn: f32, danger: f32) -> (f32, f32) {
    assert!(
        0.0 < warn && warn <= danger && danger <= 1.0,
        "thresholds {warn} and {danger} must rise within 0..=1"
    );
    (warn, danger)
}

fn checked(share: f32) -> f32 {
    assert!(
        (0.0..=1.0).contains(&share),
        "a share of {share} is not 0..=1"
    );
    share
}

/// Glides from the last share to `share` on a change, and stays still on first paint.
fn gliding<E, D>(
    id: ElementId,
    share: f32,
    window: &mut Window,
    cx: &mut App,
    draw: D,
) -> impl IntoElement + use<E, D>
where
    E: IntoElement + 'static,
    D: Fn(f32) -> E + 'static,
{
    let (from, turn) = motion::follow(&id, share, window, cx);
    div().size_full().with_animation(
        (id, format!("glide-{turn}")),
        Animation::new(motion::duration(motion::BASE, cx)).with_easing(motion::ease_out_cubic),
        move |frame, t| {
            let at = if turn == 0 {
                share
            } else {
                from + (share - from) * t
            };
            frame.child(draw(at))
        },
    )
}

/// How full something is, such as a disk or a quota: a label, how much, and a bar that turns amber, then red, as it fills.
#[derive(IntoElement)]
pub struct Meter {
    id: ElementId,
    label: SharedString,
    share: f32,
    detail: Option<SharedString>,
    thresholds: (f32, f32),
}

impl Meter {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>, share: f32) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            share: checked(share),
            detail: None,
            thresholds: (0.8, 0.95),
        }
    }

    /// The share in units, such as "38 GB of 256 GB"; by default, the percent.
    pub fn detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Where the bar turns amber, then red.
    pub fn thresholds(mut self, warn: f32, danger: f32) -> Self {
        self.thresholds = thresholds(warn, danger);
        self
    }
}

impl RenderOnce for Meter {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let (track, lane, tones) = (theme.meter_track(), colors.border, tones(colors));
        let detail = self
            .detail
            .unwrap_or_else(|| format::percent(self.share as f64, 0, false).into());
        let head = div()
            .flex()
            .items_baseline()
            .justify_between()
            .gap_3()
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_color(colors.fg)
                    .child(Ellipsis::new(self.label)),
            )
            .child(
                tabular(div())
                    .debug_selector(|| "meter-detail".into())
                    .flex_none()
                    .text_color(colors.fg_muted)
                    .child(detail),
            );
        let limits = self.thresholds;
        let bar = gliding(self.id, self.share, window, cx, move |at| {
            div().relative().h(track).rounded_full().bg(lane).child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .h_full()
                    .w(relative(at))
                    .rounded_full()
                    .bg(tones[Level::of(at, limits) as usize]),
            )
        });
        div()
            .flex()
            .flex_col()
            .gap_1p5()
            .child(head)
            .child(div().h(track).child(bar))
    }
}

/// Where a gauge's arc starts, at the lower left, and how far it sweeps, clockwise to the lower right.
const START: f32 = 3.0 * FRAC_PI_4;
const SWEEP: f32 = 0.75 * TAU;

/// A value on a dial: an arc of three quarters that fills clockwise from the lower left, the number rolling at the center, the label below.
#[derive(IntoElement)]
pub struct Gauge {
    id: ElementId,
    label: SharedString,
    value: f64,
    range: (f64, f64),
    decimals: usize,
    suffix: Option<SharedString>,
    thresholds: (f32, f32),
}

impl Gauge {
    /// A `value` past `min..=max` pegs the dial and shows as it is.
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        value: f64,
        min: f64,
        max: f64,
    ) -> Self {
        assert!(
            min < max,
            "a gauge needs min below max, got {min} and {max}"
        );
        Self {
            id: id.into(),
            label: label.into(),
            value,
            range: (min, max),
            decimals: 0,
            suffix: None,
            thresholds: (0.8, 0.95),
        }
    }

    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    /// A unit after the number, such as "°C".
    pub fn suffix(mut self, suffix: impl Into<SharedString>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// Where the arc turns amber, then red, as shares of the range.
    pub fn thresholds(mut self, warn: f32, danger: f32) -> Self {
        self.thresholds = thresholds(warn, danger);
        self
    }
}

impl RenderOnce for Gauge {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (min, max) = self.range;
        let share = ((self.value - min) / (max - min)).clamp(0.0, 1.0) as f32;
        let theme = cx.theme();
        let (limits, track, tones) = (self.thresholds, theme.colors.border, tones(&theme.colors));
        let dial = gliding(self.id.clone(), share, window, cx, move |at| {
            let color = tones[Level::of(at, limits) as usize];
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    let side = bounds.size.width.min(bounds.size.height);
                    let width = side / 16.0;
                    let (center, radius) = (bounds.center(), side / 2.0 - width);
                    motion::arc(center, radius, START, SWEEP, width, track, window);
                    if at > 0.0 {
                        motion::arc(center, radius, START, SWEEP * at, width, color, window);
                    }
                },
            )
            .size_full()
        });
        let theme = cx.theme();
        let unit = |text: SharedString| {
            div()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(theme.colors.fg_muted)
                .child(text)
        };
        div()
            .relative()
            .flex_none()
            .size(theme.gauge())
            .child(div().absolute().inset_0().child(dial))
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap_0p5()
                            .child(
                                AnimatedNumber::new((self.id, "number"), self.value)
                                    .decimals(self.decimals)
                                    .size(TextSize::Xl),
                            )
                            .children(self.suffix.map(unit)),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .bottom_3()
                    .left_0()
                    .right_0()
                    .flex()
                    .justify_center()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(theme.colors.fg_muted)
                    .child(self.label),
            )
    }
}

type Amounts = Rc<dyn Fn(f64) -> String>;

/// How a whole divides: one bar of colored parts with the free rest as track, and a legend of each part's share or amount.
#[derive(IntoElement)]
pub struct UsageBar {
    total: f64,
    parts: Vec<(SharedString, f64)>,
    amounts: Option<Amounts>,
}

impl UsageBar {
    /// Parts of `total`; what they leave shows as free track.
    pub fn new(total: f64) -> Self {
        assert!(
            total > 0.0,
            "a usage bar needs a positive total, got {total}"
        );
        Self {
            total,
            parts: Vec::new(),
            amounts: None,
        }
    }

    /// Shows each part's amount, as `write` puts it, in place of its share.
    pub fn amounts(mut self, write: impl Fn(f64) -> String + 'static) -> Self {
        self.amounts = Some(Rc::new(write));
        self
    }

    pub fn part(mut self, name: impl Into<SharedString>, amount: f64) -> Self {
        assert!(amount >= 0.0, "a part cannot be negative, got {amount}");
        self.parts.push((name.into(), amount));
        self
    }
}

impl UsageBar {
    /// What the parts use and whether they fill the total, within float error; over it they fill it.
    fn used(&self) -> (f64, bool) {
        let used: f64 = self.parts.iter().map(|(_, amount)| amount).sum();
        (used, used >= self.total * (1.0 - 1e-9))
    }

    /// A part's figure in the legend: its amount as `amounts` writes it, or its share.
    fn figure(&self, amount: f64) -> String {
        match &self.amounts {
            Some(write) => write(amount),
            None => format::percent(amount / self.total, 0, false),
        }
    }
}

impl RenderOnce for UsageBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let (used, full) = self.used();
        let hue = |ix: usize| colors.chart[ix % colors.chart.len()];
        let total = self.total;
        let grown = |share: f64| {
            let mut part = div().h_full().flex_basis(relative(0.0));
            part.style().flex_grow = Some(share as f32);
            part
        };
        let last = self.parts.len().saturating_sub(1);
        let bar = div()
            .flex()
            .gap_0p5()
            .h(theme.meter_track())
            .children(self.parts.iter().enumerate().map(|(ix, (_, amount))| {
                grown(amount / total)
                    .bg(hue(ix))
                    .when(ix == 0, |part| part.rounded_l_full())
                    .when(ix == last && full, |part| part.rounded_r_full())
            }))
            .when(!full, |bar| {
                bar.child(
                    grown((total - used) / total)
                        .bg(colors.border)
                        .rounded_r_full()
                        .when(self.parts.is_empty(), |free| free.rounded_l_full()),
                )
            });
        let legend = self.parts.iter().enumerate().map(|(ix, (name, amount))| {
            div()
                .flex()
                .items_center()
                .gap_1p5()
                .child(div().size(theme.status_dot()).rounded_full().bg(hue(ix)))
                .child(div().text_color(colors.fg).child(name.clone()))
                .child(
                    tabular(div())
                        .text_color(colors.fg_muted)
                        .child(self.figure(*amount)),
                )
        });
        div().flex().flex_col().gap_2().child(bar).child(
            div()
                .flex()
                .flex_wrap()
                .gap_x_4()
                .gap_y_1()
                .text_size(theme.text_size(TextSize::Sm))
                .font_weight(FontWeight::NORMAL)
                .children(legend),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Level, UsageBar, tone};
    use crate::theme::Palette;

    #[test]
    fn a_tone_follows_the_share_past_each_limit() {
        let colors = Palette::light(false);
        let limits = (0.8, 0.95);
        assert_eq!(tone(0.5, limits, &colors), colors.fg);
        assert_eq!(tone(0.85, limits, &colors), colors.warning);
        assert_eq!(tone(0.97, limits, &colors), colors.danger);
    }

    #[test]
    fn parts_past_the_total_fill_it() {
        let over = UsageBar::new(10.0).part("a", 8.0).part("b", 5.0);
        assert_eq!(over.used(), (13.0, true));
    }

    #[test]
    fn parts_that_sum_to_the_total_fill_it_despite_float_error() {
        assert!(UsageBar::new(0.3).part("a", 0.1).part("b", 0.2).used().1);
        assert!(!UsageBar::new(1.0).part("a", 0.5).used().1);
    }

    #[test]
    fn a_part_reads_as_its_share_or_as_written() {
        let bar = UsageBar::new(4.0).part("a", 1.0);
        assert_eq!(bar.figure(1.0), "25%");
        let bar = bar.amounts(|amount| format!("{amount} GB"));
        assert_eq!(bar.figure(1.0), "1 GB");
    }

    #[test]
    fn a_share_turns_amber_then_red() {
        let limits = (0.8, 0.95);
        assert_eq!(Level::of(0.5, limits), Level::Calm);
        assert_eq!(Level::of(0.8, limits), Level::Warn);
        assert_eq!(Level::of(0.94, limits), Level::Warn);
        assert_eq!(Level::of(0.95, limits), Level::Danger);
    }
}
