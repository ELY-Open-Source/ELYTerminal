use std::{
    fs::File,
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use windows_sys::Win32::{
    Foundation::CloseHandle,
    System::Memory::{
        FILE_MAP_READ, MEMORY_BASIC_INFORMATION, MapViewOfFile, OpenFileMappingW, UnmapViewOfFile,
        VirtualQuery,
    },
};

pub(super) fn path(bytes: &[u8]) -> Result<PathBuf, String> {
    let path = std::str::from_utf8(bytes).map_err(|_| "a path is no UTF-8".to_string())?;
    allowed(path)?;
    Ok(PathBuf::from(path))
}

/// Paths whose open reaches a server or device, as Ghostty's.
fn allowed(path: &str) -> Result<(), String> {
    let separator = |byte: u8| byte == b'\\' || byte == b'/';
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && separator(bytes[0]) && separator(bytes[1]) {
        return Err(format!("{path:?} names a share or a device"));
    }
    if bytes.len() >= 4 && separator(bytes[0]) && &bytes[1..3] == b"??" && separator(bytes[3]) {
        return Err(format!("{path:?} names an object"));
    }
    if path.split(['\\', '/']).any(reserved) {
        return Err(format!("{path:?} names a DOS device"));
    }
    Ok(())
}

/// CON, PRN, AUX, NUL, COM0-COM9, LPT0-LPT9, superscripts too.
fn reserved(component: &str) -> bool {
    let end = component.find(['.', ':']).unwrap_or(component.len());
    let stem = component[..end].trim_end_matches(' ');
    let Some((base, rest)) = stem.split_at_checked(3) else {
        return false;
    };
    let base = base.to_ascii_uppercase();
    match rest {
        "" => matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL"),
        _ if base != "COM" && base != "LPT" => false,
        "¹" | "²" | "³" => true,
        _ => rest.len() == 1 && rest.as_bytes()[0].is_ascii_digit(),
    }
}

/// The path with its links resolved, on a local drive.
pub(super) fn real(path: &Path) -> Result<PathBuf, String> {
    let real = std::fs::canonicalize(path).map_err(|error| format!("{path:?}: {error}"))?;
    let shown = real.to_string_lossy();
    let drive = shown.strip_prefix(r"\\?\").unwrap_or(&shown);
    let local = drive.as_bytes().get(..3).is_some_and(|start| {
        start[0].is_ascii_alphabetic() && start[1] == b':' && start[2] == b'\\'
    });
    if !local {
        return Err(format!("{real:?} lies on no local drive"));
    }
    allowed(drive)?;
    Ok(PathBuf::from(drive))
}

pub(super) fn open(real: &Path) -> std::io::Result<File> {
    File::open(real)
}

/// What named shared memory `name` holds, read, then closed.
pub(in super::super) fn shared(
    name: &[u8],
    offset: u32,
    size: u32,
    raw: Option<usize>,
) -> Result<Vec<u8>, String> {
    let shown = std::str::from_utf8(name).map_err(|_| "a name is no UTF-8".to_string())?;
    let wide: Vec<u16> = std::ffi::OsStr::new(shown)
        .encode_wide()
        .chain([0])
        .collect();
    // SAFETY: a NUL-ended name.
    let handle = unsafe { OpenFileMappingW(FILE_MAP_READ, 0, wide.as_ptr()) };
    if handle.is_null() {
        return Err(format!(
            "{shown:?} opens not: {}",
            std::io::Error::last_os_error()
        ));
    }
    // SAFETY: the mapping just opened, viewed whole.
    let view = unsafe { MapViewOfFile(handle, FILE_MAP_READ, 0, 0, 0) };
    let read = if view.Value.is_null() {
        Err(format!(
            "{shown:?} maps not: {}",
            std::io::Error::last_os_error()
        ))
    } else {
        // SAFETY: the view, and a zeroed record for Windows.
        let mut info: MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
        let filled = unsafe {
            VirtualQuery(
                view.Value,
                &mut info,
                std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        };
        let read = if filled == 0 {
            Err(format!("{shown:?}: {}", std::io::Error::last_os_error()))
        } else {
            let range = super::range(info.RegionSize, offset, size, raw);
            range
                .map_err(|error| format!("{shown:?}: {error}"))
                .map(|range| {
                    // SAFETY: the view holds `RegionSize` bytes; the range lies inside.
                    unsafe { std::slice::from_raw_parts(view.Value.cast::<u8>(), info.RegionSize) }
                    [range].to_vec()
                })
        };
        // SAFETY: the view made above, unused after this.
        unsafe { UnmapViewOfFile(view) };
        read
    };
    // SAFETY: the handle OpenFileMappingW gave.
    unsafe { CloseHandle(handle) };
    read
}
