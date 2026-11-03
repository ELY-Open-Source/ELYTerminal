use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div,
};
use jiff::civil::Date;

use super::{
    board::{KanbanBoard, KanbanCard, KanbanColumn},
    card::IssueCard,
    roadmap::{Initiative, Roadmap},
    tasks::clock_today,
    work::{Status, Task},
};
use crate::{
    buttons::SegmentedControl,
    calendar::{CalendarMonthView, Event, within},
    data_display::Tone,
    forms::OnValue,
    layout::seeded::use_seeded,
    primitives::IconName,
    tables::{Cell, Column, DataTable, Row},
    theme::{ActiveTheme, ControlSize, TextSize},
    typography::fresh,
};

type OnStatus = Rc<dyn Fn(&SharedString, Status, &mut Window, &mut App)>;

/// How a database shows its records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatabaseLayout {
    Table,
    Board,
    Gallery,
    Calendar,
    Timeline,
}

impl DatabaseLayout {
    pub const ALL: [DatabaseLayout; 5] = [
        DatabaseLayout::Table,
        DatabaseLayout::Board,
        DatabaseLayout::Gallery,
        DatabaseLayout::Calendar,
        DatabaseLayout::Timeline,
    ];

    pub fn words(self) -> &'static str {
        match self {
            DatabaseLayout::Table => "Table",
            DatabaseLayout::Board => "Board",
            DatabaseLayout::Gallery => "Gallery",
            DatabaseLayout::Calendar => "Calendar",
            DatabaseLayout::Timeline => "Timeline",
        }
    }

    fn icon(self) -> IconName {
        match self {
            DatabaseLayout::Table => IconName::Table,
            DatabaseLayout::Board => IconName::Kanban,
            DatabaseLayout::Gallery => IconName::LayoutGrid,
            DatabaseLayout::Calendar => IconName::Calendar,
            DatabaseLayout::Timeline => IconName::ChartGantt,
        }
    }

    fn of_words(words: &str) -> DatabaseLayout {
        DatabaseLayout::ALL
            .into_iter()
            .find(|layout| layout.words() == words)
            .unwrap_or_else(|| panic!("no database layout {words}"))
    }
}

/// A status's tone in a table's tag.
fn tone(status: Status) -> Tone {
    match status {
        Status::Backlog | Status::Todo => Tone::Neutral,
        Status::InProgress | Status::InReview => Tone::Info,
        Status::Done => Tone::Success,
        Status::Canceled => Tone::Neutral,
    }
}

/// Records five ways: a table, a board by status, a gallery of cards, a month calendar by due day, and a timeline from start to due. A switch picks the way; a record opens from the board, the gallery and the calendar, and a card moved to another column takes its status.
#[derive(IntoElement)]
pub struct DatabaseView {
    id: ElementId,
    tasks: Vec<Task>,
    layout: DatabaseLayout,
    today: Option<Date>,
    on_open: Option<OnValue>,
    on_status: Option<OnStatus>,
}

impl DatabaseView {
    pub fn new(id: impl Into<ElementId>, tasks: impl IntoIterator<Item = Task>) -> Self {
        Self {
            id: id.into(),
            tasks: tasks.into_iter().collect(),
            layout: DatabaseLayout::Table,
            today: None,
            on_open: None,
            on_status: None,
        }
    }

    /// The way it shows first; a new one from the owner shows at once.
    pub fn layout(mut self, layout: DatabaseLayout) -> Self {
        self.layout = layout;
        self
    }

    /// The day due dates count from; the clock's otherwise.
    pub fn today(mut self, day: Date) -> Self {
        self.today = Some(day);
        self
    }

    /// Gets the key of the record opened.
    pub fn on_open(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }

    /// Gets a record's key and the status of the column it was moved to.
    pub fn on_status(
        mut self,
        handler: impl Fn(&SharedString, Status, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_status = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for DatabaseView {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let today = match self.today {
            Some(day) => day,
            None => {
                fresh((id.clone(), "clock"), window, cx);
                clock_today("database view")
            }
        };
        let layout = use_seeded((id.clone(), "layout"), self.layout, window, cx);
        let shown = layout.read(cx).value;
        let switch = DatabaseLayout::ALL
            .into_iter()
            .fold(
                SegmentedControl::new((id.clone(), "layouts"), shown.words()),
                |switch, each| switch.segment(each.words(), each.words(), Some(each.icon())),
            )
            .size(ControlSize::Sm)
            .on_change({
                let (id, layout) = (id.clone(), layout.clone());
                move |words: &SharedString, _, cx| {
                    let next = DatabaseLayout::of_words(words);
                    log::info!("database view {id:?}: {}", next.words());
                    layout.update(cx, |layout, cx| {
                        layout.value = next;
                        cx.notify();
                    })
                }
            });
        let open = self.on_open.clone();
        let opener = move |key: SharedString| {
            let open = open.clone();
            move |window: &mut Window, cx: &mut App| {
                if let Some(open) = &open {
                    open(&key, window, cx);
                }
            }
        };
        let theme = cx.theme();
        let body: AnyElement = match shown {
            DatabaseLayout::Table => {
                let columns = [
                    Column::new("title", "Title").width(theme.project().column),
                    Column::new("status", "Status"),
                    Column::new("priority", "Priority"),
                    Column::new("assignee", "Assignee"),
                    Column::new("due", "Due"),
                ];
                let rows = self.tasks.iter().map(|task| {
                    let cells = [
                        Cell::Text(task.title.clone()),
                        Cell::Tag(task.status.words().into(), tone(task.status)),
                        Cell::Text(task.priority.words().into()),
                        task.assignee
                            .as_ref()
                            .map_or(Cell::Empty, |person| Cell::Person(person.name.clone())),
                        task.due.map_or(Cell::Empty, |due| {
                            Cell::Text(due.strftime("%b %-d").to_string().into())
                        }),
                    ];
                    Row::new(task.key.clone(), cells)
                });
                DataTable::new((id.clone(), "table"), columns)
                    .rows(rows.collect::<Vec<_>>())
                    .into_any_element()
            }
            DatabaseLayout::Board => {
                let on_status = self.on_status.clone();
                Status::ALL
                    .into_iter()
                    .fold(KanbanBoard::new((id.clone(), "board")), |board, status| {
                        let cards =
                            self.tasks
                                .iter()
                                .filter(|task| task.status == status)
                                .map(|task| {
                                    let face = IssueCard::new(
                                        (id.clone(), format!("card-{}", task.key)),
                                        task.clone(),
                                    )
                                    .today(today);
                                    KanbanCard::new(task.key.clone(), face)
                                });
                        board.column(KanbanColumn::new(status.key(), status.words()).cards(cards))
                    })
                    .on_move(move |key, column, _, window, cx| {
                        let status = Status::of_key(column);
                        if let Some(on_status) = &on_status {
                            on_status(key, status, window, cx);
                        }
                    })
                    .on_open({
                        let opener = opener.clone();
                        move |key, window, cx| opener(key.clone())(window, cx)
                    })
                    .into_any_element()
            }
            DatabaseLayout::Gallery => div()
                .flex()
                .flex_wrap()
                .gap_3()
                .children(self.tasks.iter().map(|task| {
                    div().w(theme.project().column).child(
                        IssueCard::new((id.clone(), format!("tile-{}", task.key)), task.clone())
                            .today(today)
                            .on_open(opener(task.key.clone())),
                    )
                }))
                .into_any_element(),
            DatabaseLayout::Calendar => {
                let shown = |task: &&Task| match task.due {
                    Some(due) if !within(due) => {
                        log::error!(
                            "database view: {} is due past the calendar's days",
                            task.key
                        );
                        false
                    }
                    due => due.is_some(),
                };
                let due: Vec<&Task> = self.tasks.iter().filter(shown).collect();
                let month = due
                    .iter()
                    .filter_map(|task| task.due)
                    .min()
                    .unwrap_or(today);
                let events = due.iter().filter_map(|task| {
                    task.due.map(|due| {
                        Event::all_day(task.key.clone(), task.title.clone(), due, due, 0)
                    })
                });
                let opener = opener.clone();
                CalendarMonthView::new((id.clone(), "calendar"), month, events.collect::<Vec<_>>())
                    .today(today)
                    .on_event(move |key, window, cx| opener(key.clone())(window, cx))
                    .into_any_element()
            }
            DatabaseLayout::Timeline => {
                let both: Vec<&Task> = self
                    .tasks
                    .iter()
                    .filter(|task| task.start.is_some() && task.due.is_some())
                    .collect();
                let dated: Vec<&Task> = both
                    .iter()
                    .copied()
                    .filter(|task| task.start <= task.due)
                    .collect();
                let backward = both.len() - dated.len();
                if backward > 0 {
                    log::error!(
                        "database view: {backward} tasks are due before they start; left out"
                    );
                }
                let first = dated
                    .iter()
                    .filter_map(|task| task.start)
                    .min()
                    .unwrap_or(today);
                let last = dated
                    .iter()
                    .filter_map(|task| task.due)
                    .max()
                    .unwrap_or(today);
                let months = ((i32::from(last.year()) - i32::from(first.year())) * 12
                    + i32::from(last.month() - first.month())
                    + 1)
                .clamp(1, 24) as i8;
                let lanes = Status::ALL.into_iter().filter_map(|status| {
                    let bars: Vec<Initiative> = dated
                        .iter()
                        .filter(|task| task.status == status)
                        .map(|task| {
                            let (start, due) =
                                (task.start.expect("dated"), task.due.expect("dated"));
                            let done = if status == Status::Done { 1.0 } else { 0.0 };
                            Initiative::new(task.key.clone(), task.title.clone(), start, due)
                                .progress(done)
                        })
                        .collect();
                    (!bars.is_empty()).then_some((status.words(), bars))
                });
                let undated = self.tasks.len() - both.len();
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(lanes.fold(
                        Roadmap::new((id.clone(), "timeline"), first, months).today(today),
                        |map, (name, bars)| map.lane(name, bars),
                    ))
                    .children((undated > 0).then(|| {
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(theme.colors.fg_muted)
                            .child(format!("{undated} without a start and a due day"))
                    }))
                    .children((backward > 0).then(|| {
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(theme.colors.fg_muted)
                            .child(format!("{backward} due before they start"))
                    }))
                    .into_any_element()
            }
        };
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(switch)
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(theme.colors.fg_muted)
                            .child(format!("{} records", self.tasks.len())),
                    ),
            )
            .child(body)
    }
}
