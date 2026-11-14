//! Windows' default terminal: a console program started without a console opens here.

use std::{ffi::OsString, path::Path};

use anyhow::Context as _;
use futures::StreamExt as _;
use gpui::App;

use crate::registry::{self, set};

mod com;

pub use com::Handoff;

/// ELYTerminal's class, which OpenConsole hands each session.
const TERMINAL: &str = "{79B54338-A0FA-474F-B304-4932B36B6F55}";
/// OpenConsole's class, which the bundled `OpenConsole.exe` serves.
const CONSOLE: &str = "{2EACA947-7F5F-4CFA-BA87-8F7FBEEFBE69}";
/// `OpenConsoleProxy.dll`, which carries handles between the two.
const PROXY: &str = "{3171DE52-6EFA-4AEF-8A9F-D02BD67E7A4F}";
/// `IConsoleHandoff`, `ITerminalHandoff3` and `IDefaultTerminalMarker`.
const INTERFACES: [&str; 3] = [
    "{E686C757-9A35-4A1C-B3CE-0BCC8B5C69F4}",
    "{6F23DA90-15C5-4203-9DB0-64E73F1B1B00}",
    "{746E6BC0-AB05-4E38-AB14-71E86763141F}",
];

/// The user's classes and the console's startup key, as Windows reads them.
pub struct Keys<'a> {
    pub classes: &'a str,
    pub startup: &'a str,
}

pub const KEYS: Keys = Keys {
    classes: r"Software\Classes",
    startup: r"Console\%%Startup",
};

/// Who opens console programs.
#[derive(Debug, PartialEq)]
pub enum Default {
    /// This program.
    Ours,
    /// Another copy of ELYTerminal, by its command.
    Copy(String),
    /// Windows' console or another terminal.
    Elsewhere,
}

/// The command COM runs for `exe`; COM adds `-Embedding`.
fn command(exe: &Path) -> OsString {
    let mut command = OsString::from("\"");
    command.push(exe);
    command.push("\"");
    command
}

fn server(keys: &Keys, class: &str) -> String {
    format!(r"{}\CLSID\{class}\LocalServer32", keys.classes)
}

/// Who opens console programs under `keys`, judged for `exe`.
pub fn read(keys: &Keys, exe: &Path) -> anyhow::Result<Default> {
    let terminal = registry::text(keys.startup, Some("DelegationTerminal"))?;
    if terminal.is_none_or(|clsid| !clsid.eq_ignore_ascii_case(TERMINAL)) {
        return Ok(Default::Elsewhere);
    }
    Ok(match registry::text(&server(keys, TERMINAL), None)? {
        Some(found) if found == command(exe) => Default::Ours,
        Some(found) => Default::Copy(found.to_string_lossy().into()),
        None => Default::Copy(String::new()),
    })
}

/// Makes `exe` open console programs under `keys`, through the OpenConsole beside it.
pub fn make(keys: &Keys, exe: &Path) -> anyhow::Result<()> {
    let folder = exe
        .parent()
        .context("defterm: the program lies in no folder")?;
    let (console, proxy) = (
        folder.join("OpenConsole.exe"),
        folder.join("OpenConsoleProxy.dll"),
    );
    for file in [&console, &proxy] {
        anyhow::ensure!(
            file.is_file(),
            "defterm: no {} beside the program",
            file.display()
        );
    }
    set(&server(keys, TERMINAL), None, &command(exe))?;
    set(&server(keys, CONSOLE), None, &command(&console))?;
    let stub = format!(r"{}\CLSID\{PROXY}\InProcServer32", keys.classes);
    set(&stub, None, proxy.as_os_str())?;
    set(&stub, Some("ThreadingModel"), "Both".as_ref())?;
    for interface in INTERFACES {
        let key = format!(r"{}\Interface\{interface}\ProxyStubClsid32", keys.classes);
        set(&key, None, PROXY.as_ref())?;
    }
    set(keys.startup, Some("DelegationConsole"), CONSOLE.as_ref())?;
    set(keys.startup, Some("DelegationTerminal"), TERMINAL.as_ref())?;
    log::info!("defterm: {} opens console programs", exe.display());
    Ok(())
}

/// Takes ELYTerminal's classes away; Windows decides again if they were the default.
pub fn restore(keys: &Keys) -> anyhow::Result<()> {
    let terminal = registry::text(keys.startup, Some("DelegationTerminal"))?;
    if terminal.is_some_and(|clsid| clsid.eq_ignore_ascii_case(TERMINAL)) {
        registry::delete_value(keys.startup, "DelegationConsole")?;
        registry::delete_value(keys.startup, "DelegationTerminal")?;
    }
    for class in [TERMINAL, CONSOLE, PROXY] {
        registry::delete_tree(&format!(r"{}\CLSID\{class}", keys.classes))?;
    }
    for interface in INTERFACES {
        registry::delete_tree(&format!(r"{}\Interface\{interface}", keys.classes))?;
    }
    log::info!("defterm: Windows decides who opens console programs");
    Ok(())
}

/// Takes each session OpenConsole hands over, for as long as the app runs.
pub fn listen(cx: &mut App) -> anyhow::Result<()> {
    let (sender, mut handed) = futures::channel::mpsc::unbounded();
    com::register(sender)?;
    cx.spawn(async move |cx| {
        while let Some(handoff) = handed.next().await {
            if let Err(error) = cx.update(|cx| crate::app::adopt(handoff, cx)) {
                log::error!("{error:#}");
            }
        }
    })
    .detach();
    Ok(())
}

#[cfg(test)]
mod tests;
