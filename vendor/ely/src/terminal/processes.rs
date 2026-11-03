use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
};

use crate::{
    buttons::{Button, ButtonVariant},
    tables::{Cell, Column, DataTable, Row, SortKey},
    theme::{ActiveTheme, ControlSize, TextSize},
    typography::format,
};

/// A running process: its id, name and owner, and what it takes.
#[derive(Clone, Debug, PartialEq)]
pub struct Process {
    pub pid: u32,
    pub name: SharedString,
    pub user: SharedString,
    /// Percent of one core.
    pub cpu: f32,
    pub memory: u64,
}

type OnPids = Rc<dyn Fn(Vec<u32>, &mut Window, &mut App)>;

/// Running processes as a table, busiest first: found by a query, sorted by any column, and the selected ones ended.
#[derive(IntoElement)]
pub struct ProcessList {
    id: ElementId,
    processes: Vec<Process>,
    query: SharedString,
    selected: Vec<u32>,
    on_select: Option<OnPids>,
    on_end: Option<OnPids>,
}

impl ProcessList {
    pub fn new(id: impl Into<ElementId>, processes: impl IntoIterator<Item = Process>) -> Self {
        Self {
            id: id.into(),
            processes: processes.into_iter().collect(),
            query: SharedString::default(),
            selected: Vec::new(),
            on_select: None,
            on_end: None,
        }
    }

    pub fn query(mut self, query: impl Into<SharedString>) -> Self {
        self.query = query.into();
        self
    }

    pub fn selected(mut self, pids: impl IntoIterator<Item = u32>) -> Self {
        self.selected = pids.into_iter().collect();
        self
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(Vec<u32>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Called with the selected processes when End is pressed.
    pub fn on_end(mut self, handler: impl Fn(Vec<u32>, &mut Window, &mut App) + 'static) -> Self {
        self.on_end = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ProcessList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let busy: f32 = self.processes.iter().map(|process| process.cpu).sum();
        let columns = [
            Column::new("name", "Name"),
            Column::new("pid", "PID").end(),
            Column::new("user", "User"),
            Column::new("cpu", "CPU").end().decimals(1).suffix(" %"),
            Column::new("memory", "Memory")
                .end()
                .decimals(1)
                .suffix(" MB"),
        ];
        let rows: Vec<Row> = self
            .processes
            .iter()
            .map(|process| {
                Row::new(
                    process.pid.to_string(),
                    [
                        Cell::Text(process.name.clone()),
                        Cell::Text(process.pid.to_string().into()),
                        Cell::Text(process.user.clone()),
                        Cell::Number(f64::from(process.cpu)),
                        Cell::Number(process.memory as f64 / (1024.0 * 1024.0)),
                    ],
                )
            })
            .collect();
        let pid =
            |key: &SharedString| -> u32 { key.parse().expect("a process row is keyed by its pid") };
        let table = DataTable::new((self.id.clone(), "table"), columns)
            .rows(rows)
            .query(self.query)
            .sorts([SortKey {
                column: "cpu".into(),
                rising: false,
            }])
            .selected(self.selected.iter().map(|pid| pid.to_string()));
        let table = match self.on_select {
            Some(on_select) => table.on_select(move |keys, window, cx| {
                on_select(keys.iter().map(pid).collect(), window, cx)
            }),
            None => table,
        };
        let chosen = self.selected.clone();
        div()
            .flex()
            .flex_col()
            .gap_2()
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(div().flex_1().text_color(colors.fg_muted).child(format!(
                        "{} · {:.1} % CPU",
                        format::plural(self.processes.len() as u64, "process", "processes"),
                        busy
                    )))
                    .children(self.on_end.map(|on_end| {
                        Button::new((self.id.clone(), "end"), "End process")
                            .variant(ButtonVariant::Danger)
                            .size(ControlSize::Sm)
                            .disabled(chosen.is_empty())
                            .on_click(move |_, window, cx| {
                                log::info!("process list: end {chosen:?}");
                                on_end(chosen.clone(), window, cx)
                            })
                    })),
            )
            .child(table)
    }
}
