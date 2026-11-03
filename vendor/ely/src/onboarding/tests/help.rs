use gpui::{AnyElement, Entity, IntoElement, ParentElement, Styled, TestAppContext, div, px};

use super::{Bench, bench, edit, said, say, tab, tap, write};
use crate::{
    data_display::{Change, Release},
    forms::Choice,
    onboarding::{
        ContactSupport, FeedbackWidget, HelpArticle, HelpPanel, InlineHelp,
        KeyboardShortcutCheatsheet, WhatsNewDialog,
    },
    settings::Shortcut,
};

fn article(key: &str, title: &str, body: &str) -> HelpArticle {
    HelpArticle {
        key: key.to_string().into(),
        title: title.to_string().into(),
        summary: "A line on it".into(),
        body: body.to_string().into(),
    }
}

fn panel(bench: &Bench, owner: Entity<Bench>) -> AnyElement {
    let articles = [
        article("start", "Getting started", "Make a project."),
        article("team", "Invite your team", "Send an invitation by email."),
    ];
    let [opener, contact, keys, close] = [(); 4].map(|_| owner.clone());
    div()
        .h(px(420.0))
        .child(
            HelpPanel::new(
                "help",
                articles,
                &bench.search,
                bench.open.clone(),
                move |key, _, cx| {
                    let key = key.cloned();
                    opener.update(cx, |bench, cx| {
                        bench.said.push(format!(
                            "open {}",
                            key.as_ref().map_or("none", |key| key.as_ref())
                        ));
                        bench.open = key;
                        cx.notify();
                    })
                },
            )
            .on_contact(move |_, cx| say(&contact, "contact".into(), cx))
            .on_shortcuts(move |_, cx| say(&keys, "shortcuts".into(), cx))
            .on_close(move |_, cx| say(&close, "close".into(), cx)),
        )
        .into_any_element()
}

/// Stops on the list: close, the search, the list, then the foot; on an article: close, All articles, then the foot. Enter on the list's cursor, on the first row at first, opens an article and takes focus to All articles, which the release leaves alone; the way back takes it to the search, where words narrow the list.
#[gpui::test]
fn an_article_opens_from_the_list_and_focus_follows(cx: &mut TestAppContext) {
    let (host, cx) = bench(panel, cx);
    tab(3, cx);
    tap("enter", cx);
    tap("space", cx);
    write("email", cx);
    tab(3, cx);
    tap("enter", cx);
    tab(3, cx);
    tap("space", cx);
    tab(4, cx);
    tap("space", cx);
    tab(1, cx);
    tap("space", cx);
    assert_eq!(
        said(&host, cx),
        [
            "open start",
            "open none",
            "open team",
            "contact",
            "shortcuts",
            "close"
        ]
    );
}

fn many(bench: &Bench, owner: Entity<Bench>) -> AnyElement {
    let articles = (1..=12).map(|n| article(&format!("a{n}"), &format!("Article {n}"), "Words."));
    let (opener, contact) = (owner.clone(), owner);
    div()
        .w(px(280.0))
        .h(px(300.0))
        .child(
            HelpPanel::new(
                "help",
                articles,
                &bench.search,
                bench.open.clone(),
                move |key, _, cx| {
                    let key = key.cloned();
                    opener.update(cx, |bench, cx| {
                        bench.said.push(format!(
                            "open {}",
                            key.as_ref().map_or("none", |key| key.as_ref())
                        ));
                        bench.open = key;
                        cx.notify();
                    })
                },
            )
            .on_contact(move |_, cx| say(&contact, "contact".into(), cx)),
        )
        .into_any_element()
}

/// Twelve articles in a 300px panel scroll inside the list, so the foot stays in the box, and the cursor walks down to the eighth.
#[gpui::test]
fn a_long_list_scrolls_inside_the_panel(cx: &mut TestAppContext) {
    let (host, cx) = bench(many, cx);
    let foot = cx.debug_bounds("help-foot").expect("the foot");
    assert!(
        foot.bottom() <= px(316.0),
        "the foot ends inside the 300px panel, which starts 16px down: {foot:?}"
    );
    tab(2, cx);
    for _ in 0..7 {
        tap("down", cx);
    }
    tap("enter", cx);
    assert_eq!(said(&host, cx), ["open a8"]);
}

fn cheatsheet(bench: &Bench, owner: Entity<Bench>) -> AnyElement {
    let shortcut = |name: &str, keys: &str| Shortcut {
        group: "General".into(),
        name: name.to_string().into(),
        keys: keys.to_string().into(),
    };
    let dialog = bench.dialog.then(|| {
        KeyboardShortcutCheatsheet::new(
            "keys",
            [shortcut("Undo", "cmd-z"), shortcut("Find", "cmd-f")],
            &bench.search,
            move |_, cx| say(&owner, "closed".into(), cx),
        )
    });
    div().children(dialog).into_any_element()
}

/// The search takes focus as the dialog opens, and Escape closes it.
#[gpui::test]
fn the_cheatsheet_opens_on_its_search_and_escape_closes_it(cx: &mut TestAppContext) {
    let (host, cx) = bench(cheatsheet, cx);
    edit(&host, cx, |bench| bench.dialog = true);
    write("undo", cx);
    let typed = host.read_with(cx, |bench, cx| bench.search.read(cx).text().to_string());
    assert_eq!(typed, "undo");
    tap("escape", cx);
    assert_eq!(said(&host, cx), ["closed"]);
}

fn whats_new(bench: &Bench, owner: Entity<Bench>) -> AnyElement {
    let dialog = bench.dialog.then(|| {
        WhatsNewDialog::new("new", move |_, cx| say(&owner, "closed".into(), cx))
            .release(Release::new("2.4", "27 Sep 2026").change(Change::Added, "Split view"))
    });
    div().children(dialog).into_any_element()
}

/// Stops: the close button, then Got it.
#[gpui::test]
fn whats_new_lists_the_releases_and_got_it_closes(cx: &mut TestAppContext) {
    let (host, cx) = bench(whats_new, cx);
    edit(&host, cx, |bench| bench.dialog = true);
    assert!(cx.debug_bounds("whats-new-releases").is_some());
    assert!(cx.debug_bounds("whats-new-done").is_some());
    tap("tab", cx);
    tap("tab", cx);
    tap("space", cx);
    assert_eq!(said(&host, cx), ["closed"]);
}

fn feedback(_: &Bench, owner: Entity<Bench>) -> AnyElement {
    FeedbackWidget::new("feedback", move |sentiment, note, _, cx| {
        say(&owner, format!("{} {note}", sentiment.key()), cx)
    })
    .into_any_element()
}

/// Stops in the form: the four faces, the note, then Send once a face is picked.
#[gpui::test]
fn feedback_waits_for_a_face_and_says_thanks(cx: &mut TestAppContext) {
    let (host, cx) = bench(feedback, cx);
    tab(1, cx);
    tap("enter", cx);
    for _ in 0..5 {
        tap("tab", cx);
    }
    write("  Fast  ", cx);
    for _ in 0..3 {
        tap("tab", cx);
    }
    tap("space", cx);
    for _ in 0..3 {
        tap("tab", cx);
    }
    tap("space", cx);
    assert_eq!(
        said(&host, cx),
        ["good Fast"],
        "Send rested past the note, so the third Tab reached Good"
    );
    assert!(cx.debug_bounds("feedback-thanks").is_some());
}

fn support(bench: &Bench, owner: Entity<Bench>) -> AnyElement {
    let topics = [
        Choice::new("billing", "Billing"),
        Choice::new("bug", "Something broke"),
    ];
    ContactSupport::new("support", topics, move |topic, message, _, cx| {
        say(&owner, format!("{topic}: {message}"), cx)
    })
    .busy(bench.busy)
    .address("help@example.com")
    .into_any_element()
}

/// Stops: the topic, the message, then Send once both are given; the message empties once sent, with focus back in it.
#[gpui::test]
fn a_message_goes_with_its_topic_and_empties(cx: &mut TestAppContext) {
    let (host, cx) = bench(support, cx);
    tab(2, cx);
    write("It froze", cx);
    tab(3, cx);
    tap("space", cx);
    assert!(
        said(&host, cx).is_empty(),
        "no topic yet, so Send rests and the third stop wraps to the topic"
    );
    tab(1, cx);
    for key in ["escape", "enter", "down", "enter"] {
        tap(key, cx);
    }
    tab(3, cx);
    tap("space", cx);
    write("Again", cx);
    tab(3, cx);
    tap("space", cx);
    assert_eq!(said(&host, cx), ["bug: It froze", "bug: Again"]);
}

/// While the owner sends, Send spins and takes no press, yet stays the third stop and keeps its focus: once sending ends, the same press sends.
#[gpui::test]
fn a_message_waits_while_busy(cx: &mut TestAppContext) {
    let (host, cx) = bench(support, cx);
    edit(&host, cx, |bench| bench.busy = true);
    tab(1, cx);
    for key in ["enter", "down", "enter"] {
        tap(key, cx);
    }
    tab(2, cx);
    write("It froze", cx);
    tab(3, cx);
    tap("space", cx);
    assert!(said(&host, cx).is_empty(), "Send takes no press while busy");
    edit(&host, cx, |bench| bench.busy = false);
    tap("space", cx);
    assert_eq!(said(&host, cx), ["bug: It froze"]);
}

fn long_address(_: &Bench, owner: Entity<Bench>) -> AnyElement {
    div()
        .w(px(280.0))
        .child(
            ContactSupport::new(
                "support",
                [Choice::new("bug", "Something broke")],
                move |topic, message, _, cx| say(&owner, format!("{topic}: {message}"), cx),
            )
            .address("support-and-billing-questions@example-company.com"),
        )
        .into_any_element()
}

/// A long address gives way in the middle, so it ends inside a 280px box, which starts 16px in.
#[gpui::test]
fn a_long_address_is_cut_inside_a_narrow_box(cx: &mut TestAppContext) {
    let (_, cx) = bench(long_address, cx);
    let address = cx.debug_bounds("support-address").expect("the address");
    assert!(
        address.right() <= px(296.0),
        "the address ends inside the box: {address:?}"
    );
}

fn inline(_: &Bench, owner: Entity<Bench>) -> AnyElement {
    InlineHelp::new("hint", "Only admins can change billing.")
        .on_more(move |_, cx| say(&owner, "more".into(), cx))
        .into_any_element()
}

/// Stops: Learn more.
#[gpui::test]
fn inline_help_offers_more(cx: &mut TestAppContext) {
    let (host, cx) = bench(inline, cx);
    tab(1, cx);
    tap("space", cx);
    assert_eq!(said(&host, cx), ["more"]);
}

#[gpui::test]
fn an_article_that_left_draws(cx: &mut TestAppContext) {
    let (host, cx) = bench(panel, cx);
    host.update(cx, |bench, cx| {
        bench.open = Some("gone".into());
        cx.notify();
    });
    cx.run_until_parked();
}

/// Support topics, the second leaving once the bench hides it.
fn dynamic_topics(bench: &Bench, owner: Entity<Bench>) -> AnyElement {
    let mut topics = vec![Choice::new("billing", "Billing")];
    if !bench.hidden {
        topics.push(Choice::new("bug", "Something broke"));
    }
    ContactSupport::new("support", topics, move |topic, message, _, cx| {
        say(&owner, format!("{topic}: {message}"), cx)
    })
    .into_any_element()
}

#[gpui::test]
fn a_topic_that_left_sends_nothing(cx: &mut TestAppContext) {
    let (host, cx) = bench(dynamic_topics, cx);
    tab(1, cx);
    for key in ["enter", "down", "enter"] {
        tap(key, cx);
    }
    tab(2, cx);
    write("It froze", cx);
    edit(&host, cx, |bench| bench.hidden = true);
    tab(3, cx);
    tap("space", cx);
    assert_eq!(said(&host, cx), Vec::<String>::new());
}
