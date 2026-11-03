use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
};

use crate::{
    buttons::{Button, ButtonVariant, IconButton},
    data_display::{Badge, Tone},
    lists::{ListItem, SelectableList, row_action},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize},
};

/// A database a connection reaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Engine {
    Postgres,
    MySql,
    Sqlite,
    Redis,
    MongoDb,
}

impl Engine {
    pub const ALL: [Engine; 5] = [
        Engine::Postgres,
        Engine::MySql,
        Engine::Sqlite,
        Engine::Redis,
        Engine::MongoDb,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Engine::Postgres => "PostgreSQL",
            Engine::MySql => "MySQL",
            Engine::Sqlite => "SQLite",
            Engine::Redis => "Redis",
            Engine::MongoDb => "MongoDB",
        }
    }

    /// The port it listens on unless told otherwise; SQLite reads a file.
    pub fn port(self) -> Option<u16> {
        match self {
            Engine::Postgres => Some(5432),
            Engine::MySql => Some(3306),
            Engine::Sqlite => None,
            Engine::Redis => Some(6379),
            Engine::MongoDb => Some(27017),
        }
    }

    pub(super) fn named(name: &str) -> Engine {
        *Engine::ALL
            .iter()
            .find(|engine| engine.name() == name)
            .unwrap_or_else(|| panic!("no engine {name}"))
    }
}

/// Where a connection stands.
#[derive(Clone, Debug, PartialEq)]
pub enum ConnectionState {
    Connected,
    Idle,
    Failed(SharedString),
}

/// A saved connection: its key and name, the engine, where it reaches, the database, who signs in, and where it stands. For SQLite the database is the file.
#[derive(Clone, Debug, PartialEq)]
pub struct Connection {
    pub key: SharedString,
    pub name: SharedString,
    pub engine: Engine,
    pub host: SharedString,
    pub port: Option<u16>,
    pub database: SharedString,
    pub user: SharedString,
    pub password: SharedString,
    pub tls: bool,
    pub state: ConnectionState,
}

impl Connection {
    pub fn new(
        key: impl Into<SharedString>,
        name: impl Into<SharedString>,
        engine: Engine,
    ) -> Self {
        Self {
            key: key.into(),
            name: name.into(),
            engine,
            host: "localhost".into(),
            port: engine.port(),
            database: SharedString::default(),
            user: SharedString::default(),
            password: SharedString::default(),
            tls: false,
            state: ConnectionState::Idle,
        }
    }

    /// Where it reaches, as one line: host and port and any database, or the file.
    pub fn address(&self) -> String {
        match (self.port, self.database.is_empty()) {
            (Some(port), true) => format!("{}:{port}", self.host),
            (Some(port), false) => format!("{}:{port}/{}", self.host, self.database),
            (None, _) => self.database.to_string(),
        }
    }
}

type OnKey = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;
type Run = Rc<dyn Fn(&mut Window, &mut App)>;

/// Saved connections, each with its engine and address, or why it failed, and a badge for where it stands. Enter or a double press connects; the pencil edits and the bin removes; New asks for a blank one.
#[derive(IntoElement)]
pub struct ConnectionManager {
    id: ElementId,
    connections: Vec<Connection>,
    selected: Option<SharedString>,
    on_select: Option<OnKey>,
    on_open: Option<OnKey>,
    on_edit: Option<OnKey>,
    on_remove: Option<OnKey>,
    on_new: Option<Run>,
}

impl ConnectionManager {
    pub fn new(
        id: impl Into<ElementId>,
        connections: impl IntoIterator<Item = Connection>,
    ) -> Self {
        Self {
            id: id.into(),
            connections: connections.into_iter().collect(),
            selected: None,
            on_select: None,
            on_open: None,
            on_edit: None,
            on_remove: None,
            on_new: None,
        }
    }

    pub fn selected(mut self, key: Option<impl Into<SharedString>>) -> Self {
        self.selected = key.map(Into::into);
        self
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Gets the connection to connect, on Enter or a double press.
    pub fn on_open(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }

    pub fn on_edit(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_edit = Some(Rc::new(handler));
        self
    }

    pub fn on_remove(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_remove = Some(Rc::new(handler));
        self
    }

    pub fn on_new(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_new = Some(Rc::new(handler));
        self
    }
}

fn standing(state: &ConnectionState) -> (Tone, &'static str) {
    match state {
        ConnectionState::Connected => (Tone::Success, "Connected"),
        ConnectionState::Idle => (Tone::Neutral, "Idle"),
        ConnectionState::Failed(_) => (Tone::Danger, "Failed"),
    }
}

impl RenderOnce for ConnectionManager {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let theme = cx.theme();
        let muted = theme.colors.fg_muted;
        let list = self.connections.iter().fold(
            SelectableList::new((id.clone(), "list")).selected(self.selected.clone()),
            |list, connection| {
                let key = connection.key.clone();
                let (tone, words) = standing(&connection.state);
                let detail = match &connection.state {
                    ConnectionState::Failed(why) => why.to_string(),
                    _ => format!("{} · {}", connection.engine.name(), connection.address()),
                };
                let action = |icon: IconName, tip: &'static str, handler: &Option<OnKey>| {
                    let (key, handler) = (key.clone(), handler.clone());
                    row_action(
                        IconButton::new((id.clone(), format!("{tip}-{key}")), icon)
                            .tooltip(tip)
                            .on_click(move |_, window, cx| {
                                if let Some(handler) = &handler {
                                    handler(&key, window, cx);
                                }
                            }),
                    )
                };
                list.row(
                    key.clone(),
                    ListItem::new((id.clone(), format!("row-{key}")), connection.name.clone())
                        .description(detail)
                        .leading(
                            Icon::new(IconName::Database)
                                .size(IconSize::Md)
                                .color(muted),
                        )
                        .trailing(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(Badge::new(words).tone(tone).dot())
                                .child(action(IconName::Pencil, "Edit", &self.on_edit))
                                .child(action(IconName::Trash2, "Remove", &self.on_remove)),
                        ),
                )
            },
        );
        let (on_select, on_open, on_new) = (self.on_select, self.on_open, self.on_new);
        let list = list
            .on_change(move |keys, window, cx| {
                if let (Some(key), Some(on_select)) = (keys.first(), &on_select) {
                    on_select(key, window, cx);
                }
            })
            .on_activate(move |key, window, cx| {
                log::info!("connection manager: open {key}");
                if let Some(on_open) = &on_open {
                    on_open(key, window, cx);
                }
            });
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Connections"),
                    )
                    .child(
                        Button::new((id, "new"), "New")
                            .variant(ButtonVariant::Secondary)
                            .on_click(move |_, window, cx| {
                                if let Some(on_new) = &on_new {
                                    on_new(window, cx);
                                }
                            }),
                    ),
            )
            .child(list)
    }
}
