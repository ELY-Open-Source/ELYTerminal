use gpui::{
    AppContext as _, Context, Entity, Focusable, IntoElement, ParentElement, Render, Styled,
    TestAppContext, VisualTestContext, Window, div,
};

use super::{Decision, FollowMode, Peer, Reply, Suggestion, TrackChanges};
use crate::{
    forms,
    forms::{Input, TextInput},
    theme::{ActiveTheme, ControlSize, Theme},
};

/// A field outside a followed view and one inside it, with what they asked: replies sent, follows stopped.
struct Desk {
    outside: Entity<TextInput>,
    field: Entity<TextInput>,
    following: bool,
    sent: usize,
    stopped: usize,
}

impl Render for Desk {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (send, stop) = (cx.entity(), cx.entity());
        let peer = self.following.then(|| Peer::new("ada", "Ada", 0));
        div().size_full().child(Input::new(&self.outside)).child(
            FollowMode::new("follow", peer, move |_, cx| {
                stop.update(cx, |desk, _| desk.stopped += 1)
            })
            .child(Reply::new("reply", &self.field, move |_, cx| {
                send.update(cx, |desk, _| desk.sent += 1)
            })),
        )
    }
}

fn open(following: bool, cx: &mut TestAppContext) -> (Entity<Desk>, &mut VisualTestContext) {
    cx.update(|cx| {
        Theme::init(cx);
        forms::bind_keys(cx);
    });
    let (desk, cx) = cx.add_window_view(move |window, cx| Desk {
        outside: cx.new(|cx| TextInput::new(window, cx)),
        field: cx.new(|cx| TextInput::new(window, cx)),
        following,
        sent: 0,
        stopped: 0,
    });
    cx.update(|window, cx| {
        window.activate_window();
        let field = desk.read(cx).field.clone();
        window.focus(&field.focus_handle(cx), cx);
    });
    cx.run_until_parked();
    (desk, cx)
}

#[gpui::test]
fn enter_sends_a_reply_only_with_words(cx: &mut TestAppContext) {
    let (desk, cx) = open(false, cx);
    cx.simulate_keystrokes("space enter");
    assert_eq!(desk.read_with(cx, |desk, _| desk.sent), 0, "blank stays");
    cx.simulate_input("agreed");
    cx.simulate_keystrokes("enter");
    assert_eq!(desk.read_with(cx, |desk, _| desk.sent), 1);
}

#[gpui::test]
fn escape_stops_following_only_while_following(cx: &mut TestAppContext) {
    let (desk, cx) = open(true, cx);
    cx.simulate_keystrokes("escape");
    assert_eq!(desk.read_with(cx, |desk, _| desk.stopped), 1);
    desk.update(cx, |desk, cx| {
        desk.following = false;
        cx.notify();
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("escape");
    assert_eq!(desk.read_with(cx, |desk, _| desk.stopped), 1);
}

#[gpui::test]
fn following_begun_from_outside_still_stops_on_escape(cx: &mut TestAppContext) {
    let (desk, cx) = open(false, cx);
    cx.update(|window, cx| {
        let outside = desk.read(cx).outside.focus_handle(cx);
        window.focus(&outside, cx);
    });
    desk.update(cx, |desk, cx| {
        desk.following = true;
        cx.notify();
    });
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    cx.simulate_keystrokes("escape");
    assert_eq!(desk.read_with(cx, |desk, _| desk.stopped), 1);
    let back = cx.update(|window, cx| desk.read(cx).outside.focus_handle(cx).is_focused(window));
    assert!(back, "focus returns to where it was");
}

/// Two threads, one resolved, and the key each pick asked for.
struct Board {
    picked: Vec<gpui::SharedString>,
}

impl Render for Board {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let thread = |key: &str, resolved| super::Thread {
            key: key.to_string().into(),
            quote: None,
            comments: vec![super::Comment {
                author: Peer::new("ada", "Ada", 0),
                at: jiff::Timestamp::UNIX_EPOCH,
                body: format!("About {key}").into(),
                reactions: Vec::new(),
            }],
            resolved,
        };
        let pick = cx.entity();
        div().size_full().child(
            super::CommentSidebar::new("remarks", [thread("a", false), thread("b", true)])
                .active(None, move |key, _, cx| {
                    pick.update(cx, |board, _| board.picked.push(key.clone()))
                }),
        )
    }
}

#[gpui::test]
fn without_resolving_one_list_takes_keyboard_picks(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        forms::bind_keys(cx);
    });
    let (board, cx) = cx.add_window_view(|_, _| Board { picked: Vec::new() });
    cx.run_until_parked();
    assert!(cx.debug_bounds("comment-a").is_some());
    assert!(cx.debug_bounds("comment-b").is_some(), "resolved shows too");
    cx.update(|window, cx| {
        window.activate_window();
        window.focus_next(cx);
    });
    cx.simulate_keystrokes("enter");
    let picked = board.read_with(cx, |board, _| board.picked.clone());
    assert_eq!(picked, vec![gpui::SharedString::from("a")]);
}

/// Two suggestions whose first falls inside a character once the text changes, and the decisions asked.
struct Changing {
    changed: bool,
    decided: Vec<Decision>,
}

impl Render for Changing {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        let suggestion = |range, text: &str| Suggestion {
            author: Peer::new("ada", "Ada", 0),
            range,
            text: text.to_string().into(),
        };
        let (base, second) = match self.changed {
            false => ("abc", 1..2),
            true => ("éabc", 2..3),
        };
        let pending = [suggestion(0..1, "X"), suggestion(second, "B")];
        div()
            .size_full()
            .child(
                TrackChanges::new("track", base, pending).on_decide(move |decision, _, cx| {
                    owner.update(cx, |owner, _| owner.decided.push(decision))
                }),
            )
    }
}

#[gpui::test]
fn a_press_on_a_suggestion_left_out_decides_nothing(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (view, cx) = cx.add_window_view(|_, _| Changing {
        changed: false,
        decided: Vec::new(),
    });
    cx.run_until_parked();
    let row = cx
        .debug_bounds("suggestion 0")
        .expect("the first suggestion");
    let at = cx.update(|window, cx| {
        let button = cx
            .theme()
            .control_height(ControlSize::Sm)
            .to_pixels(window.rem_size());
        let gap = gpui::rems(0.5).to_pixels(window.rem_size());
        gpui::point(row.right() - button - gap - button / 2.0, row.center().y)
    });
    cx.simulate_click(at, gpui::Modifiers::none());
    assert_eq!(
        view.read_with(cx, |view, _| view.decided.clone()),
        [Decision::Accept(0)]
    );
    cx.simulate_mouse_down(at, gpui::MouseButton::Left, gpui::Modifiers::none());
    view.update(cx, |view, cx| {
        view.changed = true;
        cx.notify();
    });
    cx.run_until_parked();
    cx.simulate_mouse_up(at, gpui::MouseButton::Left, gpui::Modifiers::none());
    cx.run_until_parked();
    assert_eq!(
        view.read_with(cx, |view, _| view.decided.clone()),
        [Decision::Accept(0)]
    );
}
