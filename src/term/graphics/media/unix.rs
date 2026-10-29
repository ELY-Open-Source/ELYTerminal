use std::{
    ffi::{CString, OsStr},
    fs::File,
    os::unix::{ffi::OsStrExt, fs::OpenOptionsExt},
    path::{Path, PathBuf},
};

pub(super) fn path(bytes: &[u8]) -> Result<PathBuf, String> {
    if bytes.contains(&0) {
        return Err("a path holds a NUL".into());
    }
    Ok(PathBuf::from(OsStr::from_bytes(bytes)))
}

/// The path, links resolved, outside the kernel's folders.
pub(super) fn real(path: &Path) -> Result<PathBuf, String> {
    let real = std::fs::canonicalize(path).map_err(|error| format!("{path:?}: {error}"))?;
    let sensitive = ["/proc", "/sys", "/dev"]
        .iter()
        .any(|folder| real.starts_with(folder))
        && !real.starts_with("/dev/shm");
    if sensitive {
        return Err(format!("{real:?} lies in a kernel folder"));
    }
    Ok(real)
}

/// Opens following no link and waiting on no pipe.
pub(super) fn open(real: &Path) -> std::io::Result<File> {
    File::options()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(real)
}

/// What POSIX shared memory `name` holds, read, then unlinked.
pub(in super::super) fn shared(
    name: &[u8],
    offset: u32,
    size: u32,
    raw: Option<usize>,
) -> Result<Vec<u8>, String> {
    let shown = String::from_utf8_lossy(name).into_owned();
    // kitty passes the name to shm_open as it comes.
    let name = CString::new(name).map_err(|_| format!("{shown:?} holds a NUL"))?;
    // SAFETY: a NUL-ended name; the mode passes unread without O_CREAT.
    #[cfg(target_os = "macos")]
    let fd = unsafe { libc::shm_open(name.as_ptr(), libc::O_RDONLY, 0 as libc::c_uint) };
    // SAFETY: as above.
    #[cfg(not(target_os = "macos"))]
    let fd = unsafe { libc::shm_open(name.as_ptr(), libc::O_RDONLY, 0) };
    if fd < 0 {
        return Err(format!(
            "{shown:?} opens not: {}",
            std::io::Error::last_os_error()
        ));
    }
    let object = Object { fd, name };
    // SAFETY: an open descriptor and a zeroed stat to fill.
    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstat(object.fd, &mut stat) } != 0 {
        return Err(format!("{shown:?}: {}", std::io::Error::last_os_error()));
    }
    let available = usize::try_from(stat.st_size).map_err(|_| format!("{shown:?} has no size"))?;
    if available == 0 {
        return Err(format!("{shown:?} is empty"));
    }
    let range = super::range(available, offset, size, raw)
        .map_err(|error| format!("{shown:?}: {error}"))?;
    // SAFETY: a read-only shared map of the object's whole size.
    let map = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            available,
            libc::PROT_READ,
            libc::MAP_SHARED,
            object.fd,
            0,
        )
    };
    if map == libc::MAP_FAILED {
        return Err(format!(
            "{shown:?} maps not: {}",
            std::io::Error::last_os_error()
        ));
    }
    // SAFETY: the map holds `available` bytes; the range lies inside.
    let bytes = unsafe { std::slice::from_raw_parts(map.cast::<u8>(), available) }[range].to_vec();
    // SAFETY: the map made above, unused after this.
    unsafe { libc::munmap(map, available) };
    Ok(bytes)
}

/// An open object, closed and unlinked once read, as specified.
struct Object {
    fd: i32,
    name: CString,
}

impl Drop for Object {
    fn drop(&mut self) {
        // SAFETY: the descriptor shm_open gave, and its NUL-ended name.
        unsafe {
            libc::close(self.fd);
            libc::shm_unlink(self.name.as_ptr());
        }
    }
}
