use base64::Engine;
use futures::StreamExt;

use super::*;
use crate::term::{
    drop::Quote,
    mouse::{Format, Tracking},
};

#[test]
fn a_program_s_mouse_mode_reaches_the_tap_through_conpty() {
    crate::testlog::init();
    let script = "$e = [char]27; [Console]::Out.Write(\"$e[?1049h$e[?1002h$e[?1006h\"); \
                  Start-Sleep -Milliseconds 500";
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
    let (_pty, mut events) = spawn(&launch, (80, 24), (8, 16), config(&Settings::default()))
        .expect("powershell starts under the bundled ConPTY");
    let modes = futures::executor::block_on(async {
        let mut modes = Vec::new();
        while let Some(event) = events.next().await {
            match event {
                PtyEvent::Mouse(mode) => modes.push((mode.tracking, mode.format)),
                PtyEvent::Term(Event::Exit) => break,
                _ => {}
            }
        }
        modes
    });
    assert_eq!(
        modes.last(),
        Some(&(Tracking::Button, Format::Sgr)),
        "{modes:?}"
    );
}
