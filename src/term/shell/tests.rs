use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

use alacritty_terminal::{event::Event, grid::Dimensions, index::Line};
use futures::channel::mpsc::TryRecvError;

use super::*;
use crate::{
    settings::Settings,
    term::{
        Events, Pty, PtyEvent, config,
        marks::{Click, Mark},
        spawn,
    },
};

mod start;

/// A folder name with a space, a percent, a bell and a newline.
const INNER: &str = "in ner%\u{7}x\ny";

/// A fresh folder under the temp dir, by its real path.
fn folder(name: &str) -> PathBuf {
    let folder = env::temp_dir().join(format!("elyterminal-{name}-{}", std::process::id()));
    if folder.exists() {
        fs::remove_dir_all(&folder).expect("a stale test folder goes");
    }
    fs::create_dir_all(&folder).expect("a test folder");
    folder.canonicalize().expect("a real path")
}

fn installed(shell: &str) -> String {
    let found = std::process::Command::new("/bin/sh")
        .args(["-c", &format!("command -v {shell}")])
        .output()
        .expect("sh runs");
    assert!(found.status.success(), "{shell} is not installed");
    String::from_utf8(found.stdout)
        .expect("utf-8")
        .trim()
        .to_string()
}

/// A shell started with integration under a fake HOME, and what it reported.
struct Run {
    shell: String,
    pty: Pty,
    events: Events,
    root: PathBuf,
    start: PathBuf,
    folders: Vec<PathBuf>,
    titles: Vec<String>,
    marks: Vec<Mark>,
    /// Lines typed, each followed by a prompt of its own.
    lines: usize,
    deadline: Instant,
}

impl Run {
    /// Starts `shell` in a fresh folder; `files` go under its HOME first.
    fn start(shell: &str, files: &[(&str, &str)]) -> Self {
        Self::with(shell, files, Vec::new())
    }

    /// As `start`, with `env` on top.
    fn with(shell: &str, files: &[(&str, &str)], env: Vec<(String, String)>) -> Self {
        crate::testlog::init();
        // Two tests may run one shell at once.
        static RUNS: AtomicUsize = AtomicUsize::new(0);
        let root = folder(&format!(
            "shell-{}-{}",
            shell.rsplit('/').next().expect("a name"),
            RUNS.fetch_add(1, Ordering::Relaxed)
        ));
        let (dir, home, start) = (
            root.join("integration"),
            root.join("home"),
            root.join("start"),
        );
        fs::create_dir_all(start.join(INNER)).expect("the inner folder");
        fs::create_dir_all(&home).expect("home");
        for (name, text) in files {
            let path = home.join(name);
            fs::create_dir_all(path.parent().expect("a folder")).expect("a home folder");
            fs::write(path, text).expect("a home file");
        }
        install(&dir).expect("scripts install");
        let mut launch = launch(shell, start.clone(), &dir).expect("a launch");
        launch.env.push(("HOME".into(), home.display().to_string()));
        launch.env.extend(env);
        let (pty, events) = spawn(&launch, (100, 30), (8, 16), config(&Settings::default()))
            .expect("the shell starts");
        Self {
            shell: shell.into(),
            pty,
            events,
            root,
            start,
            folders: Vec::new(),
            titles: Vec::new(),
            marks: Vec::new(),
            lines: 0,
            deadline: Instant::now() + Duration::from_secs(30),
        }
    }

    /// Reads events, answering the shell's terminal queries, until `done` holds.
    fn until(&mut self, what: &str, done: impl Fn(&Self) -> bool) {
        while !done(self) {
            assert!(
                Instant::now() < self.deadline,
                "{}: no {what}; folders {:?}, titles {:?}, marks {:?}, screen\n{}",
                self.shell,
                self.folders,
                self.titles,
                self.marks,
                self.screen()
            );
            match self.events.try_recv() {
                Ok(PtyEvent::Folder(folder)) => self.folders.push(folder),
                Ok(PtyEvent::Title(title)) => self.titles.push(title),
                Ok(PtyEvent::Mark(mark, _)) => self.marks.push(mark),
                Ok(PtyEvent::Term(Event::PtyWrite(reply))) => self.pty.write(reply.into_bytes()),
                Ok(PtyEvent::Term(Event::ChildExit(status))) => {
                    panic!("{} ended, {status}:\n{}", self.shell, self.screen())
                }
                Ok(_) => {}
                Err(TryRecvError::Closed) => panic!("{}'s events ended", self.shell),
                Err(TryRecvError::Empty) => std::thread::sleep(Duration::from_millis(10)),
            }
        }
    }

    /// Types `line` once the line editor reads raw keys.
    fn send(&mut self, line: &str) {
        self.until("line editor", |run| run.pty.reads_keys());
        self.pty.write(format!("{line}\r").into_bytes());
        self.lines += 1;
    }

    fn shows(&self, text: &str) -> bool {
        self.screen().contains(text)
    }

    fn screen(&self) -> String {
        let term = self.pty.term.lock();
        (0..term.screen_lines())
            .map(|line| {
                let row = &term.grid()[Line(line as i32)];
                let text: String = row.into_iter().map(|cell| cell.c).collect();
                text.trim_end().to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The prompt's folder, the script loaded by `probe`, then a cd into the inner folder.
    fn walk(&mut self, probe: &str) {
        let first = [Mark::Prompt(Click::Arrows), Mark::Input];
        self.until("the first prompt's marks", |run| {
            run.marks.len() >= 2 && ends_like(&run.marks[..2], &first)
        });
        let start = self.start.clone();
        self.send(probe);
        self.until("first folder", |run| run.folders.contains(&start));
        assert_eq!(self.folders[0], start);
        self.until("loaded script", |run| run.shows("ELY-LOADED"));
        self.send("cd in*");
        let inner = start.join(INNER);
        self.until("inner folder", |run| run.folders.contains(&inner));
    }

    /// Types `line` once the prompt before it is marked, then waits for the marks it ends with.
    fn ends(&mut self, line: &str, marks: &[Mark]) {
        let prompts = self.lines + 1;
        self.until("the prompt's marks", |run| {
            run.marks
                .iter()
                .filter(|mark| **mark == Mark::Input)
                .count()
                >= prompts
        });
        let from = self.marks.len();
        self.send(line);
        self.until(&format!("the marks of {line}"), |run| {
            ends_like(&run.marks[from..], marks)
        });
    }

    fn finish(mut self) {
        self.send("exit");
        loop {
            assert!(
                Instant::now() < self.deadline,
                "{} did not exit",
                self.shell
            );
            match self.events.try_recv() {
                Ok(PtyEvent::Term(Event::ChildExit(_))) | Err(TryRecvError::Closed) => break,
                Ok(PtyEvent::Term(Event::PtyWrite(reply))) => self.pty.write(reply.into_bytes()),
                Ok(_) => {}
                Err(TryRecvError::Empty) => std::thread::sleep(Duration::from_millis(10)),
            }
        }
        let root = self.root.clone();
        drop(self);
        fs::remove_dir_all(&root).expect("cleanup");
    }
}

/// Whether this bash has PS0, 4.4 on, so it marks a command's start.
fn bash_ps0(shell: &str) -> bool {
    let version = std::process::Command::new(shell)
        .args(["-c", "echo $((BASH_VERSINFO[0] * 100 + BASH_VERSINFO[1]))"])
        .output()
        .expect("bash runs");
    let version: u32 = String::from_utf8(version.stdout)
        .expect("utf-8")
        .trim()
        .parse()
        .expect("a version");
    version >= 404
}

/// The marks a bash command that ends with `status` leaves; bash 3.2 marks no start.
fn bash_ended(ps0: bool, status: i32) -> Vec<Mark> {
    ended(status)[usize::from(!ps0)..].to_vec()
}

/// The marks a command that ends with `status` leaves: its output, its end, the next prompt.
fn ended(status: i32) -> [Mark; 4] {
    [
        Mark::Output,
        Mark::Done(Some(status)),
        Mark::Prompt(Click::Arrows),
        Mark::Input,
    ]
}

/// Whether `heard` ends as `wanted` does, a prompt's way of taking a click aside: fish from 4.1 takes clicks itself.
fn ends_like(heard: &[Mark], wanted: &[Mark]) -> bool {
    let Some(start) = heard.len().checked_sub(wanted.len()) else {
        return false;
    };
    heard[start..].iter().zip(wanted).all(|pair| match pair {
        (Mark::Prompt(_), Mark::Prompt(_)) => true,
        (heard, wanted) => heard == wanted,
    })
}

#[test]
fn the_scripts_install_once_and_mend_a_changed_one() {
    let dir = folder("install");
    install(&dir).expect("installs");
    let zshenv = dir.join("zsh/.zshenv");
    assert_eq!(fs::read_to_string(&zshenv).expect("written"), SCRIPTS[0].1);
    fs::write(&zshenv, [0xff, 0xfe]).expect("write");
    install(&dir).expect("installs again");
    assert_eq!(fs::read_to_string(&zshenv).expect("mended"), SCRIPTS[0].1);
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn a_script_it_cannot_read_fails_the_install() {
    use std::os::unix::fs::PermissionsExt;
    let dir = folder("unreadable");
    install(&dir).expect("installs");
    let bash = dir.join("bash/elyterminal.bash");
    fs::set_permissions(&bash, fs::Permissions::from_mode(0o200)).expect("200");
    let error = install(&dir).expect_err("an unreadable script fails");
    fs::set_permissions(&bash, fs::Permissions::from_mode(0o600)).expect("600");
    assert!(
        format!("{error:#}").contains("bash/elyterminal.bash"),
        "{error:#}"
    );
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn zsh_reports_its_folder_byte_for_byte_and_its_titles() {
    // A theme that sets PS1 in its own hook, which runs after ours.
    let theme = "setopt nounset\nautoload -Uz add-zsh-hook\n_theme() { PS1='THEME %? > ' }\nadd-zsh-hook precmd _theme";
    let mut run = Run::start(&installed("zsh"), &[(".zshrc", theme)]);
    run.walk("(( $+functions[_ely_precmd] )) && echo ELY-LOAD''ED");
    run.until("title", |run| {
        run.titles.iter().any(|title| title == "in ner%xy")
    });
    assert!(
        run.titles.contains(&"cd in*".to_string()),
        "{:?}",
        run.titles
    );
    run.ends("false", &ended(1));
    run.until("the theme's prompt", |run| {
        run.shows("THEME 0 > false") && run.shows("THEME 1 >")
    });
    run.send("false | true");
    run.send("echo PIPE=$pipestatus");
    run.until("the pipeline's statuses", |run| run.shows("PIPE=1 0"));
    run.finish();
}

#[test]
fn zsh_says_why_it_marks_no_prompt_without_prompt_percent() {
    let mut run = Run::start(&installed("zsh"), &[(".zshrc", "unsetopt prompt_percent")]);
    run.until("the warning", |run| {
        run.shows("ELYTerminal: zsh marks no prompt while prompt_percent is off")
    });
    run.send("false");
    let ended = [Mark::Output, Mark::Done(Some(1))];
    run.until("the command's marks", |run| run.marks.ends_with(&ended));
    let prompted = run.marks.iter().any(|mark| matches!(mark, Mark::Prompt(_)));
    assert!(!prompted, "{:?}", run.marks);
    run.finish();
}

#[test]
fn bash_reports_its_folder_byte_for_byte_and_keeps_the_status() {
    let status = r#"PROMPT_COMMAND='printf "USER_STATUS=%s\n" "$?"'; shopt -s extdebug"#;
    let files = [(".bashrc", status), (".bash_profile", status)];
    let shell = installed("bash");
    let ps0 = bash_ps0(&shell);
    let mut run = Run::start(&shell, &files);
    run.walk("declare -F _ely_prompt >/dev/null && echo ELY-LOAD''ED");
    run.until("title", |run| {
        run.titles.iter().any(|title| title == "in ner%xy")
    });
    run.ends("false", &bash_ended(ps0, 1));
    run.until("kept status", |run| run.shows("USER_STATUS=1"));
    assert!(run.shows("$ false"), "{}", run.screen());
    // Under extdebug a DEBUG trap that fails skips its command.
    run.ends("touch ran", &bash_ended(ps0, 0));
    assert!(run.start.join(INNER).join("ran").exists());
    run.ends("(exit 7)", &bash_ended(ps0, 7));
    run.send(r#"ls / >/dev/null; echo "LAST=$_""#);
    run.until("the last argument", |run| run.shows("LAST=/\n"));
    run.finish();
}

#[test]
fn bash_leaves_the_user_s_trap_and_keys_alone() {
    let rc = r#"_user_trap() { USER_TRAP=yes; }; trap '_user_trap "$_"' DEBUG
bind -x '"\C-t": :'"#;
    let files = [(".bashrc", rc), (".bash_profile", rc)];
    let shell = installed("bash");
    let ps0 = bash_ps0(&shell);
    let mut run = Run::start(&shell, &files);
    run.walk("declare -F _ely_prompt >/dev/null && echo ELY-LOAD''ED");
    run.ends("false", &bash_ended(ps0, 1));
    // A key binding runs no command, so it marks none.
    let from = run.marks.len();
    run.pty.write(b"\x14".to_vec());
    run.ends(r#"echo "TRAP=$USER_TRAP""#, &bash_ended(ps0, 0));
    assert!(run.shows("TRAP=yes"), "{}", run.screen());
    let starts = run.marks[from..]
        .iter()
        .filter(|mark| **mark == Mark::Output);
    assert_eq!(starts.count(), usize::from(ps0), "{:?}", &run.marks[from..]);
    run.finish();
}

#[test]
fn bash_says_why_it_marks_no_command_without_promptvars() {
    let files = [
        (".bashrc", "shopt -u promptvars"),
        (".bash_profile", "shopt -u promptvars"),
    ];
    let mut run = Run::start(&installed("bash"), &files);
    run.until("the warning", |run| {
        run.shows("ELYTerminal: bash marks no command while promptvars is off")
    });
    run.walk("declare -F _ely_prompt >/dev/null && echo ELY-LOAD''ED");
    run.ends("false", &[Mark::Prompt(Click::Arrows), Mark::Input]);
    run.finish();
}

#[test]
fn fish_loads_its_script_and_restores_its_data_dirs() {
    let mut run = Run::start(&installed("fish"), &[]);
    run.walk("functions -q _ely_prompt; and echo ELY-LOAD''ED");
    let previous = env::var("XDG_DATA_DIRS").unwrap_or_default();
    run.send(r#"echo "DIRS[$ELY_FISH_XDG_DATA_DIRS][$XDG_DATA_DIRS]""#);
    let restored = format!("DIRS[][{previous}]");
    run.until("restored dirs", |run| run.shows(&restored));
    // The prompt hears each command's own status, none failed so far.
    assert!(!run.shows("[1]"), "{}", run.screen());
    run.ends("false", &ended(1));
    run.until("the failed status", |run| run.shows("[1]>"));
    run.finish();
}

#[cfg(target_os = "macos")]
#[test]
fn powershell_reports_its_folder_byte_for_byte_and_keeps_the_status() {
    let prompt = r#"function global:prompt { "USER_SUCCESS=$? > " }"#;
    let files = [(
        ".config/powershell/Microsoft.PowerShell_profile.ps1",
        prompt,
    )];
    let mut run = Run::start(&installed("pwsh"), &files);
    run.walk("if ($global:ElyPrompt) { 'ELY-LOAD' + 'ED' }");
    run.until("title", |run| {
        run.titles.iter().any(|title| title == "in ner%xy")
    });
    run.ends("/usr/bin/false", &ended(1));
    run.until("kept status", |run| run.shows("USER_SUCCESS=False"));
    assert!(run.shows("> /usr/bin/false"), "{}", run.screen());
    // A cmdlet's failure leaves no error behind, a native code may repeat, and the line's last pipeline counts.
    run.ends("/usr/bin/true", &ended(0));
    run.ends("Get-Item ./missing -ErrorAction Ignore", &ended(1));
    run.ends("sh -c 'exit 3'", &ended(3));
    run.ends("sh -c 'exit 3'", &ended(3));
    run.ends("Get-Item ./missing", &ended(1));
    run.ends("sh -c 'exit 7'; Get-Item ./missing", &ended(1));
    run.ends(
        "sh -c 'exit 7'; Get-Item ./missing -ErrorAction Ignore",
        &ended(1),
    );
    run.ends("Get-Item ./missing; sh -c 'exit 7'", &ended(7));
    // A call through a variable runs a program the line does not name.
    run.send("$exe = 'sh'");
    run.ends("& $exe -c 'exit 7'", &ended(7));
    run.ends("& \"$exe\" -c 'exit 9'", &ended(9));
    let native = "$PSNativeCommandUseErrorActionPreference = $true; sh -c 'exit 7'";
    run.ends(native, &ended(7));
    run.finish();
}

#[test]
fn a_shell_named_bare_runs_from_its_full_path() {
    let started = launch("sh", PathBuf::from("/"), Path::new("/nonexistent")).expect("a launch");
    let (program, _) = started.program.expect("a program");
    assert_eq!(program, crate::settings::program_path("sh").expect("sh"));
    assert!(Path::new(&program).is_absolute(), "{program}");
    assert!(launch("no-such-shell", PathBuf::from("/"), Path::new("/")).is_err());
}

#[test]
fn an_unknown_shell_starts_without_integration() {
    let dir = PathBuf::from("/nonexistent");
    let launch = launch("/bin/sh", PathBuf::from("/"), &dir).expect("a launch");
    assert_eq!(launch.program, Some(("/bin/sh".into(), Vec::new())));
    assert!(launch.env.is_empty());
}

#[test]
fn each_shell_reads_a_dropped_path_its_own_way() {
    use std::os::unix::fs::PermissionsExt;
    let dir = folder("quote");
    for (name, quote) in [
        ("zsh", Quote::Posix),
        ("fish", Quote::Posix),
        ("pwsh", Quote::PowerShell),
        ("powershell", Quote::PowerShell),
        ("cmd", Quote::Cmd),
        ("other", Quote::Posix),
    ] {
        let program = dir.join(name);
        fs::write(&program, "").expect("a program");
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).expect("its mode");
        let shell = program.to_str().expect("utf-8");
        let launch = launch(shell, PathBuf::from("/"), &dir).expect("a launch");
        assert_eq!(launch.quote, quote, "{name}");
    }
}

#[test]
fn under_wsl_each_shell_reports_its_folder_as_windows_names_it() {
    // wslpath's stand-in names a folder under PREFIX: the share `\\wsl`, which Unix reads as `/wsl`, or a drive.
    let fake = folder("wslpath");
    let names = r#"printf '%s%s\n' "$PREFIX" "$(printf %s "$2" | tr / '\\')""#;
    fs::write(fake.join("wslpath"), format!("#!/bin/sh\n{names}\n")).expect("wslpath");
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(fake.join("wslpath"), fs::Permissions::from_mode(0o755)).expect("a mode");
    let path = format!("{}:{}", fake.display(), env::var("PATH").expect("PATH"));
    for (prefix, shell) in ["zsh", "bash", "fish"]
        .iter()
        .flat_map(|shell| [("\\\\wsl", shell), ("C:", shell)])
    {
        let wsl = [("ELY_WSL", "1"), ("PATH", &path), ("PREFIX", prefix)];
        let mut run = Run::with(
            &installed(shell),
            &[],
            wsl.map(|(k, v)| (k.into(), v.into())).to_vec(),
        );
        // fish reports its own folder too, which Windows passes by as naming none of its own.
        let unix = if prefix == "C:" { "/C:" } else { "/wsl" };
        let start = Path::new(unix).join(run.start.strip_prefix("/").expect("a full path"));
        run.until("the folder", |run| run.folders.contains(&start));
        run.send("cd in*");
        run.until("inner folder", |run| {
            run.folders.contains(&start.join(INNER))
        });
        run.finish();
    }
    fs::remove_dir_all(&fake).expect("cleanup");
}
