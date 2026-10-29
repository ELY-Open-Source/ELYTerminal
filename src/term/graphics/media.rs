//! A local client's data: file, temporary file, shared memory.

use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

#[cfg(unix)]
#[path = "media/unix.rs"]
mod system;
#[cfg(windows)]
#[path = "media/windows.rs"]
mod system;

pub(super) use system::shared;

#[cfg(test)]
mod tests;

/// kitty's bound on any one image's data.
pub(super) const MAX: usize = 400 * 1024 * 1024;

/// `size` bytes from `offset` in the file, or why not.
pub(super) fn file(
    path: &[u8],
    temporary: bool,
    offset: u32,
    size: u32,
) -> Result<Vec<u8>, String> {
    let path = system::path(path)?;
    if !path.is_absolute() {
        return Err(format!("{path:?} is no full path"));
    }
    // Resolved before opening: a device's open can block.
    let real = system::real(&path)?;
    if temporary {
        temporaries(&real)?;
    }
    let kind = std::fs::metadata(&real).map_err(|error| format!("{real:?}: {error}"))?;
    if !kind.is_file() {
        return Err(format!("{real:?} is no regular file"));
    }
    let mut opened = system::open(&real, temporary)?;
    let read = read(&mut opened.file, &real, offset, size);
    if temporary {
        match opened.delete() {
            Ok(()) => log::debug!("graphics: temporary file {real:?} deleted"),
            Err(error) => log::warn!("graphics: temporary file {real:?} stays: {error}"),
        }
    }
    read
}

fn read(file: &mut File, real: &Path, offset: u32, size: u32) -> Result<Vec<u8>, String> {
    let opened = file
        .metadata()
        .map_err(|error| format!("{real:?}: {error}"))?;
    if !opened.is_file() {
        return Err(format!("{real:?} turned into no regular file"));
    }
    if u64::from(offset) > opened.len() {
        return Err(format!("{real:?} ends before {offset}"));
    }
    file.seek(SeekFrom::Start(u64::from(offset)))
        .map_err(|error| format!("{real:?} seeks not to {offset}: {error}"))?;
    read_to(file, size).map_err(|error| format!("{real:?}: {error}"))
}

/// Exactly `size` bytes, or to the end when 0.
fn read_to(file: &mut File, size: u32) -> Result<Vec<u8>, String> {
    let size = size as usize;
    if size > MAX {
        return Err(format!("{size} bytes pass {MAX}"));
    }
    if size > 0 {
        let mut bytes = vec![0; size];
        file.read_exact(&mut bytes)
            .map_err(|error| format!("{size} bytes read not: {error}"))?;
        return Ok(bytes);
    }
    let mut bytes = Vec::new();
    file.take(MAX as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > MAX {
        return Err(format!("the file passes {MAX} bytes"));
    }
    Ok(bytes)
}

/// In a temporary folder, naming the protocol, as kitty asks.
fn temporaries(real: &Path) -> Result<(), String> {
    if !real.to_string_lossy().contains("tty-graphics-protocol") {
        return Err(format!("{real:?} does not name tty-graphics-protocol"));
    }
    let mut folders: Vec<PathBuf> = vec!["/tmp".into(), "/dev/shm".into(), std::env::temp_dir()];
    folders.extend(std::env::var_os("TMPDIR").map(PathBuf::from));
    let inside = folders
        .iter()
        .filter_map(|folder| system::real(folder).ok())
        .any(|folder| real.starts_with(folder));
    if !inside {
        return Err(format!("{real:?} lies in no temporary folder"));
    }
    Ok(())
}

/// The bytes from `offset` in an object of `available`.
pub(super) fn range(
    available: usize,
    offset: u32,
    size: u32,
    raw: Option<usize>,
) -> Result<std::ops::Range<usize>, String> {
    let start = offset as usize;
    if start > available {
        return Err(format!("offset {start} passes its {available} bytes"));
    }
    let size = match (size, raw) {
        (0, Some(raw)) => raw,
        (0, None) => available - start,
        (size, _) => size as usize,
    };
    if size > MAX || size > available - start {
        return Err(format!("{size} bytes from {start} pass its {available}"));
    }
    Ok(start..start + size)
}
