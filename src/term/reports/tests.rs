use super::*;
use crate::{i18n, settings::Language};

fn notice(id: Option<&str>, title: &str, body: &str) -> Notice {
    Notice {
        id: id.map(String::from),
        title: title.into(),
        body: body.into(),
        focus: true,
        report: false,
        close: false,
        expire: None,
    }
}

#[test]
fn a_session_posts_once_in_3_seconds_its_own_under_one_tag() {
    let mut notices = Notices::default();
    let start = Instant::now();
    let first = notices.turn(7, None, start).expect("the first post");
    assert_eq!((&*first.tag, first.focus, first.report), ("7", true, false));
    assert_eq!(notices.turn(7, None, start + Duration::from_secs(2)), None);
    let again = notices.turn(7, None, start + GAP).expect("past the gap");
    assert_eq!(&*again.tag, "7");
    assert_eq!(notices.posted.len(), 1, "the last takes its place");
}

#[test]
fn a_program_s_notifications_go_by_their_id_and_live_until_taken() {
    let mut notices = Notices::default();
    let start = Instant::now();
    let mut asked = notice(Some("build"), "T", "");
    asked.report = true;
    asked.focus = false;
    let posted = notices.turn(7, Some(&asked), start).expect("a post");
    assert_eq!(
        (&*posted.tag, posted.focus, posted.report),
        ("7:build", false, true)
    );
    let untagged = notices
        .turn(7, Some(&notice(None, "T", "")), start + GAP)
        .expect("a post");
    assert!(untagged.tag.starts_with("7#"), "{}", untagged.tag);
    assert_eq!(notices.alive(), ["build"]);
    let update = notices
        .turn(7, Some(&asked), start + GAP * 2)
        .expect("an update");
    assert!(
        notices.replaced(&posted.tag, posted.count),
        "a later post holds the tag"
    );
    assert!(!notices.replaced(&update.tag, update.count));
    assert_eq!(
        notices.take("7:build", Some(posted.count)),
        None,
        "an expiry takes its own post only"
    );
    assert_eq!(notices.take("7:build", Some(update.count)), Some(update));
    assert!(notices.alive().is_empty());
}

#[test]
fn a_session_keeps_its_last_64_notifications() {
    let mut notices = Notices::default();
    let start = Instant::now();
    for at in 0..=POSTED as u32 {
        let asked = notice(Some(format!("n{at}").as_str()), "T", "");
        notices
            .turn(7, Some(&asked), start + GAP * at)
            .expect("a post");
    }
    assert_eq!(notices.posted.len(), POSTED);
    assert!(!notices.alive().contains(&"n0"), "the oldest went");
    assert!(notices.alive().contains(&"n64"));
}

#[test]
fn a_notification_says_its_text_as_text() {
    let asked = notice(None, "Build\u{202e} done", "Line 1\n\nLine 2\u{200b}");
    assert_eq!(said(&asked), "Build done\nLine 1\nLine 2");
    let english = i18n::catalogs(Language::English);
    assert_eq!(finished(Some(0), &english), "Command finished");
    assert_eq!(finished(None, &english), "Command finished");
    assert_eq!(finished(Some(2), &english), "Command exited with 2");
    let chinese = i18n::catalogs(Language::SimplifiedChinese);
    assert_eq!(finished(Some(2), &chinese), "命令以 2 退出");
}

#[test]
fn a_command_posts_its_end_once_it_ran_5_seconds() {
    let start = Instant::now();
    assert!(!ran_long(start, start), "no time ran");
    let under = start + Duration::from_secs(5) - Duration::from_nanos(1);
    assert!(!ran_long(start, under), "just under stays quiet");
    assert!(
        ran_long(start, start + Duration::from_secs(5)),
        "the boundary tells"
    );
}

#[test]
fn a_session_posts_unless_it_is_the_one_shown_in_an_active_window() {
    assert!(!unseen(true, true));
    assert!(unseen(true, false), "the window inactive");
    assert!(unseen(false, true), "the session hidden");
    assert!(unseen(false, false));
}

#[test]
fn waiting_done_and_error_post_and_work_and_rest_do_not() {
    for (state, posts) in [
        (State::Blocked, true),
        (State::Done, true),
        (State::Error, true),
        (State::Working, false),
        (State::Idle, false),
    ] {
        assert_eq!(tells(state), posts, "{state:?}");
    }
}

#[test]
fn a_tag_names_the_session_that_posted_it() {
    for tag in ["12", "12:build", "12#3"] {
        assert!(owns(12, tag), "{tag}");
    }
    for tag in ["123", "1", "123:build", "13#3", ""] {
        assert!(!owns(12, tag), "{tag}");
    }
}

#[test]
fn a_body_goes_as_markup_s_text() {
    assert_eq!(
        escape("<b>a</b> & <a href=x>"),
        "&lt;b&gt;a&lt;/b&gt; &amp; &lt;a href=x&gt;"
    );
    assert_eq!(escape("plain"), "plain");
}
