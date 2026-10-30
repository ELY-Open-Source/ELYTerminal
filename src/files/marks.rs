use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

use ely_gpui_component::lists::GitStatus;

use crate::git;

/// `path` as marks key it: on macOS, composed, as Git.
pub(super) fn mark_key(path: &Path) -> Cow<'_, Path> {
    #[cfg(target_os = "macos")]
    if let Some(text) = path.to_str()
        && !unicode_normalization::is_nfc(text)
    {
        use unicode_normalization::UnicodeNormalization;
        return Cow::Owned(text.nfc().collect::<String>().into());
    }
    Cow::Borrowed(path)
}

/// Changes by key, and the folders under `root` holding one.
pub(super) fn marks(
    root: &Path,
    changes: Vec<(PathBuf, GitStatus)>,
) -> (HashMap<PathBuf, GitStatus>, HashSet<PathBuf>) {
    let root = mark_key(root);
    let marks: HashMap<PathBuf, GitStatus> = changes
        .into_iter()
        .map(|(path, status)| (mark_key(&path).into_owned(), status))
        .collect();
    let dirty = marks
        .keys()
        .flat_map(|path| {
            let up = path.ancestors().skip(1);
            up.take_while(|folder| folder.starts_with(&root))
        })
        .map(Path::to_path_buf)
        .collect();
    (marks, dirty)
}

/// New changes by path, their marks and dotted folders.
pub(super) type Marked = (
    Arc<Vec<(PathBuf, GitStatus)>>,
    HashMap<PathBuf, GitStatus>,
    HashSet<PathBuf>,
);

/// Git's changes under `root` unless `last`, or Git's failure.
pub(super) fn remark(root: &Path, last: &[(PathBuf, GitStatus)]) -> Result<Option<Marked>, String> {
    let changes = git::changes(root).map_err(|error| format!("{error:#}"))?;
    // No repository, as git says, an unreadable `.git` among them.
    let mut changes = changes.unwrap_or_default();
    changes.sort_by(|a, b| a.0.cmp(&b.0));
    if changes == last {
        return Ok(None);
    }
    let (marks, dirty) = marks(root, changes.clone());
    Ok(Some((Arc::new(changes), marks, dirty)))
}
