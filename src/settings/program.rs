//! The program a shell setting names, as a session starts it.

use std::{env, ffi::OsStr, path::Path};

use anyhow::Context as _;

/// The program `shell` names, as a session starts it: a file at a full path, or the first in `PATH`'s full folders.
pub fn program_path(shell: &str) -> anyhow::Result<String> {
    program_path_in(shell, &env::var_os("PATH").unwrap_or_default())
}

/// The program `shell` names, in `folders` as `PATH` lists them.
pub fn program_path_in(shell: &str, folders: &impl AsRef<OsStr>) -> anyhow::Result<String> {
    find(shell, folders.as_ref(), cfg!(windows))
}

/// `shell`'s program, or the first one named so in `folders`, listed as `PATH` lists them; with `exe`, as on Windows, a name without an extension means name.exe.
pub(super) fn find(shell: &str, folders: &OsStr, exe: bool) -> anyhow::Result<String> {
    let path = Path::new(shell);
    // A relative path, or a relative folder, would hang on each session's own folder.
    let found = if path.is_absolute() {
        program(path).then(|| path.to_path_buf())
    } else if path.components().count() == 1 {
        let name = if exe && !shell.contains('.') {
            format!("{shell}.exe")
        } else {
            shell.to_string()
        };
        env::split_paths(folders)
            .filter(|folder| folder.is_absolute())
            .map(|folder| folder.join(&name))
            .find(|file| program(file))
    } else {
        None
    };
    let found = found.with_context(|| format!("shell is {shell:?}, which names no program"))?;
    let text = found.to_str().with_context(|| {
        format!(
            "shell is {shell:?}, found at {}, which is not UTF-8",
            found.display()
        )
    })?;
    Ok(text.to_string())
}

/// Whether `path` is a file the running user may run, as the system answers, ACLs too.
#[cfg(unix)]
pub(super) fn program(path: &Path) -> bool {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let Ok(name) = CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };
    path.is_file() && unsafe { libc::access(name.as_ptr(), libc::X_OK) } == 0
}

#[cfg(not(unix))]
pub(super) fn program(path: &Path) -> bool {
    path.is_file()
}
