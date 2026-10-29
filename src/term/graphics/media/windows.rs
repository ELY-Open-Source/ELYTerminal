use std::{
    fs::File,
    os::windows::{ffi::OsStrExt, fs::OpenOptionsExt, io::AsRawHandle},
    path::{Path, PathBuf},
};

use windows_sys::Win32::{
    Foundation::CloseHandle,
    Storage::FileSystem::{
        DELETE, FILE_DISPOSITION_INFO, FILE_GENERIC_READ, FILE_NAME_NORMALIZED,
        FileDispositionInfo, GetFinalPathNameByHandleW, SetFileInformationByHandle,
        VOLUME_NAME_DOS,
    },
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

/// A file whose opened path was checked.
pub(super) struct Opened {
    pub file: File,
}

/// Opens `real`, refusing a file that moved since its check.
pub(super) fn open(real: &Path, temporary: bool) -> Result<Opened, String> {
    let mut options = File::options();
    options.read(true);
    if temporary {
        options.access_mode(FILE_GENERIC_READ | DELETE);
    }
    let file = options
        .open(real)
        .map_err(|error| format!("{real:?} opens not: {error}"))?;
    let mut path = vec![0u16; 32768];
    // SAFETY: an open handle and a buffer of that length.
    let length = unsafe {
        GetFinalPathNameByHandleW(
            file.as_raw_handle(),
            path.as_mut_ptr(),
            path.len() as u32,
            FILE_NAME_NORMALIZED | VOLUME_NAME_DOS,
        )
    } as usize;
    if length == 0 || length >= path.len() {
        return Err(format!("{real:?}: {}", std::io::Error::last_os_error()));
    }
    let held = String::from_utf16(&path[..length]).map_err(|_| format!("{real:?}: no UTF-16"))?;
    let held = Path::new(held.strip_prefix(r"\\?\").unwrap_or(&held));
    if held != real {
        return Err(format!("{real:?} moved to {held:?}"));
    }
    Ok(Opened { file })
}

impl Opened {
    /// Deletes the opened file itself, by its handle.
    pub fn delete(self) -> std::io::Result<()> {
        let info = FILE_DISPOSITION_INFO { DeleteFile: true };
        // SAFETY: an open handle with delete access, and its record.
        let done = unsafe {
            SetFileInformationByHandle(
                self.file.as_raw_handle(),
                FileDispositionInfo,
                (&info as *const FILE_DISPOSITION_INFO).cast(),
                std::mem::size_of::<FILE_DISPOSITION_INFO>() as u32,
            )
        };
        match done {
            0 => Err(std::io::Error::last_os_error()),
            _ => Ok(()),
        }
    }
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
                    let all = unsafe {
                        std::slice::from_raw_parts(view.Value.cast::<u8>(), info.RegionSize)
                    };
                    all[range].to_vec()
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
