use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div,
};

use super::market::ChartKind;
use crate::{
    buttons::SegmentedControl,
    forms::{Choice, Select},
    primitives::IconName,
    theme::{ActiveTheme, ControlSize},
};

type OnPick = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// The spans of history a market chart offers, shortest first.
pub const RANGES: [&str; 9] = ["1D", "5D", "1M", "3M", "6M", "YTD", "1Y", "5Y", "All"];

/// How long a candle may last, each with its words.
pub const INTERVALS: [(&str, &str); 8] = [
    ("1m", "1 minute"),
    ("5m", "5 minutes"),
    ("15m", "15 minutes"),
    ("1h", "1 hour"),
    ("4h", "4 hours"),
    ("1D", "1 day"),
    ("1W", "1 week"),
    ("1M", "1 month"),
];

/// A span of history to show, from a day to all of it.
#[derive(IntoElement)]
pub struct TimeRangeSelector {
    id: ElementId,
    selected: SharedString,
    on_change: Option<OnPick>,
}

impl TimeRangeSelector {
    /// `selected` is one of `RANGES`.
    pub fn new(id: impl Into<ElementId>, selected: impl Into<SharedString>) -> Self {
        let selected = selected.into();
        if !RANGES.contains(&selected.as_ref()) {
            log::error!("time range selector: {selected} is not a range; none marked");
        }
        Self {
            id: id.into(),
            selected,
            on_change: None,
        }
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for TimeRangeSelector {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let control = RANGES.iter().fold(
            SegmentedControl::new(self.id, self.selected).size(ControlSize::Sm),
            |control, range| control.segment(*range, *range, None),
        );
        match self.on_change {
            Some(on_change) => {
                control.on_change(move |range, window, cx| on_change(range, window, cx))
            }
            None => control,
        }
    }
}

/// How long each candle lasts, from a minute to a month.
#[derive(IntoElement)]
pub struct IntervalSelector {
    id: ElementId,
    selected: SharedString,
    on_change: Option<OnPick>,
}

impl IntervalSelector {
    /// `selected` is one of the keys of `INTERVALS`.
    pub fn new(id: impl Into<ElementId>, selected: impl Into<SharedString>) -> Self {
        let selected = selected.into();
        if !INTERVALS.iter().any(|(key, _)| *key == selected.as_ref()) {
            log::error!("interval selector: {selected} is not an interval; none chosen");
        }
        Self {
            id: id.into(),
            selected,
            on_change: None,
        }
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for IntervalSelector {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let choices = INTERVALS
            .iter()
            .map(|(key, words)| Choice::new(*key, *words));
        let select = Select::new(self.id, choices)
            .selected(self.selected)
            .size(ControlSize::Sm);
        let select = match self.on_change {
            Some(on_change) => select.on_change(move |key, window, cx| on_change(key, window, cx)),
            None => select,
        };
        div().w(cx.theme().label_width() * 0.75).child(select)
    }
}

/// Each chart kind with its key, words and icon.
const KINDS: [(ChartKind, &str, &str, IconName); 5] = [
    (
        ChartKind::Candles,
        "candles",
        "Candles",
        IconName::ChartCandlestick,
    ),
    (ChartKind::Bars, "bars", "Bars", IconName::ChartColumn),
    (
        ChartKind::HeikinAshi,
        "heikin-ashi",
        "Heikin-Ashi",
        IconName::ChartCandlestick,
    ),
    (ChartKind::Line, "line", "Line", IconName::ChartLine),
    (ChartKind::Area, "area", "Area", IconName::ChartArea),
];

type OnKind = Rc<dyn Fn(ChartKind, &mut Window, &mut App)>;

/// How a market chart draws its prices, chosen from candles, bars, Heikin-Ashi, a line or an area.
#[derive(IntoElement)]
pub struct ChartTypeSwitcher {
    id: ElementId,
    kind: ChartKind,
    on_change: Option<OnKind>,
}

impl ChartTypeSwitcher {
    pub fn new(id: impl Into<ElementId>, kind: ChartKind) -> Self {
        Self {
            id: id.into(),
            kind,
            on_change: None,
        }
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(ChartKind, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ChartTypeSwitcher {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let choices = KINDS
            .iter()
            .map(|(_, key, words, icon)| Choice::new(*key, *words).icon(*icon));
        let selected = KINDS
            .iter()
            .find(|(kind, ..)| *kind == self.kind)
            .map(|(_, key, ..)| *key)
            .expect("every kind is listed");
        let select = Select::new(self.id, choices)
            .selected(selected)
            .size(ControlSize::Sm);
        let select = match self.on_change {
            Some(on_change) => select.on_change(move |key, window, cx| {
                let kind = KINDS
                    .iter()
                    .find(|(_, listed, ..)| *listed == key.as_ref())
                    .map(|(kind, ..)| *kind)
                    .expect("a listed kind");
                on_change(kind, window, cx)
            }),
            None => select,
        };
        div().w(cx.theme().label_width() * 0.85).child(select)
    }
}

/// How charts share a space: one, two across, two down, or four.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Arrangement {
    #[default]
    One,
    Across,
    Down,
    Four,
}

impl Arrangement {
    const ALL: [(Arrangement, &'static str, IconName); 4] = [
        (Arrangement::One, "one", IconName::Square),
        (Arrangement::Across, "across", IconName::Columns2),
        (Arrangement::Down, "down", IconName::Rows2),
        (Arrangement::Four, "four", IconName::LayoutGrid),
    ];

    /// How many charts it holds.
    pub fn cells(self) -> usize {
        match self {
            Arrangement::One => 1,
            Arrangement::Across | Arrangement::Down => 2,
            Arrangement::Four => 4,
        }
    }
}

type OnArrange = Rc<dyn Fn(Arrangement, &mut Window, &mut App)>;

/// Charts arranged one, two across, two down or four to a grid, split by hairlines, with the four arrangements to switch between above them.
#[derive(IntoElement)]
pub struct MultiChartLayout {
    id: ElementId,
    arrangement: Arrangement,
    cells: Vec<AnyElement>,
    on_arrange: Option<OnArrange>,
}

impl MultiChartLayout {
    pub fn new(id: impl Into<ElementId>, arrangement: Arrangement) -> Self {
        Self {
            id: id.into(),
            arrangement,
            cells: Vec::new(),
            on_arrange: None,
        }
    }

    /// A chart for the next cell; the arrangement shows as many as it holds.
    pub fn cell(mut self, cell: impl IntoElement) -> Self {
        self.cells.push(cell.into_any_element());
        self
    }

    pub fn on_arrange(
        mut self,
        handler: impl Fn(Arrangement, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_arrange = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for MultiChartLayout {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let wanted = self.arrangement.cells();
        if self.cells.len() < wanted {
            log::error!(
                "multi-chart layout: {:?} holds {wanted} charts; {} given, the rest empty",
                self.arrangement,
                self.cells.len()
            );
        }
        let theme = cx.theme();
        let key = Arrangement::ALL
            .iter()
            .find(|(arrangement, ..)| *arrangement == self.arrangement)
            .map(|(_, key, _)| *key)
            .expect("listed");
        let switcher = Arrangement::ALL.iter().fold(
            SegmentedControl::new((self.id.clone(), "arrange"), key).size(ControlSize::Sm),
            |control, (_, key, icon)| control.segment(*key, "", Some(*icon)),
        );
        let switcher = match self.on_arrange {
            Some(on_arrange) => switcher.on_change(move |key, window, cx| {
                let arrangement = Arrangement::ALL
                    .iter()
                    .find(|(_, listed, _)| *listed == key.as_ref())
                    .map(|(arrangement, ..)| *arrangement)
                    .expect("listed");
                log::info!("multi-chart layout: {arrangement:?}");
                on_arrange(arrangement, window, cx)
            }),
            None => switcher,
        };
        let border = theme.colors.border;
        let cell = |child: AnyElement| div().flex_1().min_w_0().min_h_0().p_2().child(child);
        let mut cells = self.cells.into_iter().take(wanted).map(cell);
        let mut next = || {
            cells
                .next()
                .unwrap_or_else(|| cell(div().into_any_element()))
        };
        let grid = match self.arrangement {
            Arrangement::One => div().flex().child(next()),
            Arrangement::Across => div()
                .flex()
                .child(next().border_r_1().border_color(border))
                .child(next()),
            Arrangement::Down => div()
                .flex()
                .flex_col()
                .child(next().border_b_1().border_color(border))
                .child(next()),
            Arrangement::Four => div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .border_b_1()
                        .border_color(border)
                        .child(next().border_r_1().border_color(border))
                        .child(next()),
                )
                .child(
                    div()
                        .flex()
                        .child(next().border_r_1().border_color(border))
                        .child(next()),
                ),
        };
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(div().flex().justify_end().child(switcher))
            .child(
                grid.rounded(theme.radius(crate::theme::Radius::Md))
                    .border_1()
                    .border_color(border),
            )
    }
}
