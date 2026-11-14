//! Text values under the user's own registry, `HKEY_CURRENT_USER`.

use std::{
    ffi::{OsStr, OsString},
    os::windows::ffi::{OsStrExt, OsStringExt},
};

use anyhow::bail;
use windows_sys::Win32::{
    Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS},
    System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ, RRF_RT_REG_SZ,
        RegCloseKey, RegCreateKeyExW, RegDeleteKeyValueW, RegDeleteTreeW, RegGetValueW,
        RegSetValueExW,
    },
};

pub fn wide(text: &OsStr) -> Vec<u16> {
    text.encode_wide().chain([0]).collect()
}

/// `key`'s `name`, or its default, if both are there.
pub fn text(key: &str, name: Option<&str>) -> anyhow::Result<Option<OsString>> {
    let path = wide(key.as_ref());
    let name = name.map(|name| wide(name.as_ref()));
    let name = name.as_ref().map_or(std::ptr::null(), |name| name.as_ptr());
    let mut bytes = 0u32;
    // SAFETY: a null buffer asks only for the size.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            path.as_ptr(),
            name,
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
        bail!("registry: cannot read {key}: error {status}");
    }
    let mut buffer = vec![0u16; bytes as usize / 2];
    // SAFETY: `buffer` holds `bytes` bytes.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            path.as_ptr(),
            name,
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut bytes,
        )
    };
    if status != ERROR_SUCCESS {
        bail!("registry: cannot read {key}: error {status}");
    }
    let end = buffer
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(buffer.len());
    Ok(Some(OsString::from_wide(&buffer[..end])))
}

/// Sets `key`'s `name`, or its default, to `value`.
pub fn set(key: &str, name: Option<&str>, value: &OsStr) -> anyhow::Result<()> {
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
        bail!("registry: cannot make {key}: error {status}");
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
        bail!("registry: cannot write {key}: error {status}");
    }
    Ok(())
}

/// Deletes `key` and all under it; a key not there is fine.
pub fn delete_tree(key: &str) -> anyhow::Result<()> {
    let path = wide(key.as_ref());
    // SAFETY: `path` outlives the call.
    let status = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, path.as_ptr()) };
    if status != ERROR_SUCCESS && status != ERROR_FILE_NOT_FOUND {
        bail!("registry: cannot delete {key}: error {status}");
    }
    Ok(())
}

/// Deletes `key`'s `name`; a value not there is fine.
pub fn delete_value(key: &str, name: &str) -> anyhow::Result<()> {
    let (path, value) = (wide(key.as_ref()), wide(name.as_ref()));
    // SAFETY: both outlive the call.
    let status = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, path.as_ptr(), value.as_ptr()) };
    if status != ERROR_SUCCESS && status != ERROR_FILE_NOT_FOUND {
        bail!("registry: cannot delete {key}\\{name}: error {status}");
    }
    Ok(())
}
