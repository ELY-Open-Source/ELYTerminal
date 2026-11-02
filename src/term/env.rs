//! A Windows session's environment, read from the registry as it starts, as Windows Terminal's `til::env::regenerate` reads it: a folder added to `PATH` after ELYTerminal opened reaches the next session.

use std::collections::{HashMap, HashSet};

use anyhow::bail;
use windows_sys::Win32::{
    Foundation::{ERROR_FILE_NOT_FOUND, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS},
    Globalization::{CSTR_EQUAL, CompareStringOrdinal},
    System::Registry::{
        HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, REG_EXPAND_SZ, REG_SZ, RegCloseKey,
        RegEnumKeyExW, RegEnumValueW, RegOpenKeyExW, RegQueryInfoKeyW,
    },
};

/// The keys Windows reads a user's variables from, in its order.
const KEYS: [(HKEY, &str); 3] = [
    (
        HKEY_LOCAL_MACHINE,
        "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment",
    ),
    (HKEY_CURRENT_USER, "Environment"),
    (HKEY_CURRENT_USER, "Volatile Environment"),
];

/// Variables whose names match as Windows matches them, ignoring case.
#[derive(Debug, Default)]
pub struct Env(Vec<(String, String)>);

impl Env {
    pub fn get(&self, name: &str) -> Option<&str> {
        let found = self.0.iter().find(|(held, _)| same(held, name));
        found.map(|(_, value)| value.as_str())
    }

    /// Sets `name`, replacing it under any case.
    pub fn set(&mut self, name: &str, value: String) {
        match self.0.iter_mut().find(|(held, _)| same(held, name)) {
            Some((_, held)) => *held = value,
            None => self.0.push((name.to_string(), value)),
        }
    }

    /// Adds folders to a path variable, as `til::env::concat_var` does.
    fn concat(&mut self, name: &str, value: String) {
        match self.0.iter_mut().find(|(held, _)| same(held, name)) {
            Some((_, held)) => {
                if !held.ends_with(';') {
                    held.push(';');
                }
                held.push_str(&value);
            }
            None => self.0.push((name.to_string(), value)),
        }
    }

    pub fn into_map(self) -> HashMap<String, String> {
        self.0.into_iter().collect()
    }
}

/// The environment a session starts with: this process's, brought up to date from the registry.
pub fn fresh() -> anyhow::Result<Env> {
    let held = held();
    let mut env = Env(held
        .0
        .iter()
        .filter(|(name, _)| !is_path(name))
        .cloned()
        .collect());
    for (root, key) in KEYS {
        apply(&mut env, &read(root, key)?);
    }
    settle(&mut env, &held);
    Ok(env)
}

/// Each path variable listed once, with the folders only this process held, the registry naming it or not.
fn settle(env: &mut Env, held: &Env) {
    let names: Vec<String> = env
        .0
        .iter()
        .chain(&held.0)
        .map(|(name, _)| name.clone())
        .collect();
    for name in names.iter().filter(|name| is_path(name)) {
        let value = dedupe(env.get(name).unwrap_or_default(), held.get(name));
        if !value.is_empty() {
            env.set(name, value);
        }
    }
}

/// The folders `Path` holds now.
pub fn path() -> anyhow::Result<String> {
    match fresh()?.get("Path") {
        Some(path) => Ok(path.to_string()),
        None => bail!("pty: neither the registry nor this process names a Path"),
    }
}

/// What this process holds; a variable that is not text is left to alacritty, which passes the rest on.
fn held() -> Env {
    let mut held = Env::default();
    for (name, value) in std::env::vars_os() {
        match (name.into_string(), value.into_string()) {
            (Ok(name), Ok(value)) => held.set(&name, value),
            _ => log::debug!("pty: an environment variable is not text, so alacritty passes it"),
        }
    }
    held
}

/// One key's values, as Windows applies them: plain text first, then text to expand, each expanded once.
fn apply(env: &mut Env, values: &[(String, u32, String)]) {
    for pass in [REG_SZ, REG_EXPAND_SZ] {
        for (name, kind, value) in values {
            // Windows Terminal reads a path variable stored as plain text as one to expand.
            let kind = if is_path(name) { REG_EXPAND_SZ } else { *kind };
            if kind != pass || value.is_empty() {
                continue;
            }
            let value = expand(value, env);
            if is_path(name) {
                env.concat(name, value);
            } else {
                env.set(name, value);
            }
        }
    }
}

/// Whether `name` lists folders, which join rather than replace.
fn is_path(name: &str) -> bool {
    ["Path", "LibPath", "Os2LibPath"]
        .iter()
        .any(|known| same(known, name))
}

/// The folders a path variable lists, each once: the registry's order, then those only this process held.
fn dedupe(value: &str, held: Option<&str>) -> String {
    let mut seen = HashSet::new();
    let mut joined = Vec::new();
    // A `%NAME%` this process started with is stale: the registry's expansion stands for it.
    let held = held.into_iter().flat_map(|held| held.split(';'));
    for entry in value
        .split(';')
        .chain(held.filter(|entry| !entry.contains('%')))
    {
        if !entry.is_empty() && seen.insert(entry.trim_end_matches('\\').to_lowercase()) {
            joined.push(entry);
        }
    }
    joined.join(";")
}

/// `value` with each `%NAME%` replaced once from `env`, as `til::env::expand_environment_strings` does; a name `env` lacks stays.
fn expand(value: &str, env: &Env) -> String {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(open) = rest.find('%') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('%') else {
            rest = &rest[open..];
            break;
        };
        match env.get(&after[..close]) {
            Some(found) => out.push_str(found),
            None => out.push_str(&rest[open..open + close + 2]),
        }
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    out
}

/// Whether two names are one to Windows: ordinal, ignoring case through the system's own table.
fn same(a: &str, b: &str) -> bool {
    let (a, b): (Vec<u16>, Vec<u16>) = (a.encode_utf16().collect(), b.encode_utf16().collect());
    // SAFETY: each pointer and length come from a live buffer.
    let order =
        unsafe { CompareStringOrdinal(a.as_ptr(), a.len() as i32, b.as_ptr(), b.len() as i32, 1) };
    order == CSTR_EQUAL
}

/// Each text value `root\key` holds, with its kind, as written.
pub(crate) fn read(root: HKEY, key: &str) -> anyhow::Result<Vec<(String, u32, String)>> {
    let mut handle: HKEY = std::ptr::null_mut();
    let wide: Vec<u16> = key.encode_utf16().chain([0]).collect();
    // SAFETY: `wide` outlives the call.
    let status = unsafe { RegOpenKeyExW(root, wide.as_ptr(), 0, KEY_READ, &mut handle) };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(Vec::new());
    }
    if status != ERROR_SUCCESS {
        bail!("pty: cannot open the registry's {key}: error {status}");
    }
    let values = values(handle, key);
    // SAFETY: `handle` came from RegOpenKeyExW and is closed once.
    unsafe { RegCloseKey(handle) };
    values
}

/// The names of `root\key`'s subkeys; none when it is not there.
pub(crate) fn subkeys(root: HKEY, key: &str) -> anyhow::Result<Vec<String>> {
    let mut handle: HKEY = std::ptr::null_mut();
    let wide: Vec<u16> = key.encode_utf16().chain([0]).collect();
    // SAFETY: `wide` outlives the call.
    let status = unsafe { RegOpenKeyExW(root, wide.as_ptr(), 0, KEY_READ, &mut handle) };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(Vec::new());
    }
    if status != ERROR_SUCCESS {
        bail!("profiles: cannot open the registry's {key}: error {status}");
    }
    // A key's name holds 255 characters at most.
    let (mut name, mut names) = ([0u16; 256], Vec::new());
    let mut failed = None;
    for index in 0.. {
        let mut len = name.len() as u32;
        // SAFETY: `name` holds `len` units; the rest is not asked for.
        let status = unsafe {
            RegEnumKeyExW(
                handle,
                index,
                name.as_mut_ptr(),
                &mut len,
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if status == ERROR_NO_MORE_ITEMS {
            break;
        }
        if status != ERROR_SUCCESS {
            failed = Some(status);
            break;
        }
        names.push(String::from_utf16_lossy(&name[..len as usize]));
    }
    // SAFETY: `handle` came from RegOpenKeyExW and is closed once.
    unsafe { RegCloseKey(handle) };
    if let Some(status) = failed {
        bail!("profiles: cannot list the registry's {key}: error {status}");
    }
    Ok(names)
}

/// The text values of the open key `handle`, sized as the key says its largest are.
fn values(handle: HKEY, key: &str) -> anyhow::Result<Vec<(String, u32, String)>> {
    let (mut names, mut bytes) = (0u32, 0u32);
    // SAFETY: the two counts are written; every other field is not asked for.
    let status = unsafe {
        RegQueryInfoKeyW(
            handle,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut names,
            &mut bytes,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if status != ERROR_SUCCESS {
        bail!("pty: cannot measure the registry's {key}: error {status}");
    }
    let (mut name, mut data) = (vec![0u16; names as usize + 1], vec![0u8; bytes as usize]);
    let mut found = Vec::new();
    for index in 0.. {
        let (mut name_len, mut data_len, mut kind) = (name.len() as u32, data.len() as u32, 0);
        // SAFETY: each buffer is as long as the count passed beside it.
        let status = unsafe {
            RegEnumValueW(
                handle,
                index,
                name.as_mut_ptr(),
                &mut name_len,
                std::ptr::null_mut(),
                &mut kind,
                data.as_mut_ptr(),
                &mut data_len,
            )
        };
        if status == ERROR_NO_MORE_ITEMS {
            break;
        }
        if status != ERROR_SUCCESS {
            bail!("pty: cannot read value {index} of the registry's {key}: error {status}");
        }
        if name_len == 0 || !matches!(kind, REG_SZ | REG_EXPAND_SZ) {
            continue;
        }
        let units: Vec<u16> = data[..data_len as usize]
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .take_while(|unit| *unit != 0)
            .collect();
        let name = String::from_utf16_lossy(&name[..name_len as usize]);
        found.push((name, kind, String::from_utf16_lossy(&units)));
    }
    Ok(found)
}

#[cfg(test)]
mod tests;
