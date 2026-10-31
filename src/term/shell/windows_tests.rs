use std::time::{Duration, Instant};

use alacritty_terminal::{event::Event, grid::Dimensions};
use futures::channel::mpsc::TryRecvError;

use super::{SCRIPTS, Start, alias, first_shell, git_bash, install, launch};
use crate::{
    settings::Settings,
    term::{Events, PtyEvent, config, marks::Mark, spawn},
};

#[test]
fn the_scripts_wsl_runs_end_lines_as_unix_shells_read_them() {
    // PowerShell reads a carriage return; WSL's shells do not.
    for (name, text) in SCRIPTS
        .iter()
        .filter(|(name, _)| !name.starts_with("pwsh/"))
    {
        assert!(!text.contains('\r'), "{name} holds a carriage return");
    }
}

#[test]
fn wsl_starts_in_its_own_home_or_in_the_windows_folder_given() {
    let dir = std::env::temp_dir().join("elyterminal-wsl-launch");
    let home = launch("wsl", Start::Home, &dir).expect("a launch");
    let (_, args) = home.program.expect("wsl");
    assert_eq!(args[..3], ["--cd", "~", "-e"]);
    let folder = std::path::PathBuf::from(r"C:\Windows");
    let there = launch("wsl", Start::In(folder.clone()), &dir).expect("a launch");
    assert_eq!(there.cwd, folder);
    assert_eq!(there.program.expect("wsl").1[0], "-e");
    // Another distribution's folder starts that distribution there.
    let share = std::path::PathBuf::from(r"\\wsl.localhost\Debian\home\me\my repo");
    let other = launch("wsl", Start::In(share), &dir).expect("a launch");
    let (_, args) = other.program.expect("wsl");
    assert_eq!(
        args[..5],
        ["-d", "Debian", "--cd", "/home/me/my repo", "-e"]
    );
}

#[test]
fn cd_takes_git_bash_s_drive_paths() {
    let (_, text) = SCRIPTS
        .iter()
        .find(|(name, _)| *name == "pwsh/elyterminal.ps1")
        .expect("the PowerShell script");
    let script = std::env::temp_dir().join(format!("elyterminal-cd-{}.ps1", std::process::id()));
    std::fs::write(&script, text).expect("the script");
    // Only a whole `cd /x` line naming nothing here changes.
    let body = format!(
        "Import-Module PSReadLine; . '{}'; \
         ElyDrivePath 'cd /c/Windows/System32'; ElyDrivePath 'cd /C'; ElyDrivePath 'cd /c/Windows; ls'; \
         $free = [char[]](70..89) | Where-Object {{ -not (Test-Path \"$($_):\\\") }} | Select-Object -First 1; \
         if ((ElyDrivePath \"cd /$free\") -eq \"cd /$free\") {{ 'no drive: kept' }}; \
         $root = New-Item -ItemType Directory -Force \"$env:TEMP\\ely-cd-$PID\\c\\project\"; \
         New-PSDrive -Name ElyCd -PSProvider FileSystem -Root $root.Parent.Parent.FullName | Out-Null; \
         Set-Location ElyCd:\\; ElyDrivePath 'cd /c/proj*'; Set-Location C:\\; \
         Remove-Item -Recurse -Force $root.Parent.Parent.FullName; \
         Set-Alias -Name cd -Value Push-Location -Option AllScope -Scope Global -Force; \
         ElyDrivePath 'cd /c/Windows'; \
         Set-Alias -Name cd -Value Set-Location -Option AllScope -Scope Global -Force; \
         ElyDrivePath \"cd`n/c/Windows\"; $q = 'cd /c/\u{201c}Windows\u{201d}'; if ((ElyDrivePath $q) -eq $q) {{ 'curly: kept' }}; \
         function global:Set-Location {{ }}; ElyDrivePath 'cd /c/Windows'; Remove-Item function:Set-Location; \
         New-PSDrive -Name I -PSProvider FileSystem -Root $env:TEMP | Out-Null; \
         [Threading.Thread]::CurrentThread.CurrentCulture = 'tr-TR'; ElyDrivePath 'cd /i'; \
         $global:ElyReadLine = {{ 'cd /c/Windows' }}; $typed = PSConsoleHostReadLine; \"run: $typed\"",
        script.display()
    );
    let out = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &body])
        .output()
        .expect("powershell runs");
    std::fs::remove_file(&script).expect("cleanup");
    let text = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(
        lines,
        [
            r"cd C:\Windows\System32",
            r"cd C:\",
            "cd /c/Windows; ls",
            "no drive: kept",
            "cd /c/proj*",
            "cd /c/Windows",
            "cd",
            "/c/Windows",
            "curly: kept",
            "cd /c/Windows",
            r"cd I:\",
            // The hook marks the command's start first.
            "\u{1b}]133;C\u{7}run: cd C:\\Windows",
        ],
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn an_alias_hides_no_pwsh_later_on_path() {
    let root = std::env::temp_dir().join(format!("elyterminal-shells-{}", std::process::id()));
    let folders = ["WindowsApps", "Tools;PS7", "System32"].map(|name| root.join(name));
    for (folder, file) in folders
        .iter()
        .zip(["pwsh.exe", "pwsh.exe", "powershell.exe"])
    {
        std::fs::create_dir_all(folder).expect("a folder");
        std::fs::write(folder.join(file), "").expect("a program");
    }
    let path = std::env::join_paths(&folders).expect("a PATH");
    let found = first_shell(path.to_str().expect("text"));
    std::fs::remove_dir_all(&root).expect("cleanup");
    assert_eq!(
        found.expect("a shell"),
        folders[1].join("pwsh.exe").display().to_string()
    );
}

#[test]
fn an_app_execution_alias_is_recognised_by_its_folder() {
    assert!(alias(
        "C:\\Users\\x\\AppData\\Local\\Microsoft\\WindowsApps\\pwsh.exe"
    ));
    assert!(alias(
        "C:\\Users\\x\\AppData\\Local\\Microsoft\\WINDOWSAPPS\\pwsh.exe"
    ));
    assert!(!alias("C:\\Program Files\\PowerShell\\7\\pwsh.exe"));
    assert!(!alias(
        "C:\\WINDOWS\\System32\\WindowsPowerShell\\v1.0\\powershell.exe"
    ));
    assert!(!alias("pwsh.exe"));
}

#[test]
fn git_bash_is_known_by_its_sh_in_bin_beside_usr_bin_bash() {
    let root = std::env::temp_dir().join(format!("elyterminal-git-{}", std::process::id()));
    let file = |path: &str| {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().expect("a folder")).expect("a folder");
        std::fs::write(&path, "").expect("a program");
        path
    };
    let git = file("Git\\bin\\sh.exe");
    file("Git\\usr\\bin\\bash.exe");
    // MSYS2's sh, bash in POSIX mode, reads no --rcfile.
    let msys = file("msys64\\usr\\bin\\sh.exe");
    file("msys64\\usr\\bin\\bash.exe");
    let alone = file("Tools\\bin\\sh.exe");
    let found = [&git, &msys, &alone].map(|sh| git_bash(sh));
    std::fs::remove_dir_all(&root).expect("cleanup");
    assert_eq!(found, [true, false, false]);
}

/// The next folder the session reports, within 30 seconds.
fn next_folder(events: &mut Events) -> std::path::PathBuf {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        match events.try_recv() {
            Ok(PtyEvent::Folder(folder)) => return folder,
            Ok(_) => {}
            Err(TryRecvError::Closed) => {
                panic!("the session ended before it reported a folder")
            }
            Err(TryRecvError::Empty) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(50))
            }
            Err(TryRecvError::Empty) => panic!("no folder within 30 seconds"),
        }
    }
}

#[test]
fn git_bash_s_sh_reports_its_folder_as_windows_names_it() {
    let found = std::process::Command::new("git")
        .arg("--exec-path")
        .output()
        .expect("git runs");
    let exec = String::from_utf8(found.stdout).expect("utf-8");
    // ROOT/mingw64/libexec/git-core, or ucrt64.
    let git = std::path::Path::new(exec.trim())
        .ancestors()
        .nth(3)
        .expect("Git's folder");
    let root = std::env::temp_dir().join(format!("elyterminal-gitbash-{}", std::process::id()));
    let (dir, start) = (root.join("integration"), root.join("start here"));
    std::fs::create_dir_all(start.join("in ner")).expect("the folders");
    install(&dir).expect("the scripts");
    let sh = git.join("bin").join("sh.exe");
    let launch = launch(sh.to_str().expect("text"), start.clone(), &dir).expect("a launch");
    let args = launch.program.as_ref().map(|(_, args)| args.clone());
    assert_eq!(args.expect("arguments")[0], "--rcfile");
    let (pty, mut events) =
        spawn(&launch, (80, 24), (8, 16), config(&Settings::default())).expect("Git Bash starts");
    let first = next_folder(&mut events);
    pty.write(&b"cd 'in ner'\r"[..]);
    let second = next_folder(&mut events);
    // Controls and colons sit at U+F000 up; nl\n beside nl.
    pty.write(&b"mkdir $'bell\\adir' re:po $'nl\\n' nl && cd $'bell\\adir'\r"[..]);
    let bell = next_folder(&mut events);
    pty.write(&b"cd ../re:po\r"[..]);
    let colon = next_folder(&mut events);
    pty.write(&b"cd $'../nl\\n'\r"[..]);
    let newline = next_folder(&mut events);
    pty.write(&b"exit\r"[..]);
    exited(&mut events);
    let real = |path: &std::path::Path| path.canonicalize().expect("a real folder");
    assert_eq!(real(&first), real(&start));
    let inner = start.join("in ner");
    assert_eq!(real(&second), real(&inner));
    assert_eq!(real(&bell), real(&inner.join("bell\u{f007}dir")));
    assert_eq!(real(&colon), real(&inner.join("re\u{f03a}po")));
    assert_eq!(real(&newline), real(&inner.join("nl\u{f00a}")));
    std::fs::remove_dir_all(&root).expect("cleanup");
}

/// The next folder reported, with the events before it.
fn until_folder(events: &mut Events) -> (std::path::PathBuf, Vec<PtyEvent>) {
    let (deadline, mut seen) = (Instant::now() + Duration::from_secs(30), Vec::new());
    loop {
        match events.try_recv() {
            Ok(PtyEvent::Folder(folder)) => return (folder, seen),
            Ok(event) => seen.push(event),
            Err(TryRecvError::Empty) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(50))
            }
            Err(error) => panic!("no folder: {error:?}"),
        }
    }
}

/// Waits for the session to end, within 30 seconds.
fn exited(events: &mut Events) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !matches!(
        events.try_recv(),
        Ok(PtyEvent::Term(Event::Exit)) | Err(TryRecvError::Closed)
    ) {
        assert!(Instant::now() < deadline, "the session did not exit");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Types `line`, then waits for the folder its prompt reports.
fn typed(pty: &crate::term::Pty, events: &mut Events, line: &str) -> std::path::PathBuf {
    pty.write(format!("{line}\r").into_bytes());
    next_folder(events)
}

#[test]
fn cmd_marks_its_prompts_and_reports_its_folder() {
    let root = std::env::temp_dir().join(format!("elyterminal-cmd-{}", std::process::id()));
    let (dir, start) = (root.join("integration"), root.join("start"));
    std::fs::create_dir_all(&start).expect("the folders");
    install(&dir).expect("the scripts");
    let launch = launch("cmd", start.clone(), &dir).expect("a launch");
    assert!(launch.reports);
    let (pty, mut events) =
        spawn(&launch, (80, 24), (8, 16), config(&Settings::default())).expect("cmd starts");
    let first = next_folder(&mut events);
    // A name like a percent escape stays as it stands.
    let odd = start.join("rel%20one");
    std::fs::create_dir_all(&odd).expect("an odd folder");
    let named = typed(&pty, &mut events, &format!("cd /d \"{}\"", odd.display()));
    let windows = typed(&pty, &mut events, "cd /d C:\\Windows");
    pty.write(&b"exit\r"[..]);
    exited(&mut events);
    std::fs::remove_dir_all(&root).expect("cleanup");
    assert_eq!(first, start);
    assert_eq!(named, odd);
    assert_eq!(windows, std::path::Path::new("C:\\Windows"));
}

#[test]
fn a_bare_cmd_typed_in_powershell_marks_its_prompts() {
    let root = std::env::temp_dir().join(format!("elyterminal-pscmd-{}", std::process::id()));
    let (dir, start) = (root.join("integration"), root.join("start"));
    std::fs::create_dir_all(&start).expect("the folders");
    install(&dir).expect("the scripts");
    let mut launch = launch("powershell", start.clone(), &dir).expect("a launch");
    let given = |launch: &crate::term::Launch, name: &str| {
        let found = launch.env.iter().find(|(key, _)| key == name);
        found.expect("the prompt").1.clone()
    };
    assert!(
        !given(&launch, "ELY_CMD_PROMPT").contains("aid="),
        "a nested cmd's is a program's"
    );
    let own = super::launch("cmd", start.clone(), &dir).expect("a cmd launch");
    assert!(
        given(&own, "PROMPT").contains("aid=elyterminal"),
        "a cmd session's is the shell's"
    );
    // A user's prompt cmd would run, parsed as a command.
    let hostile = super::cmd::prompt(Some("$P & echo INJECTED & rem $G"), false);
    launch.env.push(("ELY_CMD_PROMPT".into(), hostile));
    let (pty, mut events) =
        spawn(&launch, (80, 24), (8, 16), config(&Settings::default())).expect("PowerShell starts");
    let shell = next_folder(&mut events);
    pty.write(&b"cmd\r"[..]);
    let (cmd, entered) = until_folder(&mut events);
    // cmd's prompt ends no command: PowerShell's `cmd` runs on.
    let ended = |event: &PtyEvent| matches!(event, PtyEvent::Mark(Mark::Done(_), _));
    assert!(!entered.iter().any(ended), "{entered:?}");
    let windows = typed(&pty, &mut events, "cd /d C:\\Windows");
    pty.write(&b"exit 7\r"[..]);
    let (back, seen) = until_folder(&mut events);
    pty.write(&b"exit\r"[..]);
    exited(&mut events);
    let term = pty.term.lock();
    let top = -(term.grid().history_size() as i32);
    let rows: Vec<String> = (top..term.screen_lines() as i32)
        .map(|line| {
            let row = term.grid()[alacritty_terminal::index::Line(line)].into_iter();
            row.map(|cell| cell.c)
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect();
    drop(term);
    assert!(!rows.iter().any(|row| row == "INJECTED"), "{rows:#?}");
    let real = |path: &std::path::Path| path.canonicalize().expect("a real folder");
    assert_eq!([real(&shell), real(&cmd)], [real(&start), real(&start)]);
    assert_eq!(windows, std::path::Path::new("C:\\Windows"));
    assert_eq!(real(&back), real(&start));
    let failed = |event: &PtyEvent| matches!(event, PtyEvent::Mark(Mark::Done(Some(7)), _));
    assert!(seen.iter().any(failed), "cmd's exit 7 is the line's status");
    std::fs::remove_dir_all(&root).expect("cleanup");
}

#[test]
fn only_a_bare_cmd_typed_takes_the_marked_prompt() {
    let (_, text) = SCRIPTS
        .iter()
        .find(|(name, _)| *name == "pwsh/elyterminal.ps1")
        .expect("the PowerShell script");
    let script = std::env::temp_dir().join(format!("elyterminal-cmd-{}.ps1", std::process::id()));
    std::fs::write(&script, text).expect("the script");
    let body = format!(
        "Import-Module PSReadLine; . '{}'; Set-StrictMode -Version Latest; ElyCmdBack; 'strict: ok'; \
         $env:PROMPT = 'USER'; \
         ElyCmd 'cmd'; \"p=$env:PROMPT\"; $env:ELY_CMD_PROMPT = 'P'; \
         ElyCmd 'cmd'; \"p=$env:PROMPT\"; ElyCmdBack; \"back=$env:PROMPT\"; \
         ElyCmd ' cmd.exe '; \"p=$env:PROMPT\"; ElyCmdBack; \
         ElyCmd 'cmd /c dir'; \"p=$env:PROMPT\"; ElyCmd 'cmd; ls'; \"p=$env:PROMPT\"; \
         function global:cmd {{ }}; ElyCmd 'cmd'; \"p=$env:PROMPT\"; Remove-Item function:cmd; \
         Remove-Item Env:PROMPT; ElyCmd 'cmd' | Out-Null; ElyCmdBack; \"unset=$($null -eq $env:PROMPT)\"",
        script.display()
    );
    let out = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &body])
        .output()
        .expect("powershell runs");
    std::fs::remove_file(&script).expect("cleanup");
    let text = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(
        lines,
        [
            "strict: ok",
            "cmd",
            "p=USER",
            "cmd",
            "p=P",
            "back=USER",
            " cmd.exe ",
            "p=P",
            "cmd /c dir",
            "p=USER",
            "cmd; ls",
            "p=USER",
            "cmd",
            "p=USER",
            "unset=True",
        ],
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
