use std::{
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use anyhow::Context as _;
use gpui::{App, Global};

use super::{drop::Quote, pty::Launch};

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
        }
        // WSL starts the distribution's login shell through the launcher, which loads its integration.
        #[cfg(windows)]
        "wsl" => {
            let launcher = r#"exec /bin/sh "$ELY_WSL/wsl/elyterminal.sh""#;
            let mut args: Vec<String> = ["-e", "/bin/sh", "-c", launcher].map(String::from).into();
            // Home is the distribution's own, which WSL resolves, not Windows'.
            if matches!(start, Start::Home) {
                args.splice(0..0, ["--cd".to_string(), "~".to_string()]);
                log::info!("shell integration: wsl starts in its distribution's home");
            }
            launch.program = Some((path.clone(), args));
            let shared = super::env::fresh()?.get("WSLENV").map(str::to_string);
            let shared = shared.map_or("ELY_WSL/p".into(), |list| format!("{list}:ELY_WSL/p"));
            launch
                .env
                .push(("ELY_WSL".into(), dir.display().to_string()));
            launch.env.push(("WSLENV".into(), shared));
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
mod windows_tests {
    use std::time::{Duration, Instant};

    use alacritty_terminal::event::Event;
    use futures::channel::mpsc::TryRecvError;

    use super::{SCRIPTS, Start, alias, first_shell, git_bash, install, launch};
    use crate::{
        settings::Settings,
        term::{Events, PtyEvent, config, spawn},
    };

    #[test]
    fn the_scripts_wsl_runs_end_lines_as_unix_shells_read_them() {
        // PowerShell reads a carriage return; WSL's shells do not.
        for (name, text) in SCRIPTS
            .iter()
            .filter(|(name, _)| !name.starts_with("pwsh/"))
        {
            assert!(!text.contains('\r'), "{name} holds a carriage return");
        }
    }

    #[test]
    fn wsl_starts_in_its_own_home_or_in_the_windows_folder_given() {
        let dir = std::env::temp_dir().join("elyterminal-wsl-launch");
        let home = launch("wsl", Start::Home, &dir).expect("a launch");
        let (_, args) = home.program.expect("wsl");
        assert_eq!(args[..3], ["--cd", "~", "-e"]);
        let folder = std::path::PathBuf::from(r"C:\Windows");
        let there = launch("wsl", Start::In(folder.clone()), &dir).expect("a launch");
        assert_eq!(there.cwd, folder);
        assert_eq!(there.program.expect("wsl").1[0], "-e");
    }

    #[test]
    fn an_alias_hides_no_pwsh_later_on_path() {
        let root = std::env::temp_dir().join(format!("elyterminal-shells-{}", std::process::id()));
        let folders = ["WindowsApps", "Tools;PS7", "System32"].map(|name| root.join(name));
        for (folder, file) in folders
            .iter()
            .zip(["pwsh.exe", "pwsh.exe", "powershell.exe"])
        {
            std::fs::create_dir_all(folder).expect("a folder");
            std::fs::write(folder.join(file), "").expect("a program");
        }
        let path = std::env::join_paths(&folders).expect("a PATH");
        let found = first_shell(path.to_str().expect("text"));
        std::fs::remove_dir_all(&root).expect("cleanup");
        assert_eq!(
            found.expect("a shell"),
            folders[1].join("pwsh.exe").display().to_string()
        );
    }

    #[test]
    fn an_app_execution_alias_is_recognised_by_its_folder() {
        assert!(alias(
            "C:\\Users\\x\\AppData\\Local\\Microsoft\\WindowsApps\\pwsh.exe"
        ));
        assert!(alias(
            "C:\\Users\\x\\AppData\\Local\\Microsoft\\WINDOWSAPPS\\pwsh.exe"
        ));
        assert!(!alias("C:\\Program Files\\PowerShell\\7\\pwsh.exe"));
        assert!(!alias(
            "C:\\WINDOWS\\System32\\WindowsPowerShell\\v1.0\\powershell.exe"
        ));
        assert!(!alias("pwsh.exe"));
    }

    #[test]
    fn git_bash_is_known_by_its_sh_in_bin_beside_usr_bin_bash() {
        let root = std::env::temp_dir().join(format!("elyterminal-git-{}", std::process::id()));
        let file = |path: &str| {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().expect("a folder")).expect("a folder");
            std::fs::write(&path, "").expect("a program");
            path
        };
        let git = file("Git\\bin\\sh.exe");
        file("Git\\usr\\bin\\bash.exe");
        // MSYS2's sh, bash in POSIX mode, reads no --rcfile.
        let msys = file("msys64\\usr\\bin\\sh.exe");
        file("msys64\\usr\\bin\\bash.exe");
        let alone = file("Tools\\bin\\sh.exe");
        let found = [&git, &msys, &alone].map(|sh| git_bash(sh));
        std::fs::remove_dir_all(&root).expect("cleanup");
        assert_eq!(found, [true, false, false]);
    }

    /// The next folder the session reports, within 30 seconds.
    fn next_folder(events: &mut Events) -> std::path::PathBuf {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            match events.try_recv() {
                Ok(PtyEvent::Folder(folder)) => return folder,
                Ok(_) => {}
                Err(TryRecvError::Closed) => {
                    panic!("the session ended before it reported a folder")
                }
                Err(TryRecvError::Empty) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(50))
                }
                Err(TryRecvError::Empty) => panic!("no folder within 30 seconds"),
            }
        }
    }

    #[test]
    fn git_bash_s_sh_reports_its_folder_as_windows_names_it() {
        let found = std::process::Command::new("git")
            .arg("--exec-path")
            .output()
            .expect("git runs");
        let exec = String::from_utf8(found.stdout).expect("utf-8");
        // ROOT/mingw64/libexec/git-core, or ucrt64.
        let git = std::path::Path::new(exec.trim())
            .ancestors()
            .nth(3)
            .expect("Git's folder");
        let root = std::env::temp_dir().join(format!("elyterminal-gitbash-{}", std::process::id()));
        let (dir, start) = (root.join("integration"), root.join("start here"));
        std::fs::create_dir_all(start.join("in ner")).expect("the folders");
        install(&dir).expect("the scripts");
        let sh = git.join("bin").join("sh.exe");
        let launch = launch(sh.to_str().expect("text"), start.clone(), &dir).expect("a launch");
        let args = launch.program.as_ref().map(|(_, args)| args.clone());
        assert_eq!(args.expect("arguments")[0], "--rcfile");
        let (pty, mut events) = spawn(&launch, (80, 24), (8, 16), config(&Settings::default()))
            .expect("Git Bash starts");
        let first = next_folder(&mut events);
        pty.write(&b"cd 'in ner'\r"[..]);
        let second = next_folder(&mut events);
        // Controls and colons sit at U+F000 up; nl\n beside nl.
        pty.write(&b"mkdir $'bell\\adir' re:po $'nl\\n' nl && cd $'bell\\adir'\r"[..]);
        let bell = next_folder(&mut events);
        pty.write(&b"cd ../re:po\r"[..]);
        let colon = next_folder(&mut events);
        pty.write(&b"cd $'../nl\\n'\r"[..]);
        let newline = next_folder(&mut events);
        pty.write(&b"exit\r"[..]);
        let deadline = Instant::now() + Duration::from_secs(30);
        while !matches!(
            events.try_recv(),
            Ok(PtyEvent::Term(Event::Exit)) | Err(TryRecvError::Closed)
        ) {
            assert!(Instant::now() < deadline, "Git Bash did not exit");
            std::thread::sleep(Duration::from_millis(50));
        }
        let real = |path: &std::path::Path| path.canonicalize().expect("a real folder");
        assert_eq!(real(&first), real(&start));
        let inner = start.join("in ner");
        assert_eq!(real(&second), real(&inner));
        assert_eq!(real(&bell), real(&inner.join("bell\u{f007}dir")));
        assert_eq!(real(&colon), real(&inner.join("re\u{f03a}po")));
        assert_eq!(real(&newline), real(&inner.join("nl\u{f00a}")));
        std::fs::remove_dir_all(&root).expect("cleanup");
    }
}
