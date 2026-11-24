//! The program a shell setting names, as a session starts it.

use std::{
    env,
    ffi::OsStr,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

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

/// On Windows, refuses a program that opens a window of its own, naming Git's `bin\bash.exe` beside it.
pub fn console(program: &str) -> anyhow::Result<()> {
    if !cfg!(windows) {
        return Ok(());
    }
    let path = Path::new(program);
    let subsystem = subsystem(path).with_context(|| format!("{program} does not read"))?;
    if subsystem != Some(WINDOWS_GUI) {
        return Ok(());
    }
    let bash = path.with_file_name("bin").join("bash.exe");
    if bash.is_file() {
        anyhow::bail!(
            "{program} opens a window of its own; Git's shell for a terminal is {}",
            bash.display()
        );
    }
    anyhow::bail!("{program} opens a window of its own, not a console")
}

/// PE's subsystem of a program with a window of its own.
const WINDOWS_GUI: u16 = 2;

/// The subsystem a PE file's optional header names; none for a file that is no PE program, a script among them.
fn subsystem(path: &Path) -> anyhow::Result<Option<u16>> {
    let mut file = File::open(path)?;
    let mut word = [0; 4];
    if file.read(&mut word[..2])? < 2 || word[..2] != *b"MZ" {
        return Ok(None);
    }
    file.seek(SeekFrom::Start(0x3c))?;
    file.read_exact(&mut word)?;
    let header = u64::from(u32::from_le_bytes(word));
    file.seek(SeekFrom::Start(header))?;
    file.read_exact(&mut word)?;
    anyhow::ensure!(word == *b"PE\0\0", "no PE header at {header}");
    let mut half = [0; 2];
    file.seek(SeekFrom::Start(header + 24 + 68))?;
    file.read_exact(&mut half)?;
    Ok(Some(u16::from_le_bytes(half)))
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

#[cfg(test)]
mod tests {
    use super::{WINDOWS_GUI, console, subsystem};

    /// A file of a PE header at 0x80 naming `kind` as its subsystem.
    fn pe(path: &std::path::Path, kind: u16) {
        let mut bytes = vec![0; 0x80 + 24 + 70];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[0x3c] = 0x80;
        bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
        bytes[0x80 + 24 + 68..][..2].copy_from_slice(&kind.to_le_bytes());
        std::fs::write(path, bytes).expect("written");
    }

    #[test]
    fn a_program_s_subsystem_reads_from_its_pe_header() {
        let folder = std::env::temp_dir().join(format!("ely-pe-{}", std::process::id()));
        std::fs::create_dir_all(folder.join("bin")).expect("a folder");
        let (gui, cli) = (
            folder.join("git-bash.exe"),
            folder.join("bin").join("bash.exe"),
        );
        pe(&gui, WINDOWS_GUI);
        pe(&cli, 3);
        assert_eq!(subsystem(&gui).expect("reads"), Some(WINDOWS_GUI));
        assert_eq!(subsystem(&cli).expect("reads"), Some(3));
        let script = folder.join("start.cmd");
        std::fs::write(&script, "@echo off\r\n").expect("written");
        assert_eq!(subsystem(&script).expect("reads"), None, "no PE program");
        console(script.to_str().expect("UTF-8")).expect("a script passes");
        std::fs::write(&script, "MZ").expect("written");
        assert!(subsystem(&script).is_err(), "a PE file cut short");
        let refused = console(gui.to_str().expect("UTF-8"));
        if cfg!(windows) {
            let error = refused.expect_err("a window of its own").to_string();
            assert!(error.contains(&cli.display().to_string()), "{error}");
            console(cli.to_str().expect("UTF-8")).expect("a console program");
        } else {
            refused.expect("only Windows asks");
        }
        std::fs::remove_dir_all(&folder).expect("cleaned");
    }
}
