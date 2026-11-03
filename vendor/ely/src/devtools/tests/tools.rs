use gpui::{AnyElement, App, Entity, IntoElement, TestAppContext, Window};

use super::{Bench, bench, said, say, tab, tap};
use crate::devtools::{
    Container, ContainerList, ContainerState, Encoder, Reading, RegexTester, ResourceMonitor,
};

fn list(_: &Bench, _: &mut Window, _: &mut App, owner: Entity<Bench>) -> AnyElement {
    let container = |key: &str, state| Container {
        key: key.to_string().into(),
        name: key.to_string().into(),
        image: "img".into(),
        state,
        ports: Vec::new(),
        cpu: 0.1,
        memory: 1 << 20,
    };
    let (toggled, opened) = (owner.clone(), owner);
    ContainerList::new(
        "containers",
        [
            container("web", ContainerState::Running),
            container("db", ContainerState::Exited(0)),
        ],
    )
    .on_toggle(move |key, _, cx| say(&toggled, format!("toggle {key}"), cx))
    .on_open(move |key, _, cx| say(&opened, format!("open {key}"), cx))
    .into_any_element()
}

/// Stops: the list, then web's Stop and db's Start; with no handlers for them, Restart and Logs do not show.
#[gpui::test]
fn a_rows_stop_acts_on_its_container_alone(cx: &mut TestAppContext) {
    let (host, cx) = bench(list, cx);
    tab(2, cx);
    tap("space", cx);
    tab(3, cx);
    tap("space", cx);
    tab(1, cx);
    tap("enter", cx);
    assert_eq!(said(&host, cx), ["toggle web", "toggle db", "open web"]);
}

fn tester(_: &Bench, _: &mut Window, _: &mut App, _: Entity<Bench>) -> AnyElement {
    RegexTester::new("regex", "a", "Aa").into_any_element()
}

/// Stops: the pattern, then the flags i, m, s and x.
#[gpui::test]
fn a_flag_changes_what_matches(cx: &mut TestAppContext) {
    let (_, cx) = bench(tester, cx);
    assert!(
        cx.debug_bounds("regex-matches-1").is_some(),
        "a alone matches once"
    );
    tab(2, cx);
    tap("space", cx);
    assert!(
        cx.debug_bounds("regex-matches-2").is_some(),
        "ignoring case matches A too"
    );
}

fn encoder(_: &Bench, _: &mut Window, _: &mut App, _: Entity<Bench>) -> AnyElement {
    Encoder::new("encoder", "aGk=").into_any_element()
}

/// Stops: Base64, URL and Hex, Encode and Decode, the text, then Copy.
#[gpui::test]
fn decoding_base64_copies_the_text(cx: &mut TestAppContext) {
    let (_, cx) = bench(encoder, cx);
    tab(5, cx);
    tap("space", cx);
    tab(7, cx);
    tap("space", cx);
    let copied = cx
        .update(|_, cx| cx.read_from_clipboard())
        .and_then(|item| item.text());
    assert_eq!(copied.as_deref(), Some("hi"));
}

fn over(_: &Bench, _: &mut Window, _: &mut App, _: Entity<Bench>) -> AnyElement {
    ResourceMonitor::new(
        "monitor",
        [Reading::new("Queue", "jobs", [80.0, 120.0]).limit(100.0)],
    )
    .into_any_element()
}

/// A meter takes a share from 0 to 1, so a reading past its limit must fill it, not break it.
#[gpui::test]
fn a_reading_past_its_limit_fills_its_meter(cx: &mut TestAppContext) {
    let (_, cx) = bench(over, cx);
    assert!(cx.debug_bounds("monitor-tile-0").is_some(), "the tile drew");
}
