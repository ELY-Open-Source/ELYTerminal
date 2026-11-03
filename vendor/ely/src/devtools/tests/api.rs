use gpui::{
    AnyElement, App, AppContext as _, Entity, IntoElement, ParentElement, Styled, TestAppContext,
    VisualTestContext, Window, div, px,
};

use super::{Bench, bench, said, say, settle, tab, tap};
use crate::{
    devtools::{
        ApiRequest, ApiRequestBuilder, Auth, CollectionTree, Direction, Environment,
        EnvironmentSelector, GraphQLExplorer, Method, Saved, SchemaType, SocketMessage,
        SocketState, TypeKind, WebSocketConsole,
    },
    editor::CodeEditor,
    forms::TextInput,
};

fn builder(bench: &Bench, _: &mut Window, _: &mut App, owner: Entity<Bench>) -> AnyElement {
    let mut request = ApiRequest::new("a", Method::Get, "https://{{host}}/items");
    request.params = vec![("q".into(), "red shoes".into())];
    request.auth = Auth::Basic {
        user: "ada".into(),
        password: "pw".into(),
    };
    let mut sending = false;
    match bench.choice {
        1 => {
            request = ApiRequest::new("b", Method::Post, "https://b.example.com/orders");
            request.auth = Auth::Bearer("t0k".into());
        }
        2 => request = ApiRequest::new("c", Method::Get, ""),
        3 => sending = true,
        _ => {}
    }
    ApiRequestBuilder::new("builder", request)
        .variables([("host", "api.example.com")])
        .sending(sending)
        .on_send(move |request, _, cx| {
            let words = format!(
                "{} {} {:?}",
                request.method.name(),
                request.address(),
                request.auth
            );
            say(&owner, words, cx)
        })
        .into_any_element()
}

fn choose(host: &Entity<Bench>, choice: usize, cx: &mut VisualTestContext) {
    host.update(cx, |bench, cx| {
        bench.choice = choice;
        cx.notify();
    });
    settle(cx);
}

/// Stops: the method, the address, then Send.
#[gpui::test]
fn send_hands_on_the_request_with_its_pairs(cx: &mut TestAppContext) {
    let (host, cx) = bench(builder, cx);
    tab(3, cx);
    tap("space", cx);
    assert_eq!(
        said(&host, cx),
        [r#"GET https://{{host}}/items?q=red%20shoes Basic { user: "ada", password: "pw" }"#]
    );
}

/// Stops: the method, the address, then Send.
#[gpui::test]
fn a_new_request_refills_the_builder_with_its_sign_in(cx: &mut TestAppContext) {
    let (host, cx) = bench(builder, cx);
    choose(&host, 1, cx);
    tab(3, cx);
    tap("space", cx);
    assert_eq!(
        said(&host, cx),
        [r#"POST https://b.example.com/orders Bearer("t0k")"#]
    );
}

/// Stops: the method and the address; Send rests, so a third Tab wraps to the method.
#[gpui::test]
fn send_rests_without_an_address_and_while_sending(cx: &mut TestAppContext) {
    let (host, cx) = bench(builder, cx);
    for choice in [2, 3] {
        choose(&host, choice, cx);
        tab(3, cx);
        tap("space", cx);
        tap("escape", cx);
    }
    assert!(said(&host, cx).is_empty());
}

fn console(bench: &Bench, window: &mut Window, cx: &mut App, owner: Entity<Bench>) -> AnyElement {
    let draft = window.use_keyed_state("draft", cx, TextInput::new);
    let now: jiff::Timestamp = "2026-09-27T12:00:00Z".parse().expect("a time");
    let state = match bench.choice {
        0 => SocketState::Open,
        _ => SocketState::Closed,
    };
    let toggled = owner.clone();
    WebSocketConsole::new(
        "socket",
        "wss://example.com",
        state,
        [],
        &draft,
        now,
        move |text, _, cx| say(&owner, format!("send {text}"), cx),
    )
    .on_toggle(move |_, cx| say(&toggled, "toggle".into(), cx))
    .into_any_element()
}

/// Stops: Disconnect, the message field, then Send once it holds words.
#[gpui::test]
fn enter_in_the_field_sends_and_empties_it(cx: &mut TestAppContext) {
    let (host, cx) = bench(console, cx);
    tab(2, cx);
    cx.simulate_input("ping");
    tap("enter", cx);
    tap("enter", cx);
    assert_eq!(
        said(&host, cx),
        ["send ping"],
        "an empty field sends nothing"
    );
}

/// Stops while closed: Connect, then the field; Send rests, so a third Tab wraps to Connect.
#[gpui::test]
fn a_closed_socket_sends_nothing_and_connect_asks_the_owner(cx: &mut TestAppContext) {
    let (host, cx) = bench(console, cx);
    host.update(cx, |bench, cx| {
        bench.choice = 1;
        cx.notify();
    });
    settle(cx);
    tab(2, cx);
    cx.simulate_input("ping");
    tap("enter", cx);
    tab(3, cx);
    tap("space", cx);
    assert_eq!(said(&host, cx), ["toggle"]);
}

fn saved(_: &Bench, _: &mut Window, _: &mut App, owner: Entity<Bench>) -> AnyElement {
    let list = Saved::Request {
        key: "list".into(),
        name: "List orders".into(),
        method: Method::Get,
    };
    let orders = Saved::Folder {
        key: "orders".into(),
        name: "Orders".into(),
        items: vec![list],
    };
    div()
        .h(px(200.0))
        .child(
            CollectionTree::new("saved", [orders])
                .open(["orders"])
                .on_open(move |key, _, cx| say(&owner, format!("open {key}"), cx)),
        )
        .into_any_element()
}

/// Stops: the tree, its cursor on the folder.
#[gpui::test]
fn enter_on_a_saved_request_opens_it(cx: &mut TestAppContext) {
    let (host, cx) = bench(saved, cx);
    tab(1, cx);
    tap("down", cx);
    tap("enter", cx);
    assert_eq!(said(&host, cx), ["open list"]);
}

fn graph(bench: &Bench, window: &mut Window, cx: &mut App, owner: Entity<Bench>) -> AnyElement {
    let (Some(variables), query) = (
        bench.editor.clone(),
        window.use_keyed_state("query", cx, |window, cx| {
            CodeEditor::new("{ a }", window, cx)
        }),
    ) else {
        return div().into_any_element();
    };
    let search = window.use_keyed_state("search", cx, TextInput::new);
    GraphQLExplorer::new(
        "graph",
        [],
        &query,
        &variables,
        &search,
        move |query, variables, _, cx| say(&owner, format!("{query} {variables}"), cx),
    )
    .into_any_element()
}

/// Stops: the schema, the query, the variables, then Run while they read.
#[gpui::test]
fn run_waits_for_variables_that_read(cx: &mut TestAppContext) {
    let (host, cx) = bench(graph, cx);
    let variables = cx.update(|window, cx| cx.new(|cx| CodeEditor::new("{", window, cx)));
    host.update(cx, |bench, cx| {
        bench.editor = Some(variables.clone());
        cx.notify();
    });
    settle(cx);
    tab(4, cx);
    tap("space", cx);
    assert!(
        said(&host, cx).is_empty(),
        "variables that do not read keep Run at rest"
    );
    variables.update(cx, |editor, cx| {
        editor.set_text(r#"{"id": 1}"#.to_string(), cx)
    });
    settle(cx);
    tab(4, cx);
    tap("space", cx);
    assert_eq!(said(&host, cx), [r#"{ a } {"id": 1}"#]);
}

fn environments(bench: &Bench, _: &mut Window, _: &mut App, _: Entity<Bench>) -> AnyElement {
    let env = |key: &str, token: &str| Environment {
        key: key.to_string().into(),
        name: key.to_string().into(),
        variables: vec![("token".into(), token.to_string().into())],
        secrets: vec!["token".into()],
    };
    let selected = match bench.choice {
        0 => "staging",
        1 => "production",
        _ => "gone",
    };
    EnvironmentSelector::new(
        "environments",
        [
            env("staging", "sk_test_1111"),
            env("production", "sk_live_9999"),
        ],
        selected,
        |_, _, _| {},
    )
    .into_any_element()
}

/// Stops: the Select, then the token's eye.
#[gpui::test]
fn an_eye_opens_one_environments_secret_only(cx: &mut TestAppContext) {
    let (host, cx) = bench(environments, cx);
    tab(2, cx);
    tap("space", cx);
    assert!(cx.debug_bounds("environment-staging-token-shown").is_some());
    choose(&host, 1, cx);
    assert!(
        cx.debug_bounds("environment-production-token-masked")
            .is_some()
    );
    assert!(
        cx.debug_bounds("environment-production-token-shown")
            .is_none(),
        "another environment's secret stays masked"
    );
}

fn schema(_: &Bench, window: &mut Window, cx: &mut App, owner: Entity<Bench>) -> AnyElement {
    let query = window.use_keyed_state("query", cx, |window, cx| {
        CodeEditor::new("{ a }", window, cx)
    });
    let variables = window.use_keyed_state("variables", cx, |window, cx| {
        CodeEditor::new("", window, cx)
    });
    let search = window.use_keyed_state("search", cx, TextInput::new);
    let order = SchemaType {
        name: "Order".into(),
        kind: TypeKind::Object,
        fields: vec![("id".into(), "ID!".into())],
    };
    GraphQLExplorer::new(
        "graph",
        [order],
        &query,
        &variables,
        &search,
        |_, _, _, _| {},
    )
    .on_pick(move |path, _, cx| say(&owner, path.to_string(), cx))
    .into_any_element()
}

/// Stops: the schema first, its cursor on Order.
#[gpui::test]
fn enter_on_a_field_hands_its_path(cx: &mut TestAppContext) {
    let (host, cx) = bench(schema, cx);
    tab(1, cx);
    for key in ["right", "down", "enter"] {
        tap(key, cx);
    }
    assert_eq!(said(&host, cx), ["Order.id"]);
}

fn log(_: &Bench, window: &mut Window, cx: &mut App, owner: Entity<Bench>) -> AnyElement {
    let draft = window.use_keyed_state("draft", cx, TextInput::new);
    let now: jiff::Timestamp = "2026-09-27T12:00:00Z".parse().expect("a time");
    let messages = (0..12).map(|ix| SocketMessage {
        direction: Direction::Received,
        text: format!("message {ix}").into(),
        at: now,
    });
    WebSocketConsole::new(
        "socket",
        "wss://example.com",
        SocketState::Open,
        messages,
        &draft,
        now,
        move |text, _, cx| say(&owner, format!("send {text}"), cx),
    )
    .into_any_element()
}

#[gpui::test]
fn the_log_holds_at_its_newest_message(cx: &mut TestAppContext) {
    let (_, cx) = bench(log, cx);
    let newest = cx
        .debug_bounds("socket-message-11")
        .expect("the newest message is drawn");
    let log = cx.debug_bounds("socket-log").expect("the log");
    assert!(
        newest.bottom() <= log.bottom() && newest.top() >= log.top(),
        "the newest message lies inside the log: {newest:?} in {log:?}"
    );
}

#[gpui::test]
fn an_environment_that_left_shows_none_chosen(cx: &mut TestAppContext) {
    let (host, cx) = bench(environments, cx);
    host.update(cx, |bench, cx| {
        bench.choice = 2;
        cx.notify();
    });
    settle(cx);
    assert!(
        cx.debug_bounds("environment-staging-token-masked")
            .is_none()
    );
    assert!(
        cx.debug_bounds("environment-production-token-masked")
            .is_none()
    );
}
