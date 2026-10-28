use super::*;
use crate::{i18n, settings::Language};

#[test]
fn folders_under_home_read_from_a_tilde() {
    let home = PathBuf::from("/Users/ely");
    let shown = |folder: &str| shown(Path::new(folder), Some(&home));
    assert_eq!(shown("/Users/ely"), "~");
    assert_eq!(
        shown("/Users/ely/src/app"),
        format!("~{MAIN_SEPARATOR}src/app")
    );
    assert_eq!(
        shown("/Users/elyse"),
        "/Users/elyse",
        "a sibling is no child"
    );
    assert_eq!(shown("/tmp"), "/tmp");
    assert_eq!(
        shown("/Users/ely/line\nbreak"),
        format!("~{MAIN_SEPARATOR}line␊break")
    );
}

#[test]
fn control_characters_show_as_their_pictures() {
    assert_eq!(one_line("a\nb\tc\rd\0e\u{7f}"), "a␊b␉c␍d␀e␡");
    assert_eq!(one_line("café ✓"), "café ✓", "printable text stays");
}

#[test]
fn closing_shows_the_next_session_or_keeps_the_shown_one() {
    assert_eq!(after_close(2, 0, 3), 1, "one before it went");
    assert_eq!(after_close(1, 1, 3), 1, "it went: the next one");
    assert_eq!(after_close(3, 3, 3), 2, "the last went: the one before");
    assert_eq!(after_close(1, 2, 3), 1, "one after it went");
}

#[test]
fn the_mark_shows_the_most_pressing_look() {
    let record = |state, progress| Record {
        state,
        kind: None,
        progress,
        app: None,
        title: None,
        msg: None,
    };
    let waiting = look(Some(&record(State::Blocked, None)), true);
    assert_eq!(
        waiting,
        Some(Look::Waiting),
        "waiting outranks a failed command"
    );
    let working = look(Some(&record(State::Working, Some(40))), false);
    assert_eq!(working, Some(Look::Working(Some(40))));
    let idle = record(State::Idle, None);
    assert_eq!(
        look(Some(&idle), true),
        Some(Look::Error),
        "a failed command outranks idle"
    );
    assert_eq!(look(Some(&idle), false), Some(Look::Idle));
    assert_eq!(
        look(Some(&record(State::Done, None)), true),
        Some(Look::Done)
    );
    assert_eq!(
        look(Some(&record(State::Error, None)), false),
        Some(Look::Error)
    );
    assert_eq!(look(None, false), None);
}

#[test]
fn hover_shows_a_program_s_text_as_text() {
    assert_eq!(plain("evil\u{202e}txt.exe"), "eviltxt.exe");
    assert_eq!(
        plain("a\u{200b}b\u{feff}c\u{e0041}d\u{85}e\u{2066}f"),
        "abcdef"
    );
    assert_eq!(
        plain("👨\u{200d}💻 two\nlines"),
        "👨\u{200d}💻 two␊lines",
        "joiners stay"
    );
    assert_eq!(plain(&"x".repeat(SHORT)), "x".repeat(SHORT));
    assert_eq!(
        plain(&"é".repeat(SHORT + 1)),
        format!("{}…", "é".repeat(SHORT))
    );
}

#[test]
fn hover_lists_each_record_with_its_app_then_a_failed_command() {
    let record =
        |state: State, kind: Option<Kind>, progress: Option<u8>, texts: [Option<&str>; 3]| {
            let [app, title, msg] = texts.map(|text| text.map(String::from));
            Record {
                state,
                kind,
                progress,
                app,
                title,
                msg,
            }
        };
    let mut records = Records::default();
    records.report(
        String::new(),
        record(
            State::Working,
            None,
            Some(40),
            [Some("deploy"), None, Some("Deploying")],
        ),
        None,
    );
    records.report(
        "eu".into(),
        record(
            State::Blocked,
            Some(Kind::Permission),
            None,
            [None, Some("EU West"), Some("Approve?")],
        ),
        None,
    );
    records.report(
        "eu/db".into(),
        record(State::Done, None, None, [Some("db"), None, None]),
        None,
    );
    let english = i18n::catalogs(Language::English);
    let lines: Vec<String> = hover(&records, Some(2), &english)
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        lines,
        [
            "Working, 40%: deploy · Deploying",
            "Waiting for permission: deploy · EU West · Approve?",
            "Done: db",
            "Last command exited with 2"
        ]
    );
    let chinese = i18n::catalogs(Language::SimplifiedChinese);
    let lines = hover(&records, Some(2), &chinese);
    assert_eq!(lines[0].as_ref(), "工作中，40%：deploy · Deploying");
    assert_eq!(lines[1].as_ref(), "等待授权：deploy · EU West · Approve?");
    assert_eq!(lines[3].as_ref(), "上一条命令以 2 退出");
    for at in 0..LISTED {
        records.report(
            format!("r{at}"),
            record(State::Idle, None, None, [None; 3]),
            None,
        );
    }
    let lines = hover(&records, None, &english);
    assert_eq!(lines.len(), LISTED + 1);
    assert_eq!(lines[LISTED].to_string(), "3 more");
}
