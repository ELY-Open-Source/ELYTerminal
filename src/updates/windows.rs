//! On Windows a release's zip: its files over this copy's folder, each file in use moved aside and removed at the next start.

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, ensure};

use super::install::run;
use crate::registry;

/// The installer's entry in Settings › Apps, for the current user.
const KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\ELYTerminal";
/// What a replaced file's name gains until the next start removes it, a name only an update makes.
const ASIDE: &str = ".elyterminal-old";

/// The folder this copy runs from, refused where it cannot be written.
pub fn folder() -> anyhow::Result<PathBuf> {
    let exe = std::env::current_exe().context("updates: this copy's path does not read")?;
    let folder = exe
        .parent()
        .context("updates: this copy lies in no folder")?;
    let probe = folder.join(format!(".elyterminal-probe-{}", std::process::id()));
    std::fs::File::create_new(&probe)
        .with_context(|| format!("updates: {} is not writable", folder.display()))?;
    std::fs::remove_file(&probe)
        .with_context(|| format!("updates: cannot remove {}", probe.display()))?;
    Ok(folder.to_path_buf())
}

/// Unpacks `zip` beside `folder`'s files and puts each in place; the installer's entry takes its version where it names `folder`.
pub fn place(zip: &Path, folder: &Path) -> anyhow::Result<()> {
    let staged = folder.join(".elyterminal-update");
    if staged.exists() {
        std::fs::remove_dir_all(&staged)
            .with_context(|| format!("updates: cannot remove {}", staged.display()))?;
    }
    std::fs::create_dir(&staged)
        .with_context(|| format!("updates: cannot create {}", staged.display()))?;
    run(
        "tar",
        &[
            "-xf".as_ref(),
            zip.as_os_str(),
            "-C".as_ref(),
            staged.as_os_str(),
        ],
    )?;
    let mut names = Vec::new();
    for entry in std::fs::read_dir(&staged)
        .with_context(|| format!("updates: cannot list {}", staged.display()))?
    {
        names.push(
            entry
                .context("updates: the zip's files do not list")?
                .file_name(),
        );
    }
    ensure!(
        names.iter().any(|name| name == "elyterminal.exe"),
        "updates: {} holds no elyterminal.exe",
        zip.display()
    );
    swap_all(&staged, folder, &names)?;
    std::fs::remove_dir(&staged)
        .with_context(|| format!("updates: cannot remove {}", staged.display()))?;
    log::info!(
        "updates: {} holds the release's {} files",
        folder.display(),
        names.len()
    );
    entry(zip, folder)
}

/// Puts each of `names` from `staged` in `folder`; a failure puts back the files swapped before it.
fn swap_all(staged: &Path, folder: &Path, names: &[std::ffi::OsString]) -> anyhow::Result<()> {
    for (done, name) in names.iter().enumerate() {
        let Err(error) = swap(&staged.join(name), &folder.join(name)) else {
            continue;
        };
        for name in names[..done].iter().rev() {
            let at = folder.join(name);
            let back =
                std::fs::rename(&at, staged.join(name)).and_then(|()| match aside(&at).exists() {
                    true => std::fs::rename(aside(&at), &at),
                    false => Ok(()),
                });
            if let Err(stuck) = back {
                log::error!("updates: {} stays the release's: {stuck}", at.display());
            }
        }
        return Err(error.context("updates: the files swapped before it are put back"));
    }
    Ok(())
}

/// Puts `new` at `at`, the file there, running or not, moved aside first.
fn swap(new: &Path, at: &Path) -> anyhow::Result<()> {
    let aside = aside(at);
    if aside.exists() {
        std::fs::remove_file(&aside)
            .with_context(|| format!("updates: cannot remove {}", aside.display()))?;
    }
    let moved = at.exists();
    if moved {
        std::fs::rename(at, &aside)
            .with_context(|| format!("updates: cannot move {} aside", at.display()))?;
    }
    if let Err(error) = std::fs::rename(new, at) {
        if moved {
            std::fs::rename(&aside, at)
                .with_context(|| format!("updates: {} could not come back", at.display()))?;
        }
        return Err(error).with_context(|| format!("updates: cannot put {}", at.display()));
    }
    Ok(())
}

fn aside(at: &Path) -> PathBuf {
    let mut name = at.as_os_str().to_os_string();
    name.push(ASIDE);
    PathBuf::from(name)
}

/// The installer's entry takes the zip's version, where it names `folder`.
fn entry(zip: &Path, folder: &Path) -> anyhow::Result<()> {
    let Some(installed) = registry::text(KEY, Some("InstallLocation"))? else {
        log::info!("updates: no installer's entry names this copy");
        return Ok(());
    };
    let trim = |path: &OsStr| path.to_string_lossy().trim_end_matches('\\').to_lowercase();
    if trim(&installed) != trim(folder.as_os_str()) {
        log::info!(
            "updates: the installer's entry names {}, not this copy",
            installed.display()
        );
        return Ok(());
    }
    let name = zip.file_name().and_then(OsStr::to_str).unwrap_or_default();
    let version = (name.strip_prefix("ELYTerminal-"))
        .and_then(|rest| rest.split_once("-windows-"))
        .map(|(version, _)| version)
        .with_context(|| format!("updates: {name} names no version"))?;
    registry::set(KEY, Some("DisplayVersion"), version.as_ref())?;
    log::info!("updates: the installer's entry says {version}");
    Ok(())
}

/// Removes the files an update moved aside, which their processes held until now.
pub fn sweep() {
    let folder = match std::env::current_exe() {
        Ok(exe) => exe.parent().map(Path::to_path_buf),
        Err(error) => return log::error!("updates: this copy's path does not read: {error}"),
    };
    let Some(folder) = folder else {
        return;
    };
    let entries = match std::fs::read_dir(&folder) {
        Ok(entries) => entries,
        Err(error) => return log::error!("updates: cannot list {}: {error}", folder.display()),
    };
    for path in entries.filter_map(|entry| entry.ok().map(|entry| entry.path())) {
        if !path.to_string_lossy().ends_with(ASIDE) {
            continue;
        }
        match std::fs::remove_file(&path) {
            Ok(()) => log::info!("updates: removed {}", path.display()),
            Err(error) => log::warn!(
                "updates: {} stays until a later start: {error}",
                path.display()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_swap_puts_back_the_files_swapped_before_it() {
        let folder = std::env::temp_dir().join(format!("ely-swap-all-{}", std::process::id()));
        let staged = folder.join("staged");
        std::fs::create_dir_all(&staged).expect("folders");
        std::fs::write(staged.join("conpty.dll"), b"2").expect("a new file");
        std::fs::write(folder.join("conpty.dll"), b"1").expect("an old file");
        std::fs::write(folder.join("elyterminal.exe"), b"1").expect("an old file");
        let names = ["conpty.dll".into(), "elyterminal.exe".into()];
        swap_all(&staged, &folder, &names).expect_err("elyterminal.exe is not staged");
        assert_eq!(
            std::fs::read(folder.join("conpty.dll")).expect("reads"),
            b"1"
        );
        assert_eq!(
            std::fs::read(folder.join("elyterminal.exe")).expect("reads"),
            b"1"
        );
        assert!(
            !aside(&folder.join("conpty.dll")).exists(),
            "nothing left aside"
        );
        std::fs::remove_dir_all(&folder).expect("the folder goes");
    }

    #[test]
    fn a_running_file_moves_aside_and_a_new_one_takes_its_name() {
        let folder = std::env::temp_dir().join(format!("ely-swap-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("a folder");
        let (new, at) = (folder.join("new"), folder.join("elyterminal.exe"));
        std::fs::write(&new, b"2").expect("the new file");
        std::fs::write(&at, b"1").expect("the old file");
        swap(&new, &at).expect("swaps");
        assert_eq!(std::fs::read(&at).expect("reads"), b"2");
        assert_eq!(std::fs::read(aside(&at)).expect("aside"), b"1");
        std::fs::remove_dir_all(&folder).expect("the folder goes");
    }
}
