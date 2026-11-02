//! What the new-session menu offers: the user's profiles, then the shells this system holds, as Windows Terminal's dynamic profiles.

use std::path::PathBuf;

use ely_gpui_component::{
    i18n::text,
    menus::{Menu, MenuItem, OverflowMenu},
    primitives::IconName,
};
use gpui::{App, Entity, SharedString};

use crate::{
    settings::{self, SettingsStore},
    workspace::Workspace,
};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::detected;

/// A shell this system holds: its name, its program and arguments.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub name: String,
    pub words: Vec<String>,
}

/// The shells `/etc/shells` lists that run, each once.
#[cfg(unix)]
pub fn detected() -> Vec<Found> {
    let text = match std::fs::read_to_string("/etc/shells") {
        Ok(text) => text,
        Err(error) => {
            log::error!("profiles: cannot read /etc/shells: {error}");
            return Vec::new();
        }
    };
    let found = listed(&text);
    log::info!("profiles: {} shells found in /etc/shells", found.len());
    found
}

/// Each shell `text` lists that the user may run, by its path, a link to one listed before passed by.
#[cfg(unix)]
fn listed(text: &str) -> Vec<Found> {
    let mut seen = Vec::new();
    let mut found = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') || settings::program_path(line).is_err() {
            continue;
        }
        let real = match std::fs::canonicalize(line) {
            Ok(real) => real,
            Err(error) => {
                log::warn!("profiles: {line} does not resolve: {error}");
                continue;
            }
        };
        if !seen.contains(&real) {
            seen.push(real);
            let words = vec![line.to_string()];
            found.push(Found {
                name: line.into(),
                words,
            });
        }
    }
    found
}

/// The chevron beside +, its menu of profiles; none while there is none.
pub fn button(workspace: &Entity<Workspace>, found: &[Found], cx: &App) -> Option<OverflowMenu> {
    let profiles = &SettingsStore::get(cx).profiles;
    if profiles.is_empty() && found.is_empty() {
        return None;
    }
    let mut menu = Menu::new();
    for profile in profiles {
        let words = settings::words(&profile.command).expect("the settings checked it");
        let folder = (!profile.folder.is_empty()).then(|| PathBuf::from(&profile.folder));
        menu = menu.item(item(workspace, &profile.name, words, folder));
    }
    if !profiles.is_empty() && !found.is_empty() {
        menu = menu.separator();
    }
    for shell in found {
        menu = menu.item(item(workspace, &shell.name, shell.words.clone(), None));
    }
    let tooltip = text(cx, "et.session.profiles", &[]);
    Some(
        OverflowMenu::new("profiles", menu)
            .icon(IconName::ChevronDown)
            .tooltip(tooltip),
    )
}

/// An entry that opens `words` in `folder`, else where Start in says.
fn item(
    workspace: &Entity<Workspace>,
    name: &str,
    words: Vec<String>,
    folder: Option<PathBuf>,
) -> MenuItem {
    let workspace = workspace.clone();
    MenuItem::new(SharedString::from(name.to_string())).on_click(move |window, cx| {
        let (shell, args) = words.split_first().expect("a command names a program");
        let command = (shell.clone(), args.to_vec());
        workspace.update(cx, |this, cx| {
            this.profile(command, folder.clone(), window, cx)
        });
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn etc_shells_lists_each_running_shell_once() {
        let text = "# List of shells\n\n/bin/sh\n  /bin/sh  \n/no/such/shell\n/bin/zsh\n";
        let names: Vec<_> = listed(text).into_iter().map(|found| found.name).collect();
        let zsh = std::path::Path::new("/bin/zsh").exists();
        let expected: &[&str] = if zsh {
            &["/bin/sh", "/bin/zsh"]
        } else {
            &["/bin/sh"]
        };
        assert_eq!(names, expected);
    }
}
