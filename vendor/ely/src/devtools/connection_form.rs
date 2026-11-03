use std::rc::Rc;

use gpui::{
    App, AppContext as _, ElementId, Entity, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div,
};

use super::connections::{Connection, Engine};
use crate::{
    buttons::{Button, ButtonVariant},
    feedback::InlineMessage,
    forms::{Choice, FormField, Input, PasswordInput, Select, Switch, TextInput},
    primitives::Severity,
};

type OnConnection = Rc<dyn Fn(Connection, &mut Window, &mut App)>;

/// What trying a connection came to.
#[derive(Clone, Debug, PartialEq)]
pub enum TestState {
    Testing,
    /// It answered, in so many milliseconds.
    Passed(u32),
    Failed(SharedString),
}

/// The port `text` names, from 1 to 65535.
pub(crate) fn parse_port(text: &str) -> Option<u16> {
    text.trim().parse().ok().filter(|port| *port > 0)
}

fn port(field: &Entity<TextInput>, cx: &App) -> Option<u16> {
    parse_port(field.read(cx).text())
}

/// Whether a connection is filled enough to test or save: a name, and a file for SQLite, or a host and a port for the rest.
pub(crate) fn ready(name: &str, engine: Engine, host: &str, database: &str, port: &str) -> bool {
    let filled = |text: &str| !text.trim().is_empty();
    filled(name)
        && match engine {
            Engine::Sqlite => filled(database),
            _ => filled(host) && parse_port(port).is_some(),
        }
}

/// The form's own fields, filled from the connection named by `seed`.
struct Fields {
    seed: SharedString,
    name: Entity<TextInput>,
    host: Entity<TextInput>,
    database: Entity<TextInput>,
    user: Entity<TextInput>,
    password: Entity<TextInput>,
    port: Entity<TextInput>,
    engine: Engine,
    tls: bool,
}

impl Fields {
    fn filled(connection: &Connection, window: &mut Window, cx: &mut App) -> Self {
        let mut field = |text: &SharedString| {
            let text = text.to_string();
            cx.new(|cx| {
                let mut input = TextInput::new(window, cx);
                input.set_text(text, cx);
                input
            })
        };
        Self {
            seed: connection.key.clone(),
            name: field(&connection.name),
            host: field(&connection.host),
            database: field(&connection.database),
            user: field(&connection.user),
            password: field(&connection.password),
            port: field(
                &connection
                    .port
                    .map(|port| port.to_string())
                    .unwrap_or_default()
                    .into(),
            ),
            engine: connection.engine,
            tls: connection.tls,
        }
    }

    fn read(&self, from: &Connection, cx: &App) -> Connection {
        let text = |input: &Entity<TextInput>| {
            SharedString::from(input.read(cx).text().trim().to_string())
        };
        Connection {
            key: from.key.clone(),
            name: text(&self.name),
            engine: self.engine,
            host: text(&self.host),
            port: port(&self.port, cx),
            database: text(&self.database),
            user: text(&self.user),
            password: SharedString::from(self.password.read(cx).text().to_string()),
            tls: self.tls,
            state: from.state.clone(),
        }
    }
}

/// A connection to fill in: its name and engine, then host and port, database, user, password and TLS, or only a file for SQLite. A new connection from the owner refills it. Test asks the owner to try it and shows what the owner reports; Save hands the owner the connection. Both wait for a name, and for a host or a file.
#[derive(IntoElement)]
pub struct ConnectionForm {
    id: ElementId,
    connection: Connection,
    test: Option<TestState>,
    on_test: Option<OnConnection>,
    on_save: Option<OnConnection>,
}

impl ConnectionForm {
    pub fn new(id: impl Into<ElementId>, connection: Connection) -> Self {
        Self {
            id: id.into(),
            connection,
            test: None,
            on_test: None,
            on_save: None,
        }
    }

    /// What the owner's last try came to.
    pub fn test(mut self, state: TestState) -> Self {
        self.test = Some(state);
        self
    }

    pub fn on_test(
        mut self,
        handler: impl Fn(Connection, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_test = Some(Rc::new(handler));
        self
    }

    pub fn on_save(
        mut self,
        handler: impl Fn(Connection, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_save = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ConnectionForm {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (id, connection) = (self.id, self.connection);
        let fields = window.use_keyed_state((id.clone(), "fields"), cx, |window, cx| {
            Fields::filled(&connection, window, cx)
        });
        if fields.read(cx).seed != connection.key {
            log::info!("connection form: filled from {}", connection.key);
            let fresh = Fields::filled(&connection, window, cx);
            fields.update(cx, |fields, _| *fields = fresh);
        }
        let now = fields.read(cx);
        let (engine, tls) = (now.engine, now.tls);
        let (name, host, database, user, password, port_field) = (
            now.name.clone(),
            now.host.clone(),
            now.database.clone(),
            now.user.clone(),
            now.password.clone(),
            now.port.clone(),
        );
        let file = engine == Engine::Sqlite;
        let text = |input: &Entity<TextInput>| input.read(cx).text().to_string();
        let good_port = file || port(&port_field, cx).is_some();
        let ready = ready(
            &text(&name),
            engine,
            &text(&host),
            &text(&database),
            &text(&port_field),
        );
        let testing = self.test == Some(TestState::Testing);
        let (chosen, switched) = (fields.clone(), fields.clone());
        let field = |key: &'static str, label: &'static str, control: gpui::AnyElement| {
            FormField::new((id.clone(), key), label).child(control)
        };
        let ask = |handler: Option<OnConnection>, what: &'static str| {
            let (fields, from) = (fields.clone(), connection.clone());
            move |_: &gpui::ClickEvent, window: &mut Window, cx: &mut App| {
                let connection = fields.read(cx).read(&from, cx);
                log::info!("connection form: {what} {}", connection.key);
                if let Some(handler) = &handler {
                    handler(connection, window, cx);
                }
            }
        };
        let message = self.test.map(|test| match test {
            TestState::Testing => InlineMessage::new(Severity::Info, "Trying the connection…"),
            TestState::Passed(ms) => {
                InlineMessage::new(Severity::Success, format!("Connected in {ms} ms"))
            }
            TestState::Failed(why) => InlineMessage::new(Severity::Danger, why),
        });
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(field("name", "Name", Input::new(&name).into_any_element()))
            .child(field(
                "engine",
                "Engine",
                Select::new(
                    (id.clone(), "engine-select"),
                    Engine::ALL
                        .iter()
                        .map(|engine| Choice::new(engine.name(), engine.name())),
                )
                .selected(engine.name())
                .on_change(move |value, _, cx| {
                    chosen.update(cx, |fields, cx| {
                        fields.engine = Engine::named(value);
                        let port = fields.engine.port().map(|port| port.to_string());
                        fields
                            .port
                            .update(cx, |input, cx| input.set_text(port.unwrap_or_default(), cx));
                        cx.notify();
                    })
                })
                .into_any_element(),
            ))
            .children(file.then(|| field("file", "File", Input::new(&database).into_any_element())))
            .children((!file).then(|| {
                div()
                    .flex()
                    .flex_wrap()
                    .gap_3()
                    .child(div().flex_1().min_w_40().child(field(
                        "host",
                        "Host",
                        Input::new(&host).into_any_element(),
                    )))
                    .child(div().w_32().flex_none().child({
                        let mut port = FormField::new((id.clone(), "port"), "Port")
                            .child(Input::new(&port_field).invalid(!good_port));
                        if !good_port {
                            port = port.error("From 1 to 65535");
                        }
                        port
                    }))
            }))
            .children((!file).then(|| {
                field(
                    "database",
                    "Database",
                    Input::new(&database).into_any_element(),
                )
            }))
            .children((!file).then(|| field("user", "User", Input::new(&user).into_any_element())))
            .children((!file).then(|| {
                field(
                    "password",
                    "Password",
                    PasswordInput::new(&password).into_any_element(),
                )
            }))
            .children((!file).then(|| {
                Switch::new((id.clone(), "tls"), tls)
                    .label("Use TLS")
                    .on_change(move |on, _, cx| {
                        switched.update(cx, |fields, cx| {
                            fields.tls = on;
                            cx.notify();
                        })
                    })
            }))
            .children(message)
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new((id.clone(), "test"), "Test")
                            .variant(ButtonVariant::Secondary)
                            .disabled(!ready || testing)
                            .on_click(ask(self.on_test, "test")),
                    )
                    .child(
                        Button::new((id, "save"), "Save")
                            .variant(ButtonVariant::Primary)
                            .disabled(!ready)
                            .on_click(ask(self.on_save, "save")),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_port, ready};
    use crate::devtools::Engine;

    #[test]
    fn a_port_runs_from_one_to_65535() {
        assert_eq!(parse_port("5433"), Some(5433));
        assert_eq!(parse_port(" 22 "), Some(22));
        assert_eq!(parse_port("0"), None);
        assert_eq!(parse_port("70000"), None);
        assert_eq!(parse_port("http"), None);
    }

    #[test]
    fn a_connection_waits_for_a_name_and_a_place() {
        assert!(ready("Shop", Engine::Postgres, "localhost", "", "5432"));
        assert!(
            !ready("", Engine::Postgres, "localhost", "", "5432"),
            "no name"
        );
        assert!(!ready("Shop", Engine::Postgres, "", "", "5432"), "no host");
        assert!(
            !ready("Shop", Engine::Postgres, "localhost", "", "0"),
            "a bad port"
        );
        assert!(
            !ready("Notes", Engine::Sqlite, "localhost", "", "5432"),
            "SQLite waits for a file"
        );
        assert!(
            ready("Notes", Engine::Sqlite, "", "notes.db", ""),
            "and needs nothing else"
        );
    }
}
