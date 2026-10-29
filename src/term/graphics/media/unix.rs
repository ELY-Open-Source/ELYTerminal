use std::{
    ffi::{CString, OsStr},
    fs::File,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::ffi::OsStrExt,
    },
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

/// A file opened through the folder it was checked in.
pub(super) struct Opened {
    pub file: File,
    folder: OwnedFd,
    name: CString,
}

/// Opens a folder for search alone, all a read needs.
#[cfg(target_os = "macos")]
const SEARCH: i32 = libc::O_SEARCH;
#[cfg(not(target_os = "macos"))]
const SEARCH: i32 = libc::O_PATH;

/// Opens `real` in its folder, refusing one that moved since.
pub(super) fn open(real: &Path, _temporary: bool) -> Result<Opened, String> {
    let (Some(parent), Some(name)) = (real.parent(), real.file_name()) else {
        return Err(format!("{real:?} has no folder"));
    };
    let path = CString::new(parent.as_os_str().as_bytes())
        .map_err(|_| format!("{parent:?} holds a NUL"))?;
    let flags = SEARCH | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    // SAFETY: a NUL-ended path.
    let fd = unsafe { libc::open(path.as_ptr(), flags) };
    if fd < 0 {
        return Err(format!(
            "{parent:?} opens not: {}",
            std::io::Error::last_os_error()
        ));
    }
    // SAFETY: the descriptor just opened, owned by nothing else.
    let folder = unsafe { OwnedFd::from_raw_fd(fd) };
    let held = held(&folder)?;
    if held != parent {
        return Err(format!("{parent:?} moved to {held:?}"));
    }
    let name = CString::new(name.as_bytes()).map_err(|_| format!("{real:?} holds a NUL"))?;
    let flags = libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC;
    // SAFETY: an open folder and a NUL-ended name inside it.
    let fd = unsafe { libc::openat(folder.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(format!(
            "{real:?} opens not: {}",
            std::io::Error::last_os_error()
        ));
    }
    // SAFETY: the descriptor just opened, owned by nothing else.
    let file = unsafe { File::from_raw_fd(fd) };
    Ok(Opened { file, folder, name })
}

impl Opened {
    /// Unlinks the opened name in its held folder.
    pub fn delete(self) -> std::io::Result<()> {
        // SAFETY: the held folder and a NUL-ended name inside it.
        match unsafe { libc::unlinkat(self.folder.as_raw_fd(), self.name.as_ptr(), 0) } {
            0 => Ok(()),
            _ => Err(std::io::Error::last_os_error()),
        }
    }
}

/// Where an open folder lies now.
#[cfg(target_os = "macos")]
fn held(folder: &OwnedFd) -> Result<PathBuf, String> {
    let mut path = vec![0u8; libc::PATH_MAX as usize];
    // SAFETY: an open descriptor and a buffer of PATH_MAX bytes.
    if unsafe { libc::fcntl(folder.as_raw_fd(), libc::F_GETPATH, path.as_mut_ptr()) } != 0 {
        return Err(format!("no path: {}", std::io::Error::last_os_error()));
    }
    let path = std::ffi::CStr::from_bytes_until_nul(&path).map_err(|_| "no NUL".to_string())?;
    Ok(PathBuf::from(OsStr::from_bytes(path.to_bytes())))
}

/// Where an open folder lies now.
#[cfg(not(target_os = "macos"))]
fn held(folder: &OwnedFd) -> Result<PathBuf, String> {
    let fd = format!("/proc/self/fd/{}", folder.as_raw_fd());
    std::fs::read_link(&fd).map_err(|error| format!("{fd}: {error}"))
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
    // glibc opens under /dev/shm, where a pipe would block.
    #[cfg(not(target_os = "macos"))]
    // SAFETY: as above.
    let fd = unsafe { libc::shm_open(name.as_ptr(), libc::O_RDONLY | libc::O_NONBLOCK, 0) };
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
    // macOS objects are no files; Linux's are, under /dev/shm.
    #[cfg(not(target_os = "macos"))]
    if stat.st_mode & libc::S_IFMT != libc::S_IFREG {
        return Err(format!("{shown:?} is no memory object"));
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
