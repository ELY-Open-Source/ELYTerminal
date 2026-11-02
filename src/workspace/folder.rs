use std::path::PathBuf;

use anyhow::Context as _;
use ely_gpui_component::i18n::text;
use gpui::{App, Context, PathPromptOptions, PromptLevel, Window};

use super::Workspace;

impl Workspace {
    pub(super) fn choose_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.choosing_folder {
            return;
        }
        self.choosing_folder = true;
        log::info!("app: choosing a folder for a new session");
        let chosen = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(text(cx, "et.files.choose_folder", &[])),
        });
        cx.spawn_in(window, async move |workspace, cx| {
            let result = match chosen.await {
                Ok(Ok(paths)) => {
                    cx.background_executor()
                        .spawn(async move { selected(paths) })
                        .await
                }
                Ok(Err(error)) => Err(error.context("app: cannot choose a folder")),
                Err(error) => {
                    Err(anyhow::Error::new(error).context("app: folder chooser disconnected"))
                }
            };
            if let Err(error) = workspace.update_in(cx, |this, window, cx| {
                this.choosing_folder = false;
                cx.notify();
                match result {
                    Ok(Some(folder)) => {
                        if let Err(error) = this.here(folder, window, cx) {
                            failed(error, window, cx);
                        }
                    }
                    Ok(None) => log::info!("app: folder choice cancelled"),
                    Err(error) => failed(error, window, cx),
                }
            }) {
                log::debug!("app: folder choice owner closed: {error:#}");
            }
        })
        .detach();
    }
}

fn selected(paths: Option<Vec<PathBuf>>) -> anyhow::Result<Option<PathBuf>> {
    let Some(mut paths) = paths else {
        return Ok(None);
    };
    anyhow::ensure!(paths.len() == 1, "app: choose exactly one folder");
    let path = paths.remove(0);
    let meta = path
        .metadata()
        .with_context(|| format!("app: cannot read {}", path.display()))?;
    anyhow::ensure!(meta.is_dir(), "app: {} is not a folder", path.display());
    Ok(Some(path))
}

pub(super) fn failed(error: anyhow::Error, window: &mut Window, cx: &mut App) {
    log::error!("app: cannot open a session in the chosen folder: {error:#}");
    let title = text(cx, "et.files.folder_failed", &[]);
    let close = text(cx, "et.files.dismiss", &[]);
    let answer = window.prompt(
        PromptLevel::Critical,
        &title,
        Some(&format!("{error:#}")),
        &[close.as_ref()],
        cx,
    );
    cx.spawn(async move |_| {
        let _ = answer.await;
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_and_invalid_selections_never_name_a_folder() {
        assert_eq!(selected(None).unwrap(), None);
        assert!(selected(Some(vec![])).is_err());
        let dir = std::env::temp_dir();
        assert!(selected(Some(vec![dir.clone(), dir])).is_err());
        assert!(selected(Some(vec![std::env::current_exe().unwrap()])).is_err());
    }

    #[test]
    fn a_folder_keeps_its_path_and_a_missing_one_fails() {
        let dir = std::env::temp_dir();
        assert_eq!(selected(Some(vec![dir.clone()])).unwrap(), Some(dir));
        let missing = std::env::current_exe().unwrap().join("missing");
        assert!(selected(Some(vec![missing])).is_err());
    }
}
