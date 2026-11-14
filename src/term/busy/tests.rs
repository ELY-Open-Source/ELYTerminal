use std::{
    thread,
    time::{Duration, Instant},
};

use super::running;
use crate::{
    settings::Settings,
    term::{Launch, Pty, Quote, config, spawn},
};

fn launched(program: &str, args: &[&str]) -> Pty {
    crate::testlog::init();
    let launch = Launch {
        program: Some((
            program.into(),
            args.iter().map(|arg| arg.to_string()).collect(),
        )),
        cwd: std::env::temp_dir(),
        env: Vec::new(),
        reports: false,
        quote: if cfg!(windows) {
            Quote::Cmd
        } else {
            Quote::Posix
        },
    };
    let (pty, _events) =
        spawn(&launch, (80, 24), (8, 16), config(&Settings::default())).expect("the shell starts");
    pty
}

/// What the terminal runs within 10 seconds, once it runs something.
fn first_run(pty: &Pty) -> Option<String> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        if let Some(name) = running(pty).expect("the foreground reads") {
            return Some(name);
        }
        thread::sleep(Duration::from_millis(50));
    }
    None
}

#[test]
fn a_program_the_shell_runs_is_running() {
    #[cfg(unix)]
    let (pty, want) = (launched("/bin/sh", &["-c", "set -m; sleep 20; :"]), "sleep");
    #[cfg(windows)]
    let (pty, want) = (
        launched("cmd.exe", &["/c", "ping -n 20 127.0.0.1 >nul"]),
        "PING.EXE",
    );
    let name = first_run(&pty).expect("the program runs");
    assert!(name.eq_ignore_ascii_case(want), "{name}");
}

#[test]
fn a_program_the_terminal_runs_itself_is_running() {
    #[cfg(unix)]
    let (pty, want) = (launched("/bin/sleep", &["20"]), "sleep");
    #[cfg(windows)]
    let (pty, want) = (launched("PING.EXE", &["-n", "20", "127.0.0.1"]), "PING.EXE");
    let name = first_run(&pty).expect("the program runs");
    assert!(name.eq_ignore_ascii_case(want), "{name}");
}

#[cfg(unix)]
#[test]
fn a_shell_beside_its_job_is_running() {
    let pty = launched("/bin/sh", &["-c", "sleep 20; :"]);
    let name = first_run(&pty).expect("the shell runs its job");
    assert!(super::shell(name.as_ref()), "{name}");
}

#[test]
fn a_shell_waiting_for_a_line_runs_nothing() {
    #[cfg(unix)]
    let pty = launched("/bin/sh", &["-c", "set -m; read line"]);
    #[cfg(windows)]
    let pty = launched("cmd.exe", &["/c", "set /p line="]);
    thread::sleep(Duration::from_secs(1));
    for _ in 0..10 {
        assert_eq!(running(&pty).expect("the foreground reads"), None);
        thread::sleep(Duration::from_millis(50));
    }
}
