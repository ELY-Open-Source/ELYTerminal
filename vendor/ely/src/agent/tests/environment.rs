use gpui::{
    AppContext as _, Context, Entity, IntoElement, KeyBinding, ParentElement, Render, SharedString,
    Styled, TestAppContext, VisualTestContext, Window, div, px,
};
use jiff::Timestamp;

use super::{press, settle};
use crate::{
    agent::{
        Checkpoint, CheckpointList, McpServer, McpServerList, Memory, MemoryPanel, SandboxState,
        SandboxStatus, ServerState,
    },
    forms::{self, TextInput},
    primitives::FocusNext,
    theme::Theme,
};

fn setup(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        forms::bind_keys(cx);
        cx.bind_keys([KeyBinding::new("tab", FocusNext, None)]);
    });
}

/// A memory panel over a field, and the words it kept.
struct Memories {
    field: Entity<TextInput>,
    kept: Vec<String>,
}

impl Render for Memories {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let memory = Memory {
            key: "tone".into(),
            text: "Prefers plain words".into(),
            source: None,
            at: Timestamp::UNIX_EPOCH,
        };
        div().w(px(360.0)).child(
            MemoryPanel::new("memory", &self.field, [memory])
                .on_add(move |text, _, cx| {
                    view.update(cx, |memories, _| memories.kept.push(text.to_string()))
                })
                .on_forget(|_, _, _| {}),
        )
    }
}

fn memories(cx: &mut TestAppContext) -> (Entity<Memories>, &mut VisualTestContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|window, cx| Memories {
        field: cx.new(|cx| TextInput::new(window, cx)),
        kept: Vec::new(),
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (view, cx)
}

#[gpui::test]
fn enter_keeps_a_memory_and_empties_the_field(cx: &mut TestAppContext) {
    let (view, cx) = memories(cx);
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_input("  ");
    press("enter", cx);
    cx.simulate_input("Lifts stay under 20%");
    press("enter", cx);
    view.read_with(cx, |memories, cx| {
        assert_eq!(
            memories.kept,
            ["Lifts stay under 20%"],
            "blank words are kept out"
        );
        assert_eq!(memories.field.read(cx).text(), "");
    });
}

/// Two servers, and the switches thrown.
struct Servers {
    thrown: Vec<(SharedString, bool)>,
}

impl Render for Servers {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let server = |key: &str, state| McpServer {
            key: key.to_string().into(),
            name: key.to_string().into(),
            detail: "npx server".into(),
            state,
            tools: 3,
        };
        div().w(px(360.0)).child(
            McpServerList::new(
                "servers",
                [
                    server("files", ServerState::Connected),
                    server("web", ServerState::Off),
                ],
            )
            .on_toggle(move |key, on, _, cx| {
                view.update(cx, |servers, _| servers.thrown.push((key.clone(), on)))
            }),
        )
    }
}

#[gpui::test]
fn a_servers_switch_turns_it_off_and_on(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Servers { thrown: Vec::new() });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    cx.update(|window, cx| window.focus_next(cx));
    press("space", cx);
    cx.update(|window, cx| window.focus_next(cx));
    press("space", cx);
    let thrown = view.read_with(cx, |servers, _| servers.thrown.clone());
    assert_eq!(thrown, [("files".into(), false), ("web".into(), true)]);
}

/// Three checkpoints standing on the middle one, and the rewinds asked for.
struct Points {
    asked: Vec<usize>,
}

impl Render for Points {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let point = |key: &str| Checkpoint {
            key: key.to_string().into(),
            label: key.to_string().into(),
            at: Timestamp::UNIX_EPOCH,
            files: 2,
        };
        div().w(px(360.0)).child(
            CheckpointList::new(
                "points",
                [point("first"), point("second"), point("third")],
                1,
            )
            .on_rewind(move |ix, _, cx| view.update(cx, |points, _| points.asked.push(ix))),
        )
    }
}

#[gpui::test]
fn every_checkpoint_but_the_current_rewinds(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Points { asked: Vec::new() });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    cx.update(|window, cx| window.focus_next(cx));
    press("enter", cx);
    cx.update(|window, cx| window.focus_next(cx));
    press("enter", cx);
    assert_eq!(
        view.read_with(cx, |points, _| points.asked.clone()),
        [2, 0],
        "newest first, the current skipped"
    );
}

/// A sandbox past its memory limit and checkpoints whose current one left.
struct Overrun;

impl Render for Overrun {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let point = Checkpoint {
            key: "one".into(),
            label: "One".into(),
            at: Timestamp::UNIX_EPOCH,
            files: 1,
        };
        div()
            .w(px(360.0))
            .child(SandboxStatus::new("box", "python", SandboxState::Ready).memory(5, 4))
            .child(CheckpointList::new("points", [point], 4))
    }
}

#[gpui::test]
fn memory_past_its_limit_and_a_gone_checkpoint_draw(cx: &mut TestAppContext) {
    setup(cx);
    let (_, cx) = cx.add_window_view(|_, _| Overrun);
    settle(cx);
}

#[test]
#[should_panic(expected = "cpu 1.8 of 1")]
fn cpu_past_full_fails_loud() {
    let _ = SandboxStatus::new("box", "python", SandboxState::Ready).cpu(1.8);
}
