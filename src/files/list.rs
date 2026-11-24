//! A folder's entries, folders first.

use std::{ffi::OsString, fs, io, path::Path, time::SystemTime};

/// A folder's entry: name, whether a folder, and modified time.
#[derive(Debug, PartialEq)]
pub(super) struct Entry {
    pub(super) name: OsString,
    pub(super) folder: bool,
    pub(super) modified: SystemTime,
}

/// A folder's entries, or why it could not be read.
pub(super) type Listing = Result<Vec<Entry>, String>;

/// Reads `folder`: folders first, then files, each by name ignoring case.
pub(super) fn list(folder: &Path) -> Listing {
    let read = || -> io::Result<Vec<Entry>> {
        let mut entries = Vec::new();
        for entry in fs::read_dir(folder)? {
            let entry = entry?;
            // An entry gone since the folder listed it is gone.
            let meta = match entry.metadata() {
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                meta => meta?,
            };
            let kind = meta.file_type();
            entries.push(Entry {
                folder: kind.is_dir() || (kind.is_symlink() && entry.path().is_dir()),
                name: entry.file_name(),
                modified: meta.modified()?,
            });
        }
        Ok(entries)
    };
    let mut entries = read().map_err(|error| error.to_string())?;
    sort(&mut entries);
    Ok(entries)
}

/// Folders first, then files, each by name ignoring case.
pub(super) fn sort(entries: &mut [Entry]) {
    entries.sort_by_cached_key(|entry| {
        let name = entry.name.to_string_lossy().to_lowercase();
        (!entry.folder, name, entry.name.clone())
    });
}
