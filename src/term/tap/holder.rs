use super::{Holder, marks::Click};

/// The process group that holds the terminal now, by `master`.
#[cfg(unix)]
pub(super) fn holder(master: Option<&std::fs::File>, pid: u32) -> Option<i32> {
    match crate::term::unix::foreground(master?) {
        Ok(group) => group,
        Err(error) => {
            log::warn!("pty {pid}: no foreground group for a report: {error}");
            None
        }
    }
}

/// Whether an interactive shell leading `group` drew it.
#[cfg(unix)]
pub(super) fn prompts(group: i32, click: Click, pid: u32) -> bool {
    use crate::term::{shell, unix};
    let exe = match unix::path(group) {
        Ok(exe) => exe,
        // Another user's executable hides, its name does not.
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            match unix::name(group) {
                Ok(name) => name,
                Err(error) => {
                    log::debug!("pty {pid}: group {group}'s leader has no name: {error}");
                    return false;
                }
            }
        }
        Err(error) => {
            log::debug!("pty {pid}: group {group}'s leader is gone: {error}");
            return false;
        }
    };
    // A relay's own pty shows its foreground, sudo's among them.
    if shell::relays(&exe) {
        return match unix::inner(group) {
            Ok(Some(inner)) if inner != group => prompts(inner, click, pid),
            Ok(_) => false,
            Err(error) => {
                log::debug!("pty {pid}: relay {group}'s pty does not read: {error}");
                false
            }
        };
    }
    let Some(options) = shell::options(&exe) else {
        return false;
    };
    let interactive = match unix::args(group) {
        // Linux shows no arguments while an exec loads.
        Ok(args) if args.first().is_none_or(String::is_empty) => false,
        // An operand names a script or `-c`'s command.
        Ok(args) => options.interactive(&args[1..]),
        Err(error) => {
            log::debug!("pty {pid}: group {group}'s arguments hide, taken as options: {error}");
            true
        }
    };
    // fish and nu mark their own lines and fork no copy.
    interactive
        && ((click != Click::Arrows && shell::clicks(&exe))
            || unix::lone(group).unwrap_or_else(|error| {
                log::warn!("pty {pid}: group {group}'s processes do not read: {error}");
                false
            }))
}

/// Whether an interactive shell, the one leaf under the session's, drew it.
#[cfg(windows)]
pub(super) fn nested(pid: u32) -> bool {
    use crate::term::{procs, shell};
    let processes = match procs::processes() {
        Ok(processes) => processes,
        Err(error) => {
            log::warn!("pty {pid}: the processes do not read: {error}");
            return false;
        }
    };
    let [leaf] = leaves(&processes, pid, &procs::started)[..] else {
        return false;
    };
    let name = processes.iter().find(|(id, ..)| *id == leaf);
    let Some(options) = name.and_then(|(_, _, name)| shell::options(name.as_ref())) else {
        return false;
    };
    match procs::launch(leaf) {
        Ok((args, _)) if leaf != pid && !args.is_empty() => options.interactive(&args[1..]),
        Ok(_) => false,
        Err(error) => {
            log::debug!("pty {pid}: process {leaf}'s arguments do not read: {error}");
            false
        }
    }
}

/// The processes under `root` that start none, consoles aside.
#[cfg(any(windows, test))]
pub(super) fn leaves(
    processes: &[(u32, u32, String)],
    root: u32,
    started: &dyn Fn(u32) -> Option<u64>,
) -> Vec<u32> {
    let console = |name: &str| {
        name.eq_ignore_ascii_case("conhost.exe") || name.eq_ignore_ascii_case("OpenConsole.exe")
    };
    let (mut under, mut at) = (vec![root], 0);
    while let Some(&pid) = under.get(at) {
        at += 1;
        // A dead parent's id passes on; a child starts after its parent.
        let born = started(pid);
        for (child, _, name) in processes.iter().filter(|(_, of, _)| *of == pid) {
            if console(name) {
                continue;
            }
            let Some((parent, start)) = born.zip(started(*child)) else {
                // An unknown start leaves the tree unknown.
                return Vec::new();
            };
            let after = start >= parent;
            if after && !under.contains(child) {
                under.push(*child);
            }
        }
    }
    let starts = |pid: u32| {
        processes.iter().any(|(child, of, name)| {
            *of == pid && *child != pid && under.contains(child) && !console(name)
        })
    };
    under.iter().copied().filter(|pid| !starts(*pid)).collect()
}

/// An unnamed prompt's holder, by its click and `group`, or none.
pub(super) fn prompter(
    click: Click,
    group: Option<i32>,
    pid: u32,
    named: bool,
    prompts: &dyn Fn(Option<i32>, Click) -> bool,
) -> Holder {
    let ours = |group: i32| u32::try_from(group) == Ok(pid);
    match group {
        Some(group) if !ours(group) && prompts(Some(group), click) => Holder::Shell,
        Some(group) if !ours(group) => Holder::Group(group),
        Some(_) if !named => Holder::Shell,
        None if prompts(None, click) => Holder::Shell,
        _ => Holder::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unnamed_prompt_is_held_by_its_group_as_its_leader_reads() {
        let (shell, program) = (
            |_: Option<i32>, _: Click| true,
            |_: Option<i32>, _: Click| false,
        );
        let (fish, plain) = (
            |_: Option<i32>, click: Click| click != Click::Arrows,
            Click::Arrows,
        );
        // fish's or another unnamed prompt, held by the shell.
        assert_eq!(
            prompter(plain, Some(10), 10, false, &program),
            Holder::Shell
        );
        // A program's prompt under PowerShell, which shares its group.
        assert_eq!(prompter(plain, Some(10), 10, true, &shell), Holder::Unknown);
        // omp's, in a group of its own.
        assert_eq!(
            prompter(plain, Some(20), 10, true, &program),
            Holder::Group(20)
        );
        // A nested interactive shell's, alone in its own group.
        assert_eq!(prompter(plain, Some(20), 10, true, &shell), Holder::Shell);
        assert_eq!(prompter(plain, None, 10, false, &program), Holder::Unknown);
        // The leader's reading hears the click, as fish's does.
        assert_eq!(
            prompter(Click::Absolute, Some(20), 10, true, &fish),
            Holder::Shell
        );
        assert_eq!(
            prompter(plain, Some(20), 10, true, &fish),
            Holder::Group(20)
        );
        // Without a group, Windows' process tree decides.
        assert_eq!(prompter(plain, None, 10, true, &shell), Holder::Shell);
        assert_eq!(prompter(plain, None, 10, true, &program), Holder::Unknown);
    }

    #[test]
    fn the_one_leaf_under_the_session_s_shell_is_the_nested_one() {
        let process = |pid: u32, of: u32, name: &str| (pid, of, name.to_string());
        // pwsh 10 runs nu 20; its console passes by.
        let idle = vec![
            process(10, 1, "pwsh.exe"),
            process(11, 10, "conhost.exe"),
            process(20, 10, "nu.exe"),
        ];
        let order = |pid: u32| Some(u64::from(pid));
        assert_eq!(leaves(&idle, 10, &order), [20]);
        let mut busy = idle.clone();
        busy.push(process(30, 20, "omp.exe"));
        assert_eq!(leaves(&busy, 10, &order), [30]);
        // An orphan whose parent's id passed on hangs under no one.
        let mut reused = busy.clone();
        reused.push(process(5, 30, "nu.exe"));
        assert_eq!(leaves(&reused, 10, &order), [30]);
        // An unreadable start tells nothing, so no shell.
        let hidden = |pid: u32| (pid != 30).then_some(u64::from(pid));
        assert!(leaves(&busy, 10, &hidden).is_empty());
        // A console whose start hides passes by.
        let quiet = |pid: u32| (pid != 11).then_some(u64::from(pid));
        assert_eq!(leaves(&idle, 10, &quiet), [20]);
        assert_eq!(leaves(&[process(10, 1, "pwsh.exe")], 10, &order), [10]);
    }

    #[cfg(windows)]
    #[test]
    fn a_nested_interactive_powershell_is_the_one_leaf() {
        use std::process::{Command, Stdio};
        let read = |inner: &str| {
            let mut outer = Command::new("pwsh")
                .args(["-NoLogo", "-NoProfile", "-NoExit", "-Command", inner])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .spawn()
                .expect("pwsh starts");
            std::thread::sleep(std::time::Duration::from_secs(5));
            let read = nested(outer.id());
            let _ = Command::new("taskkill")
                .args(["/T", "/F", "/PID", &outer.id().to_string()])
                .output();
            outer.wait().expect("it ends");
            read
        };
        assert!(read("pwsh -NoLogo -NoProfile"));
        assert!(!read("pwsh -NoLogo -NoProfile -Command Start-Sleep 30"));
    }

    #[cfg(unix)]
    #[test]
    fn an_option_s_value_is_no_operand() {
        use crate::term::shell::options;
        use std::path::Path;
        let read = |shell: &str, args: &[&str]| {
            let args: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
            options(Path::new(shell))
                .expect("a shell")
                .interactive(&args)
        };
        assert!(read(
            "/bin/bash",
            &["--rcfile", "/etc/ely", "-O", "extglob"]
        ));
        assert!(read("/usr/bin/zsh", &["-o", "vi"]));
        assert!(read("/opt/homebrew/bin/fish", &["-C", "set t 1"]));
        assert!(!read(
            "/opt/homebrew/bin/fish",
            &["-C", "set t 1", "a.fish"]
        ));
        assert!(!read("/bin/bash", &["-c", "true"]));
        assert!(!read("/bin/bash", &["--rcfile"]));
        assert!(read(
            "/opt/homebrew/bin/nu",
            &["--config", "c.nu", "-e", "ls"]
        ));
        assert!(!read("/opt/homebrew/bin/nu", &["a.nu"]));
        // Bundles, bash's optional values, command mode, the end of options.
        assert!(read("/bin/zsh", &["-lo", "vi"]));
        assert!(read("/bin/zsh", &["--emulate", "zsh"]));
        assert!(read("/bin/bash", &["-iO", "extglob"]));
        assert!(read("/bin/bash", &["+o"]));
        assert!(read("/opt/homebrew/bin/fish", &["-NC", "set t 1"]));
        assert!(!read(
            "/opt/homebrew/bin/fish",
            &["-C", "x", "--command=read"]
        ));
        assert!(!read("/opt/homebrew/bin/fish", &["-c'read'"]));
        assert!(!read("/bin/bash", &["--", "--rcfile"]));
        assert!(!read("/bin/zsh", &["--", "-o", "vi"]));
        assert!(read("/bin/bash", &["--"]));
        // Each bash value is the next argument; zsh's other stops; fish's prefixes.
        assert!(read("/bin/bash", &["-oi", "vi"]));
        assert!(read("/bin/bash", &["-oO", "vi", "extglob"]));
        for stop in [
            &["+", "-o", "vi"][..],
            &["+-", "-o", "vi"],
            &["-lb", "-o", "vi"],
            &["-i-", "-o", "vi"],
        ] {
            assert!(!read("/bin/zsh", stop), "{stop:?}");
        }
        assert!(read("/bin/zsh", &["-b"]));
        assert!(!read("/opt/homebrew/bin/fish", &["-C", "x", "--comm=read"]));
        assert!(!read("/opt/homebrew/bin/fish", &["--c=read"]));
        assert!(read("/opt/homebrew/bin/fish", &["--init-comm", "set t 1"]));
        assert!(!read("/opt/homebrew/bin/fish", &["--pr"]));
        // bash's single dash names; pwsh's prefixes; nu's bundles and lists.
        assert!(read("/bin/bash", &["-rcfile", "rc", "-norc"]));
        assert!(read("/bin/bash", &["-init-file", "rc"]));
        assert!(read(
            "/opt/homebrew/bin/pwsh",
            &["-nop", "-wo", "/tmp", "-outp", "Text"]
        ));
        assert!(!read("/opt/homebrew/bin/pwsh", &["-Comm", "Get-Date"]));
        assert!(read("/opt/homebrew/bin/nu", &["-ne", "print 1"]));
        assert!(read("/opt/homebrew/bin/nu", &["-nI", "/tmp"]));
        assert!(read(
            "/opt/homebrew/bin/nu",
            &["-n", "--log-include", "a", "b"]
        ));
        assert!(!read("/opt/homebrew/bin/nu", &["-nc", "ls"]));
        assert!(read("/opt/homebrew/bin/pwsh", &["-NoLogo", "-NoPro"]));
        assert!(read("/opt/homebrew/bin/pwsh", &["-NoProfile", "-in"]));
        assert!(read("/opt/homebrew/bin/pwsh", &["--NoLogo", "/NoProfile"]));
        assert!(read(
            "/opt/homebrew/bin/pwsh",
            &["\u{2013}NoLogo", " -NoProfile ", "\u{2014}\u{2014}NoExit"]
        ));
        assert!(read("/opt/homebrew/bin/pwsh", &["-ea", "x", "-NoExit"]));
        assert!(!read(
            "/opt/homebrew/bin/nu",
            &["-n", "--log-include=nu_cli", "a.nu"]
        ));
        assert!(read(
            "/opt/homebrew/bin/pwsh",
            &["-NoLogo", "-executionpolicy", "Bypass"]
        ));
        assert!(!read("/opt/homebrew/bin/pwsh", &["-Command", "Get-Date"]));
        assert!(options(Path::new("/bin/sleep")).is_none());
        assert!(crate::term::shell::relays(Path::new("/usr/bin/ssh")));
        assert!(!crate::term::shell::relays(Path::new("/bin/zsh")));
        let own = crate::term::unix::name(std::process::id() as i32).expect("its own name");
        assert!(own.to_string_lossy().starts_with("elyterminal"), "{own:?}");
    }

    #[cfg(unix)]
    #[test]
    fn only_an_interactive_shell_leading_its_group_draws_its_prompt() {
        use std::{
            io::{BufRead, BufReader, Write},
            os::unix::process::CommandExt,
            process::{Command, Stdio},
        };
        // Each says `ready` once it runs, past Linux's exec.
        let read = |program: &str, args: &[&str], line: &[u8], click: Click| {
            let mut child = Command::new(program)
                .args(args)
                .process_group(0)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .expect("it starts");
            child
                .stdin
                .as_mut()
                .expect("its input")
                .write_all(line)
                .expect("it reads");
            let mut ready = String::new();
            let output = child.stdout.as_mut().expect("its output");
            BufReader::new(output)
                .read_line(&mut ready)
                .expect("it says");
            assert_eq!(ready, "ready\n", "{program} {args:?}");
            let read = prompts(child.id() as i32, click, 0);
            // SAFETY: a signal to the test's own group.
            unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL) };
            child.wait().expect("it ends");
            read
        };
        let (plain, click) = (Click::Arrows, Click::Absolute);
        assert!(read("bash", &["-s"], b"echo ready\n", plain));
        assert!(read("bash", &["+H", "-s"], b"echo ready\n", plain));
        assert!(read("zsh", &["-f", "-s"], b"echo ready\n", plain));
        // A command's or a script's operand.
        assert!(!read(
            "bash",
            &["-c", "echo ready; read -r line"],
            b"",
            plain
        ));
        assert!(!read("bash", &["-s", "operand"], b"echo ready\n", plain));
        // A job beside the shell, as a subshell runs one.
        assert!(!read("bash", &["-s"], b"sleep 30 & echo ready\n", plain));
        assert!(!read("bash", &["-s"], b"sleep 30 & echo ready\n", click));
        // A program alone in its group.
        assert!(!read(
            "sh",
            &["-c", "echo ready; exec sleep 30"],
            b"",
            plain
        ));
        // fish's clicks are its own line's, beside a job too.
        let job = ["--no-config", "--init-command=sleep 30 & echo ready"];
        assert!(read("fish", &job, b"", click));
        assert!(!read("fish", &job, b"", plain));
        // A fish command's `read`, as a script's, is the program's.
        let asks = ["--no-config", "-c", "echo ready; read line"];
        assert!(!read("fish", &asks, b"", click));
    }
}
