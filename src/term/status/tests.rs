use super::*;
use crate::term::{
    marks::{Click, Mark},
    notify::Progress,
};

fn encoded(text: &[u8]) -> String {
    BASE64.encode(text)
}

/// What `body` says when its sequence is as long as its frame makes it.
fn said(body: &[u8]) -> Option<Status> {
    read(body, body.len() + 9)
}

fn record(state: State, app: Option<&str>) -> Record {
    Record {
        state,
        kind: None,
        progress: None,
        app: app.map(str::to_string),
        title: None,
        msg: None,
    }
}

fn reported(body: &str) -> Record {
    match said(body.as_bytes()) {
        Some(Status::Report(_, record)) => record,
        other => panic!("{body}: {other:?}"),
    }
}

#[test]
fn the_spec_s_examples_read_as_it_means_them() {
    let body = b"state=blocked:kind=permission:app=terraform:msg=QXBwbHkgMyB0byBhZGQsIDEgdG8gY2hhbmdlLCAwIHRvIGRlc3Ryb3k/";
    let blocked = Record {
        kind: Some(Kind::Permission),
        msg: Some("Apply 3 to add, 1 to change, 0 to destroy?".into()),
        ..record(State::Blocked, Some("terraform"))
    };
    assert_eq!(said(body), Some(Status::Report(String::new(), blocked)));
    let body = b"state=working:id=us-east:title=VVMgRWFzdA==:progress=40:msg=UHVzaGluZyBpbWFnZQ==";
    let working = Record {
        progress: Some(40),
        title: Some("US East".into()),
        msg: Some("Pushing image".into()),
        ..record(State::Working, None)
    };
    assert_eq!(said(body), Some(Status::Report("us-east".into(), working)));
    assert_eq!(
        said(b"state=clear:id=us-east"),
        Some(Status::Clear(Some("us-east".into())))
    );
    assert_eq!(said(b"state=clear"), Some(Status::Clear(None)));
    assert_eq!(said(b"?"), Some(Status::Query));
    assert_eq!(
        reported("state=done:msg=VXBncmFkZWQgMTIgcGFja2FnZXM")
            .msg
            .as_deref(),
        Some("Upgraded 12 packages"),
        "padding is optional"
    );
}

#[test]
fn a_malformed_pair_is_skipped_the_last_of_a_key_wins_and_unknown_keys_pass() {
    let record = reported(" junk : =x : state = idle :Key=v:app=a b:color=red:state=done:app=ok ");
    assert_eq!(record, self::record(State::Done, Some("ok")));
}

#[test]
fn a_sequence_that_breaks_a_rule_drops_whole() {
    let long = |key: &str, length: usize| format!("state=done:{key}={}", "a".repeat(length));
    let drops = [
        "app=x".to_string(),
        "state=sleeping".into(),
        "state=done:id=".into(),
        "state=done:id=a//b".into(),
        "state=done:id=a/b/c/d/e/f/g/h/i".into(),
        format!("state=done:id={}", "a".repeat(33)),
        format!("state=done:{}=1", "k".repeat(17)),
        long("app", 33),
        long("title", 257),
        long("msg", 2733),
        "state=done:msg=A".into(),
        format!("state=done:msg={}", encoded(b"a\nb")),
        format!("state=done:msg={}", encoded("a\u{85}b".as_bytes())),
        format!("state=done:msg={}", encoded(&[0xff])),
        format!("state=done:title={}", encoded(&[b'a'; 193])),
        format!("state=done:x={}", "a".repeat(4096 - 9 - 12)),
    ];
    for body in drops {
        assert_eq!(said(body.as_bytes()), None, "{body}");
    }
    assert!(said(format!("state=done:x={}", "a".repeat(4096 - 9 - 14)).as_bytes()).is_some());
}

#[test]
fn kind_progress_and_app_count_only_where_the_spec_has_them() {
    let done = reported("state=done:kind=auth:progress=5");
    assert_eq!((done.kind, done.progress), (None, None));
    let blocked = reported("state=blocked:kind=nope:progress=101");
    assert_eq!((blocked.kind, blocked.progress), (None, None));
    assert_eq!(reported("state=working:progress=007").progress, Some(7));
    assert_eq!(reported("state=working:progress=+5").progress, None);
    assert_eq!(reported("state=blocked:kind=auth").kind, Some(Kind::Auth));
    assert_eq!(
        reported("state=working:app=a.b+c_d-e").app.as_deref(),
        Some("a.b+c_d-e")
    );
    assert_eq!(reported("state=working:app=a,b").app, None);
}

#[test]
fn records_replace_and_clear_as_the_spec_says() {
    let mut records = Records::default();
    records.report(String::new(), record(State::Working, Some("deploy")), None);
    records.report("us-east".into(), record(State::Working, None), None);
    records.report("us-east/push".into(), record(State::Blocked, None), None);
    records.report("us-easter".into(), record(State::Done, Some("other")), None);
    records.report("us-east".into(), record(State::Done, None), None);
    let ids: Vec<&str> = records.list.iter().map(|(id, ..)| id.as_str()).collect();
    assert_eq!(ids, ["", "us-east", "us-east/push", "us-easter"]);
    let east = records
        .list
        .iter()
        .find(|(id, ..)| id == "us-east")
        .map(|(_, record, ..)| record.clone());
    assert_eq!(
        east,
        Some(record(State::Done, None)),
        "a report replaces its record whole"
    );
    records.clear(Some("us-east"));
    let ids: Vec<&str> = records.list.iter().map(|(id, ..)| id.as_str()).collect();
    assert_eq!(
        ids,
        ["", "us-easter"],
        "clear takes the record and those under it"
    );
    records.clear(None);
    assert!(records.list.is_empty());
}

#[test]
fn herdr_s_agents_replace_their_own_records_alone() {
    let mut records = Records::default();
    records.report("build".into(), record(State::Working, None), None);
    // A program's id ends at a colon, so `herdr:w1:p1` names its record `herdr`.
    for body in [
        &b"state=done:id=herdr/build"[..],
        b"state=done:id=herdr:w1:p1",
    ] {
        let Some(Status::Report(id, done)) = said(body) else {
            panic!("a program's report");
        };
        records.report(id, done, None);
    }
    let ids = |records: &Records| -> Vec<String> {
        records.list.iter().map(|(id, ..)| id.clone()).collect()
    };
    records.herdr(&[]);
    assert_eq!(
        ids(&records),
        ["build", "herdr/build", "herdr"],
        "an empty snapshot takes no program's record"
    );
    let codex = [(
        "herdr:w1:p1".to_string(),
        record(State::Working, Some("codex")),
    )];
    records.herdr(&codex);
    let clock = records.clock;
    records.herdr(&codex);
    assert_eq!(records.clock, clock, "the same agents change nothing");
    let claude = [(
        "herdr:w1:p2".to_string(),
        record(State::Blocked, Some("claude")),
    )];
    records.herdr(&claude);
    assert_eq!(
        ids(&records),
        ["build", "herdr/build", "herdr", "herdr:w1:p2"]
    );
    records.herdr(&[]);
    assert_eq!(
        ids(&records),
        ["build", "herdr/build", "herdr"],
        "herdr's leaving takes its own alone"
    );
    records.reset();
    records.herdr(&claude);
    assert_eq!(
        ids(&records),
        ["herdr:w1:p2"],
        "the next snapshot puts them back"
    );
}

/// The records' ids, oldest first.
fn kept(records: &Records) -> Vec<&str> {
    records.list.iter().map(|(id, ..)| id.as_str()).collect()
}

/// omp's process group.
const OMP: i32 = 200;

/// The shell's own process group.
const SHELL: i32 = 100;

#[test]
fn a_shell_s_prompt_leaves_done_and_error_though_programs_reported() {
    let mut records = Records::default();
    for (id, state) in [
        ("a", State::Idle),
        ("b", State::Working),
        ("c", State::Done),
        ("d", State::Blocked),
        ("e", State::Error),
    ] {
        records.report(id.into(), record(state, None), Some(OMP));
    }
    records.mark(&Mark::Prompt(Click::Arrows), Holder::Shell);
    assert_eq!(kept(&records), ["c", "e"]);
}

#[test]
fn a_program_s_own_prompts_keep_what_it_reported_while_it_holds_the_terminal() {
    let mut records = Records::default();
    // omp reports work, then marks messages A, B, C, D;0.
    records.report(
        String::new(),
        record(State::Working, Some("omp")),
        Some(OMP),
    );
    for _ in 0..3 {
        for mark in [
            Mark::Prompt(Click::Arrows),
            Mark::Input,
            Mark::Output,
            Mark::Done(Some(0)),
        ] {
            records.mark(&mark, Holder::Group(OMP));
        }
    }
    assert_eq!(kept(&records), [""]);
    // omp ends, even mid-command; the shell's named prompt follows.
    records.mark(&Mark::Prompt(Click::Arrows), Holder::Shell);
    assert!(kept(&records).is_empty());
}

#[test]
fn an_unnamed_prompt_drops_what_a_group_no_longer_holding_the_terminal_reported() {
    // fish's own prompt after omp died, or a nested shell's.
    let mut records = Records::default();
    records.report("a".into(), record(State::Working, None), Some(OMP));
    records.mark(&Mark::Prompt(Click::Arrows), Holder::Group(SHELL));
    assert!(kept(&records).is_empty());
}

#[test]
fn a_shell_without_job_control_keeps_its_program_s_work_until_it_names_itself() {
    // PowerShell's program shares its group, so its holder is unknown.
    let mut records = Records::default();
    records.mark(&Mark::Output, Holder::Unknown);
    records.report(String::new(), record(State::Working, None), Some(SHELL));
    records.mark(&Mark::Prompt(Click::Arrows), Holder::Unknown);
    assert_eq!(kept(&records), [""]);
    records.mark(&Mark::Done(Some(0)), Holder::Unknown);
    records.mark(&Mark::Prompt(Click::Arrows), Holder::Shell);
    assert!(kept(&records).is_empty());
}

#[test]
fn a_named_prompt_closes_commands_left_open() {
    let mut records = Records::default();
    // A program died between its C and D.
    records.mark(&Mark::Output, Holder::Unknown);
    records.mark(&Mark::Output, Holder::Unknown);
    records.mark(&Mark::Done(Some(130)), Holder::Unknown);
    records.mark(&Mark::Prompt(Click::Arrows), Holder::Shell);
    records.report("a".into(), record(State::Working, None), None);
    records.mark(&Mark::Prompt(Click::Arrows), Holder::Unknown);
    assert!(kept(&records).is_empty());
}

#[test]
fn without_a_holder_a_prompt_inside_a_running_command_keeps_the_records() {
    // Windows has no foreground group: C and D count commands.
    let mut records = Records::default();
    records.mark(&Mark::Output, Holder::Unknown);
    records.report(String::new(), record(State::Working, Some("omp")), None);
    for mark in [
        Mark::Prompt(Click::Arrows),
        Mark::Input,
        Mark::Output,
        Mark::Done(Some(0)),
        Mark::Prompt(Click::Arrows),
    ] {
        records.mark(&mark, Holder::Unknown);
    }
    assert_eq!(kept(&records), [""]);
    records.mark(&Mark::Done(Some(0)), Holder::Unknown);
    records.mark(&Mark::Prompt(Click::Arrows), Holder::Unknown);
    assert!(kept(&records).is_empty());
}

#[test]
fn a_full_reset_closes_a_command_left_open() {
    let mut records = Records::default();
    records.mark(&Mark::Output, Holder::Unknown);
    records.reset();
    records.report("a".into(), record(State::Working, None), None);
    records.mark(&Mark::Prompt(Click::Arrows), Holder::Unknown);
    assert!(kept(&records).is_empty());
}

#[test]
fn past_the_cap_the_record_updated_longest_ago_goes() {
    let mut records = Records::default();
    for at in 0..RECORDS {
        records.report(format!("r{at}"), record(State::Working, None), None);
    }
    records.report("r0".into(), record(State::Done, None), None);
    records.report("new".into(), record(State::Working, None), None);
    assert_eq!(records.list.len(), RECORDS);
    assert!(
        records.list.iter().all(|(id, ..)| id != "r1"),
        "r1 was updated longest ago"
    );
    assert!(records.list.iter().any(|(id, ..)| id == "r0"));
}

#[test]
fn a_clear_that_breaks_a_rule_drops_whole() {
    let drops = [
        "state=clear:msg=Cg==".to_string(),
        "state=clear:msg=A".into(),
        format!("state=clear:app={}", "a".repeat(33)),
        format!("state=clear:title={}", "a".repeat(257)),
        "state=clear:id=a//b".into(),
    ];
    for body in drops {
        assert_eq!(said(body.as_bytes()), None, "{body}");
    }
}

#[test]
fn every_pair_counts_though_the_last_of_a_key_wins() {
    let drops = [
        format!("state=done:app={}:app=ok", "a".repeat(33)),
        "state=done:title=A:title=VVMgRWFzdA==".into(),
        "state=done:id=a//b:id=ok".into(),
        format!("state=done:msg={}:msg=b2s=", encoded(b"a\nb")),
    ];
    for body in drops {
        assert_eq!(said(body.as_bytes()), None, "{body}");
    }
    assert_eq!(reported("state=done:app=x:app=y").app.as_deref(), Some("y"));
    assert_eq!(
        reported("state=done:app=a,b:app=ok").app.as_deref(),
        Some("ok")
    );
}

#[test]
fn the_sequence_counts_from_osc_through_st() {
    assert_eq!(read(b"state=done", 4097), None);
    assert!(read(b"state=done", 4096).is_some());
}

#[test]
fn osc_9_4_maps_to_the_root_record_until_a_program_reports() {
    let root = |records: &Records| {
        records
            .list
            .iter()
            .find(|(id, ..)| id.is_empty())
            .map(|(_, record, ..)| (record.state, record.progress))
    };
    let mut records = Records::default();
    records.progress(Progress::Set(Some(40)), None);
    assert_eq!(root(&records), Some((State::Working, Some(40))));
    records.progress(Progress::Pause(Some(5)), None);
    assert_eq!(root(&records), Some((State::Working, Some(5))));
    records.progress(Progress::Indeterminate, None);
    assert_eq!(root(&records), Some((State::Working, None)));
    records.progress(Progress::Error(Some(9)), None);
    assert_eq!(root(&records), Some((State::Error, None)));
    records.progress(Progress::Remove, None);
    assert_eq!(root(&records), None);
    records.report("a".into(), record(State::Idle, None), None);
    records.progress(Progress::Set(Some(1)), None);
    assert_eq!(root(&records), None, "a program reports through OSC 7501");
    records.reset();
    assert!(records.list.is_empty());
    records.progress(Progress::Set(Some(2)), None);
    assert_eq!(
        root(&records),
        Some((State::Working, Some(2))),
        "a full reset maps again"
    );
}

#[test]
fn a_row_shows_the_most_pressing_record_the_last_reported_in_its_state() {
    let mut records = Records::default();
    assert_eq!(records.shown(), None);
    let shown = |records: &Records| {
        records
            .shown()
            .map(|(_, record)| (record.state, record.kind))
    };
    records.report("a".into(), record(State::Idle, None), None);
    assert_eq!(shown(&records), Some((State::Idle, None)));
    records.report("b".into(), record(State::Done, None), None);
    assert_eq!(shown(&records), Some((State::Done, None)));
    records.report("c".into(), record(State::Working, None), None);
    assert_eq!(shown(&records), Some((State::Working, None)));
    records.report("d".into(), record(State::Error, None), None);
    assert_eq!(shown(&records), Some((State::Error, None)));
    let asks = |kind| Record {
        kind: Some(kind),
        ..record(State::Blocked, None)
    };
    records.report("e".into(), asks(Kind::Auth), None);
    records.report("f".into(), asks(Kind::Question), None);
    assert_eq!(
        shown(&records),
        Some((State::Blocked, Some(Kind::Question)))
    );
    records.report("e".into(), asks(Kind::Auth), None);
    assert_eq!(
        shown(&records),
        Some((State::Blocked, Some(Kind::Auth))),
        "the last reported speaks"
    );
}

#[test]
fn a_record_without_an_app_takes_its_nearest_ancestor_s() {
    let mut records = Records::default();
    records.report("us/east".into(), record(State::Working, None), None);
    assert_eq!(records.app("us/east"), None);
    records.report(String::new(), record(State::Working, Some("deploy")), None);
    records.report("us".into(), record(State::Working, Some("push")), None);
    records.report("us/east/a".into(), record(State::Working, None), None);
    records.report("eu/west".into(), record(State::Working, None), None);
    assert_eq!(
        records.app("us/east/a"),
        Some("push"),
        "past a parent without one"
    );
    assert_eq!(records.app("eu/west"), Some("deploy"), "the root's last");
    assert_eq!(records.app(""), Some("deploy"));
    assert_eq!(
        records.app("usa"),
        Some("deploy"),
        "a sibling's name is no parent"
    );
    records.herdr(&[("herdr:w1:p1".into(), record(State::Working, None))]);
    assert_eq!(
        records.app("herdr:w1:p1"),
        None,
        "herdr's agents are no program's children"
    );
}

#[test]
fn typing_sees_done_and_error_but_herdr_s() {
    let mut records = Records::default();
    records.report("a".into(), record(State::Done, None), None);
    records.report("b".into(), record(State::Error, None), None);
    records.report("c".into(), record(State::Working, None), None);
    assert!(records.herdr(&[("herdr:w1:p1".into(), record(State::Done, None))]));
    assert!(records.seen());
    let ids: Vec<&str> = records.iter().map(|(id, _)| id).collect();
    assert_eq!(ids, ["c", "herdr:w1:p1"]);
    assert!(!records.seen(), "nothing more to see");
}

#[test]
fn each_change_of_the_state_a_row_shows_is_noted() {
    let mut records = Records::default();
    assert!(!records.restate(), "none shows yet");
    records.report(String::new(), record(State::Blocked, None), None);
    assert!(records.restate());
    records.report(String::new(), record(State::Working, None), None);
    assert!(records.restate());
    records.report(String::new(), record(State::Blocked, None), None);
    assert!(records.restate(), "waiting again after work in one output");
    assert_eq!(records.shows(), Some(State::Blocked));
    records.report("b".into(), record(State::Working, None), None);
    assert!(!records.restate(), "the row shows what it showed");
    records.reset();
    assert!(records.restate());
    assert_eq!(records.shows(), None);
}
