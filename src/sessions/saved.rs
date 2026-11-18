//! The sessions a window holds, kept in `sessions.json` beside the settings, so the next start opens them again.

use std::path::{Path, PathBuf};

use anyhow::Context as _;
use serde::{Deserialize, Serialize};

/// What the next start opens: the sessions in their rows' order, their groups, and the one shown.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Saved {
    pub sessions: Vec<SavedSession>,
    pub groups: Vec<SavedGroup>,
    pub shown: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedSession {
    pub name: String,
    pub group: Option<u64>,
    /// The focused pane, by its place among the layout's panes.
    pub focused: usize,
    pub layout: SavedLayout,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedGroup {
    pub id: u64,
    pub name: String,
}

/// A pane's program and folder, or a split: `down` stacks its parts, the first taking `share`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SavedLayout {
    Pane {
        shell: String,
        args: Vec<String>,
        #[serde(with = "folder")]
        folder: PathBuf,
    },
    Split {
        down: bool,
        share: f32,
        first: Box<SavedLayout>,
        second: Box<SavedLayout>,
    },
}

impl SavedLayout {
    /// Its panes, left to right and top to bottom: each program, its arguments and its folder.
    pub fn panes(&self) -> Vec<(&str, &[String], &Path)> {
        match self {
            SavedLayout::Pane {
                shell,
                args,
                folder,
            } => vec![(shell, args, folder)],
            SavedLayout::Split { first, second, .. } => {
                let mut panes = first.panes();
                panes.extend(second.panes());
                panes
            }
        }
    }

    /// Without the panes whose folder `is_folder` denies, logged; a split left one part is that part.
    fn kept(self, is_folder: &impl Fn(&Path) -> bool) -> Option<Self> {
        match self {
            SavedLayout::Pane { ref folder, .. } if !is_folder(folder) => {
                log::warn!("sessions: a pane's folder {} is gone", folder.display());
                None
            }
            SavedLayout::Pane { .. } => Some(self),
            SavedLayout::Split {
                down,
                share,
                first,
                second,
            } => match (first.kept(is_folder), second.kept(is_folder)) {
                (Some(first), Some(second)) => Some(SavedLayout::Split {
                    down,
                    share: share.clamp(0.0, 1.0),
                    first: Box::new(first),
                    second: Box::new(second),
                }),
                (one, other) => one.or(other),
            },
        }
    }
}

impl Saved {
    /// What still opens: panes whose folder is there, sessions with a pane, groups a session holds; none when nothing does.
    pub fn kept(self, is_folder: impl Fn(&Path) -> bool) -> Option<Self> {
        let shown = self.shown;
        let mut place = None;
        let mut sessions = Vec::new();
        for (index, session) in self.sessions.into_iter().enumerate() {
            let alive: Vec<bool> = (session.layout.panes().iter())
                .map(|(_, _, folder)| is_folder(folder))
                .collect();
            // The focused pane, at its place among the panes left; the first when it went.
            let focused = match alive.get(session.focused) {
                Some(true) => alive[..session.focused]
                    .iter()
                    .filter(|kept| **kept)
                    .count(),
                _ => 0,
            };
            let Some(layout) = session.layout.kept(&is_folder) else {
                log::warn!("sessions: session {} has no folder left", index + 1);
                continue;
            };
            if index == shown {
                place = Some(sessions.len());
            }
            sessions.push(SavedSession {
                focused,
                layout,
                ..session
            });
        }
        if sessions.is_empty() {
            return None;
        }
        let groups = (self.groups.into_iter())
            .filter(|group| {
                sessions
                    .iter()
                    .any(|session| session.group == Some(group.id))
            })
            .collect::<Vec<_>>();
        for session in &mut sessions {
            if session
                .group
                .is_some_and(|id| groups.iter().all(|group| group.id != id))
            {
                log::warn!(
                    "sessions: a session's group {:?} is listed nowhere",
                    session.group
                );
                session.group = None;
            }
        }
        Some(Self {
            sessions,
            groups,
            shown: place.unwrap_or(0),
        })
    }
}

/// `sessions.json`, beside the settings.
pub fn path() -> anyhow::Result<PathBuf> {
    let folder = dirs::config_dir().context("sessions: the platform names no config folder")?;
    Ok(folder.join("ELYTerminal").join("sessions.json"))
}

/// The sessions `path` keeps; none while it is missing.
pub fn load(path: &Path) -> anyhow::Result<Option<Saved>> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| format!("sessions: cannot read {}", path.display()));
        }
    };
    let saved = serde_json::from_str(&text)
        .with_context(|| format!("sessions: {} does not read", path.display()))?;
    log::info!("sessions: loaded {}", path.display());
    Ok(Some(saved))
}

/// A folder as text, or on Unix its bytes where they are no UTF-8.
mod folder {
    use std::path::{Path, PathBuf};

    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    #[cfg(not(unix))]
    use serde::{de::Error as _, ser::Error as _};

    #[derive(Serialize, Deserialize)]
    #[serde(untagged)]
    enum Folder {
        Text(String),
        Bytes(Vec<u8>),
    }

    pub fn serialize<S: Serializer>(path: &Path, to: S) -> Result<S::Ok, S::Error> {
        if let Some(text) = path.to_str() {
            return Folder::Text(text.to_string()).serialize(to);
        }
        #[cfg(unix)]
        return Folder::Bytes(std::os::unix::ffi::OsStrExt::as_bytes(path.as_os_str()).to_vec())
            .serialize(to);
        #[cfg(not(unix))]
        Err(S::Error::custom(format!("{} is no text", path.display())))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(from: D) -> Result<PathBuf, D::Error> {
        match Folder::deserialize(from)? {
            Folder::Text(text) => Ok(PathBuf::from(text)),
            #[cfg(unix)]
            Folder::Bytes(bytes) => Ok(PathBuf::from(
                <std::ffi::OsString as std::os::unix::ffi::OsStringExt>::from_vec(bytes),
            )),
            #[cfg(not(unix))]
            Folder::Bytes(_) => Err(D::Error::custom("a folder's bytes name no folder here")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pane(folder: &str) -> SavedLayout {
        SavedLayout::Pane {
            shell: String::new(),
            args: Vec::new(),
            folder: PathBuf::from(folder),
        }
    }

    fn split(first: SavedLayout, second: SavedLayout) -> SavedLayout {
        SavedLayout::Split {
            down: false,
            share: 0.3,
            first: Box::new(first),
            second: Box::new(second),
        }
    }

    fn session(group: Option<u64>, focused: usize, layout: SavedLayout) -> SavedSession {
        SavedSession {
            name: String::new(),
            group,
            focused,
            layout,
        }
    }

    #[test]
    fn a_gone_folder_drops_its_pane_its_empty_session_and_their_group() {
        let saved = Saved {
            sessions: vec![
                session(Some(1), 0, pane("/gone")),
                session(None, 1, split(pane("/a"), split(pane("/gone"), pane("/b")))),
                session(Some(2), 0, pane("/c")),
            ],
            groups: vec![
                SavedGroup {
                    id: 1,
                    name: "one".into(),
                },
                SavedGroup {
                    id: 2,
                    name: "two".into(),
                },
            ],
            shown: 2,
        };
        let kept = (saved.clone())
            .kept(|folder| folder != Path::new("/gone"))
            .expect("two remain");
        assert_eq!(kept.sessions.len(), 2);
        assert_eq!(kept.sessions[0].layout, split(pane("/a"), pane("/b")));
        assert_eq!(
            kept.sessions[0].focused, 0,
            "its focused pane went: the first"
        );
        assert_eq!(
            kept.groups,
            [SavedGroup {
                id: 2,
                name: "two".into()
            }]
        );
        assert_eq!(kept.shown, 1, "the shown one, at its new place");
        let mut first = saved;
        first.sessions[1].focused = 2;
        let kept = first
            .kept(|folder| folder != Path::new("/a"))
            .expect("all three");
        assert_eq!(
            kept.sessions[1].focused, 1,
            "the focused pane, past one gone"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_folder_not_in_utf8_keeps_its_bytes() {
        use std::os::unix::ffi::OsStringExt;
        let folder = PathBuf::from(std::ffi::OsString::from_vec(b"/tmp/a\xffb".to_vec()));
        let layout = SavedLayout::Pane {
            shell: String::new(),
            args: Vec::new(),
            folder,
        };
        let text = serde_json::to_string(&layout).expect("encodes");
        assert_eq!(
            serde_json::from_str::<SavedLayout>(&text).expect("decodes"),
            layout
        );
        assert_eq!(
            serde_json::from_str::<SavedLayout>(&serde_json::to_string(&pane("/a")).expect("text"))
                .expect("text"),
            pane("/a")
        );
    }

    #[test]
    fn nothing_kept_opens_nothing_and_the_file_reads_back() {
        let saved = Saved {
            sessions: vec![session(None, 4, pane("/gone"))],
            groups: Vec::new(),
            shown: 0,
        };
        assert_eq!(saved.clone().kept(|_| false), None);
        let kept = saved.kept(|_| true).expect("one");
        assert_eq!(kept.sessions[0].focused, 0, "a focus past its panes");
        let text = serde_json::to_string(&kept).expect("encodes");
        assert_eq!(serde_json::from_str::<Saved>(&text).expect("decodes"), kept);
        assert!(
            serde_json::from_str::<Saved>("{\"sessions\":[]}").is_err(),
            "a field missing"
        );
    }
}
