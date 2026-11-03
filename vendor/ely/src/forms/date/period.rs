use std::rc::Rc;

use gpui::{
    App, Div, ElementId, FocusHandle, FontWeight, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, Stateful, StatefulInteractiveElement, Styled, Window, div,
    prelude::*,
};
use jiff::civil::Date;

use super::{
    super::options::{Run, float, surface},
    picker::{Face, dropdown, picker_field},
    zoned_now,
};
use crate::{
    buttons::{ButtonVariant, IconButton},
    layout::seeded::use_seeded,
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
};

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// What a period grid shows: a page title, cells, and which one is chosen and which is now.
struct Page {
    title: String,
    cells: Vec<String>,
    chosen: Option<usize>,
    now: Option<usize>,
    columns: u16,
}

type Turn = Rc<dyn Fn(i16, &mut Window, &mut App)>;
type PickPeriod = Rc<dyn Fn(i16, usize, &mut Window, &mut App)>;
type OnYear = Rc<dyn Fn(i16, &mut Window, &mut App)>;
type PickCell = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// What a period grid does: turn the page, pick a cell, close, and the focus it keeps.
struct Grid {
    turn: Turn,
    pick: PickCell,
    close: Run,
    focus: FocusHandle,
}

/// A page of periods under a header with arrows. Arrow keys move, Page keys turn, Enter picks.
fn period_grid(id: &ElementId, page: Page, grid: Grid, window: &mut Window, cx: &mut App) -> Div {
    let Grid {
        turn,
        pick,
        close,
        focus,
    } = grid;
    let focus = &focus;
    let count = page.cells.len();
    let cursor = use_seeded((id.clone(), "cursor"), page.chosen.unwrap_or(0), window, cx);
    let at = cursor.read(cx).value.min(count - 1);
    let focused = focus.is_focused(window);
    let theme = cx.theme();
    let colors = &theme.colors;
    let arrow = |key: &'static str, icon: IconName, by: i16| {
        let turn = turn.clone();
        IconButton::new((id.clone(), key), icon)
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Sm)
            .on_click(move |_, window, cx| turn(by, window, cx))
    };
    let cells: Vec<_> = page
        .cells
        .into_iter()
        .enumerate()
        .map(|(ix, label)| {
            let pick = pick.clone();
            let chosen = page.chosen == Some(ix);
            div()
                .id(("period", ix))
                .flex()
                .items_center()
                .justify_center()
                .h(theme.control_height(ControlSize::Md))
                .rounded(theme.radius(Radius::Md))
                .border_1()
                .border_color(if focused && ix == at {
                    colors.focus
                } else if page.now == Some(ix) && !chosen {
                    colors.border_strong
                } else {
                    gpui::transparent_black()
                })
                .cursor_pointer()
                .map(|cell| {
                    if chosen {
                        cell.bg(colors.accent).text_color(colors.on_accent)
                    } else {
                        cell.text_color(colors.fg)
                            .hover(|style| style.bg(colors.hover))
                    }
                })
                .on_click(move |_, window, cx| pick(ix, window, cx))
                .child(label)
        })
        .collect();
    let columns = page.columns;
    let keys = (cursor.clone(), pick.clone(), turn.clone());
    div()
        .track_focus(focus)
        .flex()
        .flex_col()
        .gap_2()
        .w(theme.control_height(ControlSize::Md) * 8.0)
        .p_2()
        .text_size(theme.text_size(TextSize::Sm))
        .on_key_down(move |event, window, cx| {
            let (cursor, pick, turn) = &keys;
            let by = i16::try_from(columns).expect("a few columns");
            let step = match event.keystroke.key.as_str() {
                "left" => -1,
                "right" => 1,
                "up" => -by,
                "down" => by,
                "pageup" => return turn(-1, window, cx),
                "pagedown" => return turn(1, window, cx),
                "enter" | "space" => {
                    cx.stop_propagation();
                    return pick(at, window, cx);
                }
                "escape" => {
                    cx.stop_propagation();
                    return close(window, cx);
                }
                _ => return,
            };
            cx.stop_propagation();
            let next = (at as i16 + step).clamp(0, count as i16 - 1) as usize;
            cursor.update(cx, |cursor, cx| {
                cursor.value = next;
                cx.notify();
            });
        })
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .pl_1()
                        .text_size(theme.text_size(TextSize::Base))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(colors.fg)
                        .child(page.title),
                )
                .child(
                    div()
                        .flex()
                        .gap_0p5()
                        .child(arrow("prev", IconName::ChevronLeft, -1))
                        .child(arrow("next", IconName::ChevronRight, 1)),
                ),
        )
        .child(div().grid().grid_cols(columns).gap_1().children(cells))
}

/// A period picker's words, first page, page size, and what its cells mean.
struct Periods {
    shown: Option<SharedString>,
    placeholder: &'static str,
    seed: i16,
    step: i16,
    build: Rc<dyn Fn(i16) -> Page>,
    pick: PickPeriod,
}

/// The field and popup shared by month, quarter and year pickers.
fn period_picker(
    id: ElementId,
    periods: Periods,
    window: &mut Window,
    cx: &mut App,
) -> Stateful<Div> {
    let Periods {
        shown,
        placeholder,
        seed,
        step,
        build,
        pick,
    } = periods;
    let state = dropdown(&id, window, cx);
    let page = use_seeded((id.clone(), "page"), seed, window, cx);
    let inner = state.read(cx).inner.clone();
    let key = id.clone();
    picker_field(
        id,
        Face {
            icon: IconName::Calendar,
            shown,
            placeholder: placeholder.into(),
            size: ControlSize::default(),
            disabled: false,
        },
        move |close, anchor, window, cx| {
            let shown = page.read(cx).value;
            let turn: Turn = {
                let page = page.clone();
                Rc::new(move |by, _, cx| {
                    page.update(cx, |page, cx| {
                        page.value += by * step;
                        cx.notify();
                    })
                })
            };
            let chosen: PickCell = {
                let (pick, close) = (pick.clone(), close.clone());
                Rc::new(move |ix, window, cx| {
                    pick(shown, ix, window, cx);
                    close(window, cx);
                })
            };
            let out = close.clone();
            let grid = Grid {
                turn,
                pick: chosen,
                close,
                focus: inner.clone(),
            };
            let grid = period_grid(&key, build(shown), grid, window, cx);
            float(
                key.clone(),
                anchor,
                6,
                surface((key.clone(), "popup"), cx)
                    .on_mouse_down_out(move |_, window, cx| out(window, cx))
                    .child(grid),
                window,
                cx,
            )
        },
        window,
        cx,
    )
}

fn today(fixed: Option<Date>) -> Date {
    fixed.unwrap_or_else(|| zoned_now().date())
}

type OnPeriod = Rc<dyn Fn((i16, i8), &mut Window, &mut App)>;

/// A year's months as a grid. The value is a year and a month, 1 to 12.
#[derive(IntoElement)]
pub struct MonthPicker {
    id: ElementId,
    value: Option<(i16, i8)>,
    today: Option<Date>,
    on_change: Option<OnPeriod>,
}

impl MonthPicker {
    pub fn new(id: impl Into<ElementId>, value: Option<(i16, i8)>) -> Self {
        Self {
            id: id.into(),
            value,
            today: None,
            on_change: None,
        }
    }

    /// The date treated as now. Defaults to the system's date.
    pub fn today(mut self, date: Date) -> Self {
        self.today = Some(date);
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn((i16, i8), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for MonthPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (value, now) = (self.value, today(self.today));
        let on_change = self.on_change;
        let periods = Periods {
            shown: value
                .map(|(year, month)| format!("{} {year}", MONTH_NAMES[month as usize - 1]).into()),
            placeholder: "Pick a month",
            seed: value.map_or(now.year(), |(year, _)| year),
            step: 1,
            build: Rc::new(move |year| Page {
                title: year.to_string(),
                cells: MONTHS.iter().map(|month| month.to_string()).collect(),
                chosen: value
                    .filter(|(chosen, _)| *chosen == year)
                    .map(|(_, month)| month as usize - 1),
                now: (now.year() == year).then(|| now.month() as usize - 1),
                columns: 3,
            }),
            pick: Rc::new(move |year, ix, window, cx| {
                log::info!("month picker: {year}-{:02}", ix + 1);
                if let Some(on_change) = &on_change {
                    on_change((year, ix as i8 + 1), window, cx);
                }
            }),
        };
        period_picker(self.id, periods, window, cx)
    }
}

const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// A year's quarters. The value is a year and a quarter, 1 to 4.
#[derive(IntoElement)]
pub struct QuarterPicker {
    id: ElementId,
    value: Option<(i16, i8)>,
    today: Option<Date>,
    on_change: Option<OnPeriod>,
}

impl QuarterPicker {
    pub fn new(id: impl Into<ElementId>, value: Option<(i16, i8)>) -> Self {
        Self {
            id: id.into(),
            value,
            today: None,
            on_change: None,
        }
    }

    /// The date treated as now. Defaults to the system's date.
    pub fn today(mut self, date: Date) -> Self {
        self.today = Some(date);
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn((i16, i8), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for QuarterPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (value, now) = (self.value, today(self.today));
        let on_change = self.on_change;
        let quarter = (now.month() as usize - 1) / 3;
        let periods = Periods {
            shown: value.map(|(year, quarter)| format!("Q{quarter} {year}").into()),
            placeholder: "Pick a quarter",
            seed: value.map_or(now.year(), |(year, _)| year),
            step: 1,
            build: Rc::new(move |year| Page {
                title: year.to_string(),
                cells: (1..=4).map(|quarter| format!("Q{quarter}")).collect(),
                chosen: value
                    .filter(|(chosen, _)| *chosen == year)
                    .map(|(_, quarter)| quarter as usize - 1),
                now: (now.year() == year).then_some(quarter),
                columns: 4,
            }),
            pick: Rc::new(move |year, ix, window, cx| {
                log::info!("quarter picker: {year} Q{}", ix + 1);
                if let Some(on_change) = &on_change {
                    on_change((year, ix as i8 + 1), window, cx);
                }
            }),
        };
        period_picker(self.id, periods, window, cx)
    }
}

/// Twelve years to a page. The value is a year.
#[derive(IntoElement)]
pub struct YearPicker {
    id: ElementId,
    value: Option<i16>,
    today: Option<Date>,
    on_change: Option<OnYear>,
}

impl YearPicker {
    pub fn new(id: impl Into<ElementId>, value: Option<i16>) -> Self {
        Self {
            id: id.into(),
            value,
            today: None,
            on_change: None,
        }
    }

    pub fn today(mut self, date: Date) -> Self {
        self.today = Some(date);
        self
    }

    pub fn on_change(mut self, handler: impl Fn(i16, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

/// The first year of the twelve-year page holding `year`.
pub(crate) fn page_of(year: i16) -> i16 {
    year - year.rem_euclid(12)
}

impl RenderOnce for YearPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (value, now) = (self.value, today(self.today).year());
        let on_change = self.on_change;
        let periods = Periods {
            shown: value.map(|year| year.to_string().into()),
            placeholder: "Pick a year",
            seed: page_of(value.unwrap_or(now)),
            step: 12,
            build: Rc::new(move |first| Page {
                title: format!("{first} \u{2013} {}", first + 11),
                cells: (first..first + 12).map(|year| year.to_string()).collect(),
                chosen: value
                    .and_then(|year| (first..first + 12).position(|candidate| candidate == year)),
                now: (first..first + 12).position(|candidate| candidate == now),
                columns: 3,
            }),
            pick: Rc::new(move |first, ix, window, cx| {
                let year = first + ix as i16;
                log::info!("year picker: {year}");
                if let Some(on_change) = &on_change {
                    on_change(year, window, cx);
                }
            }),
        };
        period_picker(self.id, periods, window, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::page_of;

    #[test]
    fn twelve_year_pages_start_on_a_multiple_of_twelve() {
        assert_eq!(page_of(2026), 2016);
        assert_eq!(page_of(2028), 2028);
        assert_eq!(page_of(-5), -12);
    }
}
