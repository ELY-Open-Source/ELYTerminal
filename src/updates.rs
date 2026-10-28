//! Checks for a newer release through curl and GitHub's API: at start and once a day while the settings ask, and whenever Settings asks at once.

use std::{
    collections::HashMap,
    process::{Command, Stdio},
    time::Duration,
};

use anyhow::Context as _;
use ely_gpui_component::i18n::text;
use gpui::{App, Global, SystemNotification};
use serde::Deserialize;

use crate::{settings::SettingsStore, workspace::TITLE};

/// The latest published release.
const LATEST: &str = "https://api.github.com/repos/ELY-Open-Source/ELYTerminal/releases/latest";
/// How long an automatic check waits for the next.
const DAY: Duration = Duration::from_secs(24 * 60 * 60);
/// A newer release's notification tag, before its version.
const TAG: &str = "elyterminal-update-";
/// The running version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// What the last check found.
#[derive(Clone, Debug, PartialEq)]
pub enum Updates {
    Unchecked,
    Checking,
    Latest,
    Newer { version: String, page: String },
    Failed(String),
}

impl Global for Updates {}

/// Each newer release's page, by its notification's tag.
#[derive(Default)]
struct Pages(HashMap<String, String>);

impl Global for Pages {}

/// The fields of a release the check reads.
#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
}

/// Checks at once and then once a day, each time the settings ask for it.
pub fn start(cx: &mut App) {
    cx.set_global(Updates::Unchecked);
    cx.spawn(async move |cx| {
        loop {
            cx.update(|cx| {
                if SettingsStore::get(cx).check_updates {
                    check(true, cx);
                }
            });
            cx.background_executor().timer(DAY).await;
        }
    })
    .detach();
}

/// Asks for the latest release off the main thread; when `auto`, a newer one posts a notification.
pub fn check(auto: bool, cx: &mut App) {
    if *cx.global::<Updates>() == Updates::Checking {
        return;
    }
    log::info!("updates: checking for a release newer than {VERSION}");
    cx.set_global(Updates::Checking);
    let fetched = cx.background_executor().spawn(async { latest() });
    cx.spawn(async move |cx| {
        let found = fetched.await.and_then(newer);
        cx.update(|cx| finish(found, auto, cx));
    })
    .detach();
}

/// Keeps what the check found, and tells of a newer release it found by itself.
fn finish(found: anyhow::Result<Option<(String, String)>>, auto: bool, cx: &mut App) {
    let found = match found {
        Ok(Some((version, page))) => {
            log::info!("updates: {version} is out, newer than {VERSION}, at {page}");
            let tag = format!("{TAG}{version}");
            cx.default_global::<Pages>()
                .0
                .insert(tag.clone(), page.clone());
            if auto {
                cx.show_system_notification(SystemNotification {
                    tag: tag.into(),
                    title: TITLE.into(),
                    body: text(cx, "et.updates.notice", &[("version", &version)]),
                    actions: Vec::new(),
                });
            }
            Updates::Newer { version, page }
        }
        Ok(None) => {
            log::info!("updates: {VERSION} is the latest");
            Updates::Latest
        }
        Err(error) => {
            log::error!("{error:#}");
            Updates::Failed(format!("{error:#}"))
        }
    };
    cx.set_global(found);
}

/// Opens the page an update `tag` names; else false.
pub fn pressed(tag: &str, cx: &mut App) -> bool {
    let Some(version) = tag.strip_prefix(TAG) else {
        return false;
    };
    match cx
        .try_global::<Pages>()
        .and_then(|pages| pages.0.get(tag).cloned())
    {
        Some(page) => {
            log::info!("updates: {version}'s notification opens {page}");
            cx.open_url(&page);
        }
        None => log::info!(
            "updates: {version}'s notification is an earlier run's; no check of this run found {version}"
        ),
    }
    true
}

/// The latest release, which curl fetches from GitHub's API.
fn latest() -> anyhow::Result<Release> {
    let mut command = Command::new("curl");
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(
        &mut command,
        crate::git::CREATE_NO_WINDOW,
    );
    let output = command
        .args(["--fail", "--silent", "--show-error", "--location"])
        .args(["--max-time", "30", LATEST])
        .stdin(Stdio::null())
        .output()
        .context("updates: curl did not start")?;
    anyhow::ensure!(
        output.status.success(),
        "updates: curl failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    serde_json::from_slice(&output.stdout).context("updates: the latest release does not read")
}

/// The release's version and page when it is newer than this one.
fn newer(release: Release) -> anyhow::Result<Option<(String, String)>> {
    let latest = version(&release.tag_name)?;
    let shown = release.tag_name.trim_start_matches('v').to_string();
    Ok((latest > version(VERSION)?).then_some((shown, release.html_url)))
}

/// `tag`'s three numbers, from `v0.1.6` or `0.1.6`.
fn version(tag: &str) -> anyhow::Result<[u64; 3]> {
    let numbers: Vec<u64> = (tag.trim_start_matches('v').split('.'))
        .map(str::parse)
        .collect::<Result<_, _>>()
        .with_context(|| format!("updates: {tag:?} is no version"))?;
    numbers
        .try_into()
        .map_err(|_| anyhow::anyhow!("updates: {tag:?} is no version of three numbers"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_reads_as_three_numbers() {
        assert_eq!(version("v0.1.6").expect("a tag"), [0, 1, 6]);
        assert_eq!(version("1.10.0").expect("no v"), [1, 10, 0]);
        assert!(version("v0.1").is_err(), "two numbers");
        assert!(version("v0.1.6-beta").is_err(), "a suffix");
    }

    #[test]
    fn only_a_later_release_is_newer() {
        let release = |tag: &str| Release {
            tag_name: tag.into(),
            html_url: format!("https://example.org/{tag}"),
        };
        let [major, minor, patch] = version(VERSION).expect("this version");
        let later = format!("v{major}.{minor}.{}", patch + 1);
        let found = newer(release(&later)).expect("reads");
        let shown = later.trim_start_matches('v').to_string();
        assert_eq!(found, Some((shown, format!("https://example.org/{later}"))));
        assert_eq!(
            newer(release(&format!("v{VERSION}"))).expect("reads"),
            None,
            "this one"
        );
        assert_eq!(
            newer(release("v0.0.1")).expect("reads"),
            None,
            "an older one"
        );
        assert_eq!(
            newer(release(&format!("v{}.0.0", major + 1)))
                .expect("reads")
                .map(|(v, _)| v),
            Some(format!("{}.0.0", major + 1)),
            "by its numbers, not as text"
        );
    }

    #[test]
    fn github_s_release_reads() {
        let body = r#"{"id":213,"tag_name":"v0.1.5","name":"ELYTerminal 0.1.5","html_url":"https://github.com/ELY-Open-Source/ELYTerminal/releases/tag/v0.1.5","draft":false,"assets":[]}"#;
        let release: Release = serde_json::from_str(body).expect("a release");
        assert_eq!(release.tag_name, "v0.1.5");
        assert!(release.html_url.ends_with("/releases/tag/v0.1.5"));
    }
}
