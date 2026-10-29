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
