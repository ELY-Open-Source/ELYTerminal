use std::{
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use anyhow::Context as _;
use gpui::{App, Global};

use super::{drop::Quote, pty::Launch};

#[cfg(any(windows, test))]
pub(super) mod cmd;

/// Each script, by its path under the integration folder.
const SCRIPTS: [(&str, &str); 5] = [
    (
        "zsh/.zshenv",
        include_str!("../../assets/shell-integration/zsh/.zshenv"),
    ),
    (
        "bash/elyterminal.bash",
        include_str!("../../assets/shell-integration/bash/elyterminal.bash"),
    ),
    (
        "fish/vendor_conf.d/elyterminal.fish",
        include_str!("../../assets/shell-integration/fish/vendor_conf.d/elyterminal.fish"),
    ),
    (
        "pwsh/elyterminal.ps1",
        include_str!("../../assets/shell-integration/pwsh/elyterminal.ps1"),
    ),
    (
        "wsl/elyterminal.sh",
        include_str!("../../assets/shell-integration/wsl/elyterminal.sh"),
    ),
];

/// The folder the scripts live in, written at start.
pub struct ShellIntegration(pub PathBuf);

impl Global for ShellIntegration {}

impl ShellIntegration {
    /// Writes the scripts into the platform's data folder and keeps its path.
    pub fn init(cx: &mut App) -> anyhow::Result<()> {
        let dir = dirs::data_local_dir()
            .context("shell integration: the platform names no data folder")?
            .join("ELYTerminal")
            .join("shell-integration");
        install(&dir)?;
        cx.set_global(Self(dir));
        Ok(())
    }
}

/// Writes the scripts into `dir`, replacing any that differ.
pub fn install(dir: &Path) -> anyhow::Result<()> {
    for (name, text) in SCRIPTS {
        let path = dir.join(name);
        match fs::read_to_string(&path) {
            Ok(kept) if kept == text => continue,
            Ok(_) => {}
            Err(error) if matches!(error.kind(), ErrorKind::NotFound | ErrorKind::InvalidData) => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("shell integration: cannot read {}", path.display()));
            }
        }
        let folder = path
            .parent()
            .context("shell integration: a script has no folder")?;
        fs::create_dir_all(folder)
            .with_context(|| format!("shell integration: cannot create {}", folder.display()))?;
        fs::write(&path, text)
            .with_context(|| format!("shell integration: cannot write {}", path.display()))?;
        log::info!("shell integration: wrote {}", path.display());
    }
    Ok(())
}

/// Home, which `wsl` takes as its distribution's, or a folder.
pub enum Start {
    Home,
    In(PathBuf),
}

impl From<PathBuf> for Start {
    fn from(folder: PathBuf) -> Self {
        Self::In(folder)
    }
}

/// A program's name, as the integration knows shells.
fn name(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// Whether `folder` lies on a share, `\\server\share`.
#[cfg(windows)]
fn unc(folder: &Path) -> bool {
    use std::path::{Component, Prefix};
    let first = folder.components().next();
    matches!(first, Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::UNC(..)))
}

/// How to start `shell`, or the login shell, at `start`.
pub fn launch(shell: &str, start: impl Into<Start>, dir: &Path) -> anyhow::Result<Launch> {
    let start = start.into();
    let cwd = match &start {
        Start::Home => dirs::home_dir().context("shell integration: no home folder")?,
        Start::In(folder) => folder.clone(),
    };
    let path = if shell.is_empty() {
        default_shell()?
    } else {
        // The settings checked this path; the session runs no other.
        crate::settings::program_path(shell).context("shell integration")?
    };
    let name = name(Path::new(&path));
    #[cfg(windows)]
    let name = if name == "sh" && git_bash(Path::new(&path)) {
        log::info!("shell integration: {path} is Git for Windows' sh, which runs its bash");
        "bash".to_string()
    } else {
        name
    };
    let mut launch = Launch {
        program: (!shell.is_empty()).then(|| (path.clone(), Vec::new())),
        cwd,
        env: Vec::new(),
        reports: true,
        quote: match name.as_str() {
            "pwsh" | "powershell" => Quote::PowerShell,
            "cmd" => Quote::Cmd,
            _ => Quote::Posix,
        },
    };
    let script = |name: &str| dir.join(name).display().to_string();
    match name.as_str() {
        "zsh" => {
            if let Ok(zdotdir) = env::var("ZDOTDIR") {
                launch.env.push(("ELY_ZSH_ZDOTDIR".into(), zdotdir));
            }
            launch.env.push(("ZDOTDIR".into(), script("zsh")));
        }
        "bash" => {
            let rcfile = script("bash/elyterminal.bash");
            launch.program = Some((path.clone(), vec!["--rcfile".into(), rcfile]));
            if cfg!(target_os = "macos") {
                launch.env.push(("ELY_BASH_LOGIN".into(), "1".into()));
            }
        }
        "fish" => {
            let previous = env::var("XDG_DATA_DIRS").unwrap_or_default();
            let rest = if previous.is_empty() {
                "/usr/local/share:/usr/share"
            } else {
                &previous
            };
            let data = format!("{}:{rest}", dir.display());
            launch.env.push(("XDG_DATA_DIRS".into(), data));
            launch.env.push(("ELY_FISH_XDG_DATA_DIRS".into(), previous));
        }
        "pwsh" | "powershell" => {
            let file = script("pwsh/elyterminal.ps1").replace('\'', "''");
            launch.program = Some((
                path.clone(),
                vec![
                    "-NoLogo".into(),
                    "-NoExit".into(),
                    "-ExecutionPolicy".into(),
                    "Bypass".into(),
                    "-Command".into(),
                    format!(
                        "try {{ . '{file}' }} catch {{ [Console]::Error.WriteLine(\"ELYTerminal: shell integration failed: $_\"); exit 1 }}"
                    ),
                ],
            ));
            // A bare cmd typed there takes this prompt.
            #[cfg(windows)]
            {
                let prompt = cmd::prompt(super::env::fresh()?.get("PROMPT"), false);
                launch.env.push(("ELY_CMD_PROMPT".into(), prompt));
            }
        }
        // WSL starts the distribution's login shell through the launcher, which loads its integration.
        #[cfg(windows)]
        "wsl" => {
            let launcher = r#"exec /bin/sh "$ELY_WSL/wsl/elyterminal.sh""#;
            let mut args: Vec<String> = ["-e", "/bin/sh", "-c", launcher].map(String::from).into();
            match &start {
                // Home is the distribution's own, which WSL resolves, not Windows'.
                Start::Home => {
                    args.splice(0..0, ["--cd".to_string(), "~".to_string()]);
                    log::info!("shell integration: wsl starts in its distribution's home");
                }
                // A share's folder lies in its own distribution.
                Start::In(folder) => {
                    if let Some((share, inside)) = crate::git::wsl::split(folder) {
                        let distro = crate::git::wsl::distro(share).to_string();
                        log::info!("shell integration: wsl starts {distro} in {inside}");
                        args.splice(0..0, ["-d".into(), distro, "--cd".into(), inside]);
                    }
                }
            }
            launch.program = Some((path.clone(), args));
            let shared = super::env::fresh()?.get("WSLENV").map(str::to_string);
            let shared = shared.map_or("ELY_WSL/p".into(), |list| format!("{list}:ELY_WSL/p"));
            launch
                .env
                .push(("ELY_WSL".into(), dir.display().to_string()));
            launch.env.push(("WSLENV".into(), shared));
        }
        // Only cmd's own environment: there a batch's cd is cmd's.
        #[cfg(windows)]
        "cmd" => {
            // cmd refuses a share as its folder, saying so.
            if let Start::In(folder) = &start
                && unc(folder)
            {
                log::warn!(
                    "shell integration: cmd starts in Windows' folder, not {}",
                    folder.display()
                );
            }
            launch.program = Some((path.clone(), Vec::new()));
            let prompt = cmd::prompt(super::env::fresh()?.get("PROMPT"), true);
            launch.env.push(("PROMPT".into(), prompt));
        }
        _ => {
            launch.reports = false;
            // Windows has no login shell of its own, so the shell that was chosen runs as it is.
            #[cfg(windows)]
            {
                launch.program = Some((path.clone(), Vec::new()));
            }
            log::info!("shell integration: none for {path}; its folder goes unreported")
        }
    }
    log::info!("shell integration: {path} as {name}");
    Ok(launch)
}

#[cfg(unix)]
fn default_shell() -> anyhow::Result<String> {
    Ok(super::unix::login_shell_path()?)
}

/// `pwsh`, as Windows Terminal takes it, else the Windows PowerShell the system ships, else `cmd`.
#[cfg(windows)]
fn default_shell() -> anyhow::Result<String> {
    // The registry's PATH, so a shell in a folder added since this process opened is found.
    first_shell(&super::env::path()?)
}

/// The first of `pwsh`, `powershell` and `cmd` that `folders` hold outside `WindowsApps`.
#[cfg(windows)]
fn first_shell(folders: &str) -> anyhow::Result<String> {
    for name in ["pwsh", "powershell", "cmd"] {
        for folder in env::split_paths(folders) {
            // One folder as a PATH of its own, so a `;` inside its name stays.
            let Ok(one) = env::join_paths([&folder]) else {
                continue;
            };
            match crate::settings::program_path_in(name, &one) {
                // An App Execution Alias is a stub that starts only while its package is installed.
                Ok(found) if alias(&found) => {
                    log::info!("shell integration: {found} is an alias, so passed by")
                }
                Ok(found) => return Ok(found),
                Err(_) => {}
            }
        }
        log::info!("shell integration: no {name} on PATH");
    }
    anyhow::bail!("shell integration: neither pwsh, powershell nor cmd lies on PATH")
}

/// Whether `sh` is Git for Windows' `bin\sh.exe`, a bash launcher.
#[cfg(windows)]
fn git_bash(sh: &Path) -> bool {
    sh.parent()
        .filter(|bin| {
            bin.file_name()
                .is_some_and(|name| name.eq_ignore_ascii_case("bin"))
        })
        .and_then(Path::parent)
        .is_some_and(|root| root.join("usr").join("bin").join("bash.exe").is_file())
}

/// Whether `path` lies in the folder Windows puts its App Execution Aliases in.
#[cfg(windows)]
fn alias(path: &str) -> bool {
    Path::new(path)
        .parent()
        .and_then(Path::file_name)
        .is_some_and(|name| name.eq_ignore_ascii_case("WindowsApps"))
}

mod options;
#[cfg(unix)]
pub(super) use options::clicks;
pub(super) use options::options;
#[cfg(unix)]
pub(super) use options::relays;

#[cfg(all(test, unix))]
mod tests;

#[cfg(all(test, windows))]
mod windows_tests;
