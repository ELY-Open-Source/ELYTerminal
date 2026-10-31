//! Open in ELYTerminal on Explorer's folder menus, for this user.

use std::{ffi::OsString, os::windows::ffi::OsStrExt, path::Path};

use anyhow::bail;
use windows_sys::Win32::{
    Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS},
    System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ, RRF_RT_REG_SZ,
        RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegGetValueW, RegSetValueExW,
    },
};

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
        .map(|entry| text(&format!(r"{classes}\{entry}\command")))
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
        let key = wide(format!(r"{classes}\{entry}").as_ref());
        // SAFETY: `key` outlives the call.
        let status = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, key.as_ptr()) };
        if status != ERROR_SUCCESS && status != ERROR_FILE_NOT_FOUND {
            bail!("explorer: cannot delete {classes}\\{entry}: error {status}");
        }
    }
    log::info!("explorer: the menu no longer opens ELYTerminal");
    Ok(())
}

fn wide(text: &std::ffi::OsStr) -> Vec<u16> {
    text.encode_wide().chain([0]).collect()
}

/// `key`'s default text, if the key is there.
fn text(key: &str) -> anyhow::Result<Option<OsString>> {
    use std::os::windows::ffi::OsStringExt;
    let name = wide(key.as_ref());
    let mut bytes = 0u32;
    // SAFETY: a null buffer asks only for the size.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            name.as_ptr(),
            std::ptr::null(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut bytes,
        )
    };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    if status != ERROR_SUCCESS {
        bail!("explorer: cannot read {key}: error {status}");
    }
    let mut buffer = vec![0u16; bytes as usize / 2];
    // SAFETY: `buffer` holds `bytes` bytes.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            name.as_ptr(),
            std::ptr::null(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut bytes,
        )
    };
    if status != ERROR_SUCCESS {
        bail!("explorer: cannot read {key}: error {status}");
    }
    let end = buffer
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(buffer.len());
    Ok(Some(OsString::from_wide(&buffer[..end])))
}

/// Sets `key`'s `name`, or its default, to `value`.
fn set(key: &str, name: Option<&str>, value: &std::ffi::OsStr) -> anyhow::Result<()> {
    let path = wide(key.as_ref());
    let mut handle: HKEY = std::ptr::null_mut();
    // SAFETY: `path` outlives the call; `handle` is written.
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            path.as_ptr(),
            0,
            std::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            std::ptr::null(),
            &mut handle,
            std::ptr::null_mut(),
        )
    };
    if status != ERROR_SUCCESS {
        bail!("explorer: cannot make {key}: error {status}");
    }
    let name = name.map(|name| wide(name.as_ref()));
    let data = wide(value);
    // SAFETY: `data` holds its NUL; `handle` is closed once.
    let status = unsafe {
        let status = RegSetValueExW(
            handle,
            name.as_ref().map_or(std::ptr::null(), |name| name.as_ptr()),
            0,
            REG_SZ,
            data.as_ptr().cast(),
            (data.len() * 2) as u32,
        );
        RegCloseKey(handle);
        status
    };
    if status != ERROR_SUCCESS {
        bail!("explorer: cannot write {key}: error {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests;
