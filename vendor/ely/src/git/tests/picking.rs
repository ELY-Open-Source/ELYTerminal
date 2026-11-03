//! Long lists draw in view, load more, pick by key.

use std::{cell::RefCell, rc::Rc};

use gpui::{
    Context, IntoElement, KeyBinding, KeyUpEvent, Keystroke, ParentElement, Render, Styled,
    TestAppContext, VisualTestContext, Window, div, px,
};

use crate::git::{
    Blame, BlameView, Branch, BranchList, Commit, CommitList, FileHistory, GitTag, Stash,
    StashList, TagList,
};
use crate::{forms, primitives::FocusNext, theme::Theme};

fn commit(n: usize) -> Commit {
    Commit {
        id: format!("{n:040}").into(),
        parents: vec![format!("{:040}", n + 1).into()],
        subject: format!("change {n}").into(),
        author: "Ann".into(),
        when: "today".into(),
        refs: Vec::new(),
    }
}

/// What a list asked for.
#[derive(Default)]
struct Heard {
    picked: Vec<String>,
    ends: usize,
}

struct Lists {
    count: usize,
    heard: Rc<RefCell<Heard>>,
    which: &'static str,
}

impl Render for Lists {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let commits: Vec<Commit> = (0..self.count).map(commit).collect();
        let (pick, end) = (self.heard.clone(), self.heard.clone());
        let frame = div().w(px(600.0)).h(px(240.0));
        match self.which {
            "commits" => frame.child(
                CommitList::new("commits", Rc::new(commits))
                    .on_pick(move |id, _, _| pick.borrow_mut().picked.push(id.to_string()))
                    .on_end(move |_| end.borrow_mut().ends += 1),
            ),
            "history" => frame.child(
                FileHistory::new(
                    "history",
                    Rc::new(commits.into_iter().map(|c| (c, None)).collect()),
                )
                .on_pick(move |id, _, _| pick.borrow_mut().picked.push(id.to_string())),
            ),
            "refs" => {
                let branch = |name: &str, current| Branch {
                    name: name.to_string().into(),
                    remote: false,
                    current,
                    ahead: 0,
                    behind: 0,
                    subject: "s".into(),
                    when: "today".into(),
                };
                let tag = GitTag {
                    name: "v1".into(),
                    message: None,
                    commit: "a1".into(),
                    when: "today".into(),
                };
                let (tagged, switched, deleted) =
                    (self.heard.clone(), self.heard.clone(), self.heard.clone());
                frame
                    .child(
                        BranchList::new("branches", [branch("main", true), branch("side", false)])
                            .on_pick(move |name, _, _| {
                                pick.borrow_mut().picked.push(name.to_string())
                            })
                            .on_switch(move |name, _, _| {
                                switched.borrow_mut().picked.push(format!("switch {name}"))
                            })
                            .on_delete(move |name, _, _| {
                                deleted.borrow_mut().picked.push(format!("delete {name}"))
                            }),
                    )
                    .child(TagList::new("tags", [tag]).on_pick(move |name, _, _| {
                        tagged.borrow_mut().picked.push(name.to_string())
                    }))
            }
            "stashes" => {
                let stash = |message: &str| Stash {
                    message: message.to_string().into(),
                    branch: "main".into(),
                    when: "today".into(),
                };
                let acted = self.heard.clone();
                frame
                    .child(StashList::new("bare", [stash("unhandled")]))
                    .child(
                        StashList::new("stashes", [stash("one"), stash("two")])
                            .selected(1)
                            .on_pick(move |ix, _, _| pick.borrow_mut().picked.push(format!("{ix}")))
                            .on_action(move |ix, what, _, _| {
                                acted.borrow_mut().picked.push(format!("{what:?} {ix}"))
                            }),
                    )
            }
            _ => {
                let blame = |commit: &str| Blame {
                    commit: commit.into(),
                    author: "Ann".into(),
                    when: "today".into(),
                    subject: "first".into(),
                    age: 0.0,
                };
                frame.child(
                    BlameView::new(
                        "blame",
                        "one\ntwo\nthree",
                        [0, 0, 1],
                        [blame("a1"), blame("b2")],
                    )
                    .on_commit(move |id, _, _| pick.borrow_mut().picked.push(id.to_string())),
                )
            }
        }
    }
}

fn shown<'a>(
    cx: &'a mut TestAppContext,
    count: usize,
    which: &'static str,
) -> (Rc<RefCell<Heard>>, &'a mut VisualTestContext) {
    cx.update(|cx| {
        Theme::init(cx);
        forms::bind_keys(cx);
        cx.bind_keys([KeyBinding::new("tab", FocusNext, None)]);
    });
    let heard = Rc::new(RefCell::new(Heard::default()));
    let held = heard.clone();
    let (_, cx) = cx.add_window_view(move |_, _| Lists {
        count,
        heard: held,
        which,
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (heard, cx)
}

fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

/// Tab to the first stop and press Enter.
fn press_first(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.focus_next(cx));
    settle(cx);
    cx.simulate_keystrokes("enter");
    settle(cx);
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("enter").expect("a key"),
    });
    settle(cx);
}

#[gpui::test]
fn a_long_list_asks_for_more_only_near_its_end(cx: &mut TestAppContext) {
    let (heard, _) = shown(cx, 500, "commits");
    assert_eq!(heard.borrow().ends, 0, "500 rows, few in view");
}

#[gpui::test]
fn a_short_list_asks_for_more(cx: &mut TestAppContext) {
    let (heard, _) = shown(cx, 12, "commits");
    assert!(heard.borrow().ends > 0);
}

#[gpui::test]
fn a_commit_picks_from_the_keyboard(cx: &mut TestAppContext) {
    let (heard, cx) = shown(cx, 500, "commits");
    press_first(cx);
    assert_eq!(heard.borrow().picked, [format!("{:040}", 0)]);
}

#[gpui::test]
fn a_file_history_row_picks_from_the_keyboard(cx: &mut TestAppContext) {
    let (heard, cx) = shown(cx, 300, "history");
    press_first(cx);
    assert_eq!(heard.borrow().picked, [format!("{:040}", 0)]);
}

#[gpui::test]
fn a_blame_run_picks_its_commit_from_the_keyboard(cx: &mut TestAppContext) {
    let (heard, cx) = shown(cx, 0, "blame");
    press_first(cx);
    cx.update(|window, cx| window.focus_next(cx));
    settle(cx);
    cx.simulate_keystrokes("enter");
    settle(cx);
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("enter").expect("a key"),
    });
    settle(cx);
    assert_eq!(
        heard.borrow().picked,
        ["a1", "b2"],
        "each run's head, no stop between"
    );
}

#[gpui::test]
fn branches_and_tags_pick_from_the_keyboard(cx: &mut TestAppContext) {
    let (heard, cx) = shown(cx, 0, "refs");
    press_first(cx);
    // Step through the stops until the tag is picked.
    for _ in 0..8 {
        if heard.borrow().picked.iter().any(|name| name == "v1") {
            break;
        }
        press_first(cx);
    }
    let picked = heard.borrow().picked.clone();
    assert_eq!(picked.first().map(String::as_str), Some("main"));
    assert!(
        picked.contains(&"side".to_string()) && picked.contains(&"v1".to_string()),
        "{picked:?}"
    );
    let acted = |what: &str| picked.iter().any(|said| said == what);
    assert!(
        acted("switch side") && acted("delete side"),
        "row actions answer: {picked:?}"
    );
}

#[gpui::test]
fn stashes_pick_and_act_from_the_keyboard(cx: &mut TestAppContext) {
    let (heard, cx) = shown(cx, 0, "stashes");
    for _ in 0..4 {
        press_first(cx);
    }
    assert_eq!(
        heard.borrow().picked,
        ["0", "Apply 0", "Pop 0", "Drop 0"],
        "a list with no owner draws no stops"
    );
}
