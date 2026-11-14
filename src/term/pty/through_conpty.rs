use base64::Engine;
use futures::StreamExt;

use super::*;
use crate::term::{
    drop::Quote,
    mouse::{Format, Tracking},
};

/// A PowerShell script under the bundled ConPTY: pid and events.
fn powershell(script: &str) -> (u32, Vec<PtyEvent>) {
    crate::testlog::init();
    let wide: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let encoded = base64::engine::general_purpose::STANDARD.encode(wide);
    let launch = Launch {
        program: Some((
            "powershell.exe".into(),
            vec!["-NoProfile".into(), "-EncodedCommand".into(), encoded],
        )),
        cwd: std::env::temp_dir(),
        env: Vec::new(),
        reports: false,
        quote: Quote::PowerShell,
    };
    let (pty, mut events) = spawn(&launch, (80, 24), (8, 16), config(&Settings::default()))
        .expect("powershell starts under the bundled ConPTY");
    let events = futures::executor::block_on(async {
        let mut seen = Vec::new();
        while let Some(event) = events.next().await {
            if let PtyEvent::Term(Event::Exit) = event {
                break;
            }
            // Answered as the view answers it.
            if let PtyEvent::Term(Event::TextAreaSizeRequest(reply)) = &event {
                pty.write(reply(fit(80, 24, (8, 16))).into_bytes());
            }
            seen.push(event);
        }
        seen
    });
    (pty.pid, events)
}

#[test]
fn a_program_s_mouse_mode_reaches_the_tap_through_conpty() {
    let (_, events) = powershell(
        "$e = [char]27; [Console]::Out.Write(\"$e[?1049h$e[?1002h$e[?1006h\"); \
         Start-Sleep -Milliseconds 500",
    );
    let modes: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            PtyEvent::Mouse(mode) => Some((mode.tracking, mode.format)),
            _ => None,
        })
        .collect();
    assert_eq!(
        modes.last(),
        Some(&(Tracking::Button, Format::Sgr)),
        "{modes:?}"
    );
}

#[test]
fn a_kitty_graphics_command_passes_conpty_to_its_handler() {
    let (pid, _) = powershell(
        "$e = [char]27; [Console]::Out.Write(\"ab${e}_Gi=31,a=q,q=2;AAAA${e}\\cd\"); \
         Start-Sleep -Milliseconds 500",
    );
    let seen = format!("graphics {pid}: 17 bytes at 0, 2: i=31,a=q,q=2;AAAA");
    assert!(crate::testlog::logged(&seen), "{seen}");
}

#[test]
fn a_cell_size_query_is_answered_through_conpty() {
    // The program reads VT input, as yazi does, and titles what came.
    let (_, events) = powershell(
        "Add-Type -Name K -Namespace W -MemberDefinition '\
         [DllImport(\"kernel32.dll\")] public static extern IntPtr GetStdHandle(int n); \
         [DllImport(\"kernel32.dll\")] public static extern bool SetConsoleMode(IntPtr h, uint m);'; \
         [W.K]::SetConsoleMode([W.K]::GetStdHandle(-10), 0x200) | Out-Null; \
         $e = [char]27; [Console]::Out.Write(\"$e[16t\"); $s = ''; \
         while (($c = [Console]::In.Read()) -ge 0) { $s += [char]$c; if ([char]$c -eq 't') { break } }; \
         [Console]::Title = 'got ' + $s.Replace(\"$e\", '^'); Start-Sleep -Milliseconds 500",
    );
    let titles: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            PtyEvent::Title(title) => Some(title.as_str()),
            _ => None,
        })
        .collect();
    assert!(titles.contains(&"got ^[6;16;8t"), "{titles:?}");
}

#[test]
fn e_runs_its_program_as_written_and_the_session_ends_with_it() {
    crate::testlog::init();
    for (code, well) in [("0", true), ("3", false)] {
        let args = ["/c".to_string(), format!("exit {code}")];
        let launch = crate::term::shell::command("cmd", &args, std::env::temp_dir()).expect("cmd");
        assert!(!launch.reports, "no integration");
        let (pty, mut events) =
            spawn(&launch, (80, 24), (8, 16), config(&Settings::default())).expect("cmd starts");
        let ended = futures::executor::block_on(async {
            let mut ended = None;
            while let Some(event) = events.next().await {
                match event {
                    PtyEvent::Term(Event::ChildExit(status)) => ended = Some(status.success()),
                    PtyEvent::Term(Event::PtyWrite(reply)) => pty.write(reply.into_bytes()),
                    PtyEvent::Term(Event::Exit) => break,
                    _ => {}
                }
            }
            ended
        });
        assert_eq!(ended, Some(well), "exit {code}");
    }
}

#[test]
fn e_finds_a_relative_program_in_the_session_s_folder() {
    crate::testlog::init();
    // A semicolon in the folder must not split it.
    let folder = std::env::temp_dir().join(format!("ely-e;{}", std::process::id()));
    std::fs::create_dir_all(&folder).expect("a folder");
    let cmd = std::path::Path::new(&std::env::var("SystemRoot").expect("Windows"))
        .join(r"System32\cmd.exe");
    std::fs::copy(cmd, folder.join("tool.exe")).expect("a copy");
    let args = ["/c".to_string(), "exit 0".to_string()];
    // A bare name finds the session's folder first, as cmd would.
    for named in [r".\tool.exe", "tool.exe", "tool"] {
        let launch = crate::term::shell::command(named, &args, folder.clone()).expect(named);
        let program = &launch.program.as_ref().expect("a program").0;
        assert!(
            program.starts_with(&*folder.to_string_lossy()),
            "{named}: {program}"
        );
    }
    let launch = crate::term::shell::command("tool", &args, folder.clone()).expect("found");
    let (pty, mut events) =
        spawn(&launch, (80, 24), (8, 16), config(&Settings::default())).expect("it starts");
    let ended = futures::executor::block_on(async {
        let mut ended = None;
        while let Some(event) = events.next().await {
            match event {
                PtyEvent::Term(Event::ChildExit(status)) => ended = Some(status.success()),
                PtyEvent::Term(Event::PtyWrite(reply)) => pty.write(reply.into_bytes()),
                PtyEvent::Term(Event::Exit) => break,
                _ => {}
            }
        }
        ended
    });
    assert_eq!(ended, Some(true));
    drop(pty);
}

#[test]
fn e_finds_a_bare_program_on_a_relative_path_entry() {
    crate::testlog::init();
    let folder = std::env::temp_dir().join(format!("ely-e-path-{}", std::process::id()));
    std::fs::create_dir_all(folder.join("bin")).expect("a folder");
    let cmd = std::path::Path::new(&std::env::var("SystemRoot").expect("Windows"))
        .join(r"System32\cmd.exe");
    std::fs::copy(cmd, folder.join(r"bin\relative-tool.exe")).expect("a copy");
    // The session's PATH holds `bin`, read in its folder.
    unsafe {
        std::env::set_var(
            "PATH",
            format!("bin;{}", std::env::var("PATH").expect("PATH")),
        )
    };
    let args = ["/c".to_string(), "exit 0".to_string()];
    let launch =
        crate::term::shell::command("relative-tool", &args, folder.clone()).expect("found");
    let program = &launch.program.as_ref().expect("a program").0;
    assert!(program.ends_with(r"bin\relative-tool.exe"), "{program}");
}
