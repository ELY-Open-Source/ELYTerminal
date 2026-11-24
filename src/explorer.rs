//! Open in ELYTerminal on Explorer's folder menus, and `elyterminal` on the user's `PATH` for its address bar.

use std::{ffi::OsString, path::Path};

use anyhow::Context as _;
use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, REG_EXPAND_SZ};

use crate::{
    registry::{self, set},
    term::env::{Env, expand},
};

/// The user's own file classes and variables.
pub struct Keys<'a> {
    pub classes: &'a str,
    pub environment: &'a str,
}

pub const KEYS: Keys = Keys {
    classes: r"Software\Classes",
    environment: "Environment",
};

/// A folder's menu and its background's, under the classes.
const ENTRIES: [&str; 2] = [
    r"Directory\shell\ELYTerminal",
    r"Directory\Background\shell\ELYTerminal",
];

/// What the menu holds now.
#[derive(Debug, PartialEq)]
pub enum Entry {
    Absent,
    /// This program, on both menus and on `PATH`.
    Ours,
    /// Another command, as another copy left it.
    Other(String),
}

/// The command each entry runs: `exe` on the folder.
pub fn command(exe: &Path) -> OsString {
    let mut command = OsString::from("\"");
    command.push(exe);
    command.push("\" \"%V\"");
    command
}

/// What the menu holds under `keys` for `exe`; its menu with its folder off `PATH` reads absent, so Add writes both.
pub fn read(keys: &Keys, exe: &Path) -> anyhow::Result<Entry> {
    let ours = command(exe);
    let found = ENTRIES
        .iter()
        .map(|entry| registry::text(&format!(r"{}\{entry}\command", keys.classes), None))
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(match found.as_slice() {
        [None, None] => Entry::Absent,
        [Some(a), Some(b)] if *a == ours && *b == ours => {
            if holds(&user_path(keys)?.0, folder(exe)?, &fresh()?) {
                Entry::Ours
            } else {
                log::info!("explorer: the menu is this copy's, its folder is off PATH");
                Entry::Absent
            }
        }
        _ => {
            // Show what differs, else the one entry there.
            let mut there = found.iter().flatten();
            let other = (there.clone().find(|text| **text != ours)).or_else(|| there.next());
            let other = other.expect("one entry at least is there");
            Entry::Other(other.to_string_lossy().into())
        }
    })
}

/// Puts `exe` on both menus under `keys`, named `label`, and its folder on the user's `PATH`.
pub fn add(keys: &Keys, exe: &Path, label: &str) -> anyhow::Result<()> {
    let mut icon = exe.as_os_str().to_owned();
    icon.push(",0");
    for entry in ENTRIES {
        let key = format!(r"{}\{entry}", keys.classes);
        set(&key, None, label.as_ref())?;
        set(&key, Some("Icon"), &icon)?;
        set(&format!(r"{key}\command"), None, &command(exe))?;
    }
    let folder = folder(exe)?;
    let (path, kind) = user_path(keys)?;
    if !holds(&path, folder, &fresh()?) {
        let joined = if path.is_empty() {
            folder.to_string()
        } else {
            format!("{path};{folder}")
        };
        set_path(keys, &joined, kind)?;
    }
    log::info!(
        "explorer: the menu and PATH's `elyterminal` open {}",
        exe.display()
    );
    Ok(())
}

/// Takes both entries off the menus under `keys`, and `exe`'s folder off the user's `PATH`.
pub fn remove(keys: &Keys, exe: &Path) -> anyhow::Result<()> {
    for entry in ENTRIES {
        registry::delete_tree(&format!(r"{}\{entry}", keys.classes))?;
    }
    let folder = folder(exe)?;
    let (path, kind) = user_path(keys)?;
    let env = fresh()?;
    if holds(&path, folder, &env) {
        let kept: Vec<&str> = path
            .split(';')
            .filter(|part| !same(part, folder, &env))
            .collect();
        set_path(keys, &kept.join(";"), kind)?;
    }
    log::info!("explorer: the menu and PATH no longer open ELYTerminal");
    Ok(())
}

fn folder(exe: &Path) -> anyhow::Result<&str> {
    let folder = exe
        .parent()
        .context("explorer: the program lies in no folder")?;
    folder
        .to_str()
        .with_context(|| format!("explorer: {} is not UTF-8", folder.display()))
}

/// The user's own `PATH` as written, and its kind; empty and expandable when unset.
fn user_path(keys: &Keys) -> anyhow::Result<(String, u32)> {
    let values = crate::term::env::read(HKEY_CURRENT_USER, keys.environment)?;
    let path = values
        .into_iter()
        .find(|(name, ..)| name.eq_ignore_ascii_case("Path"));
    Ok(
        path.map_or((String::new(), REG_EXPAND_SZ), |(_, kind, value)| {
            (value, kind)
        }),
    )
}

/// The variables a program started now reads, which expand `PATH`'s entries.
fn fresh() -> anyhow::Result<Env> {
    crate::term::env::fresh().context("explorer: the user's variables")
}

fn holds(path: &str, folder: &str, env: &Env) -> bool {
    path.split(';').any(|part| same(part, folder, env))
}

/// One folder, its variables expanded, as Windows compares names, a last backslash aside.
fn same(part: &str, folder: &str, env: &Env) -> bool {
    let trim = |text: &str| text.trim().trim_end_matches('\\').to_lowercase();
    trim(&expand(part, env)) == trim(folder)
}

/// Writes the user's `PATH` and tells running programs, Explorer among them.
fn set_path(keys: &Keys, path: &str, kind: u32) -> anyhow::Result<()> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE,
    };
    registry::set_kind(keys.environment, Some("Path"), path.as_ref(), kind)?;
    let area = registry::wide("Environment".as_ref());
    let mut result = 0;
    // SAFETY: `area` outlives the call, which waits 5 seconds at most a window.
    let sent = unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            area.as_ptr() as isize,
            SMTO_ABORTIFHUNG,
            5000,
            &mut result,
        )
    };
    anyhow::ensure!(
        sent != 0,
        "explorer: PATH changed, but its broadcast failed: {}",
        std::io::Error::last_os_error()
    );
    log::info!("explorer: the user's PATH is now {path}");
    Ok(())
}

#[cfg(test)]
mod tests;
