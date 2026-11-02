//! Windows' shells as Windows Terminal finds them: PowerShell 7, Windows PowerShell, cmd, Git Bash and each WSL distribution.

use std::path::Path;

use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

use super::Found;
use crate::{
    settings,
    term::{env, shell},
};

/// Where WSL lists each distribution, one subkey each.
const LXSS: &str = r"Software\Microsoft\Windows\CurrentVersion\Lxss";

/// The shells this machine holds, in Windows Terminal's order.
pub fn detected() -> Vec<Found> {
    let mut found = Vec::new();
    match env::path() {
        Ok(path) => {
            for name in ["pwsh", "powershell", "cmd"] {
                if let Some(program) = shell::found(name, &path) {
                    found.push(Found {
                        name: name.into(),
                        words: vec![program],
                    });
                }
            }
        }
        Err(error) => log::error!("profiles: no PATH to find shells on: {error:#}"),
    }
    match git_bash() {
        Ok(Some(bash)) => found.push(Found {
            name: "Git Bash".into(),
            words: vec![bash],
        }),
        Ok(None) => log::info!("profiles: no Git for Windows"),
        Err(error) => log::error!("profiles: {error:#}"),
    }
    match distributions() {
        Ok(names) if settings::program_path("wsl").is_ok() => {
            for name in names {
                let words = ["wsl", "-d", &name].map(String::from).into();
                found.push(Found { name, words });
            }
        }
        Ok(_) => log::info!("profiles: no wsl on PATH"),
        Err(error) => log::error!("profiles: {error:#}"),
    }
    let names: Vec<_> = found.iter().map(|found| found.name.as_str()).collect();
    log::info!("profiles: found {names:?}");
    found
}

/// Git for Windows' bash, where its installer recorded it.
fn git_bash() -> anyhow::Result<Option<String>> {
    for root in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        let values = env::read(root, r"SOFTWARE\GitForWindows")?;
        let install = values.into_iter().find(|(name, ..)| name == "InstallPath");
        if let Some((_, _, install)) = install {
            let bash = Path::new(&install).join(r"bin\bash.exe");
            if bash.is_file() {
                return Ok(Some(bash.display().to_string()));
            }
            log::warn!("profiles: Git for Windows holds no {}", bash.display());
        }
    }
    Ok(None)
}

/// Each distribution WSL lists, Docker Desktop's own passed by, as Windows Terminal passes them.
fn distributions() -> anyhow::Result<Vec<String>> {
    let mut names = Vec::new();
    for key in env::subkeys(HKEY_CURRENT_USER, LXSS)? {
        let values = env::read(HKEY_CURRENT_USER, &format!(r"{LXSS}\{key}"))?;
        let name = values
            .into_iter()
            .find(|(name, ..)| name == "DistributionName");
        match name {
            Some((_, _, name)) if name.starts_with("docker-desktop") => {}
            Some((_, _, name)) => names.push(name),
            None => log::warn!("profiles: WSL's {key} names no distribution"),
        }
    }
    Ok(names)
}
