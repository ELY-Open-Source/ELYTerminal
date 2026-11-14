//! Open in ELYTerminal on Explorer's folder menus, for this user.

use std::{ffi::OsString, path::Path};

use crate::registry::{self, set};

/// Where a user's own file classes live.
pub const CLASSES: &str = r"Software\Classes";

/// A folder's menu and its background's, under the classes.
const ENTRIES: [&str; 2] = [
    r"Directory\shell\ELYTerminal",
    r"Directory\Background\shell\ELYTerminal",
];

/// What the menu holds now.
#[derive(Debug, PartialEq)]
pub enum Entry {
    Absent,
    /// This program, on both menus.
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

/// What the menu holds under `classes` for `exe`.
pub fn read(classes: &str, exe: &Path) -> anyhow::Result<Entry> {
    let ours = command(exe);
    let found = ENTRIES
        .iter()
        .map(|entry| registry::text(&format!(r"{classes}\{entry}\command"), None))
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(match found.as_slice() {
        [None, None] => Entry::Absent,
        [Some(a), Some(b)] if *a == ours && *b == ours => Entry::Ours,
        _ => {
            // Show what differs, else the one entry there.
            let mut there = found.iter().flatten();
            let other = (there.clone().find(|text| **text != ours)).or_else(|| there.next());
            let other = other.expect("one entry at least is there");
            Entry::Other(other.to_string_lossy().into())
        }
    })
}

/// Puts `exe` on both menus under `classes`, named `label`.
pub fn add(classes: &str, exe: &Path, label: &str) -> anyhow::Result<()> {
    let mut icon = exe.as_os_str().to_owned();
    icon.push(",0");
    for entry in ENTRIES {
        let key = format!(r"{classes}\{entry}");
        set(&key, None, label.as_ref())?;
        set(&key, Some("Icon"), &icon)?;
        set(&format!(r"{key}\command"), None, &command(exe))?;
    }
    log::info!("explorer: the menu opens {} on a folder", exe.display());
    Ok(())
}

/// Takes both entries off the menus under `classes`.
pub fn remove(classes: &str) -> anyhow::Result<()> {
    for entry in ENTRIES {
        registry::delete_tree(&format!(r"{classes}\{entry}"))?;
    }
    log::info!("explorer: the menu no longer opens ELYTerminal");
    Ok(())
}

#[cfg(test)]
mod tests;
