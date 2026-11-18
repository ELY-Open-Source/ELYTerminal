//! A newer release downloaded, checked against the SHA-256 GitHub lists, and put in this copy's place.

use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context as _, bail, ensure};
use gpui::App;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::{Updates, VERSION};

/// A file of a release, as GitHub lists it.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    /// `sha256:` and the file's digest in hex.
    pub digest: Option<String>,
}

/// The name of `version`'s file this system's copy takes.
pub fn wanted(version: &str) -> anyhow::Result<String> {
    #[cfg(target_os = "macos")]
    return Ok(format!("ELYTerminal-{version}.dmg"));
    #[cfg(windows)]
    return Ok(format!(
        "ELYTerminal-{version}-windows-{}.zip",
        std::env::consts::ARCH
    ));
    #[cfg(target_os = "linux")]
    bail!("updates: this system's copy takes no file of {version} yet")
}

/// Downloads `version`'s file among `assets` into `folder` and checks its digest.
pub fn fetch(assets: &[Asset], version: &str, folder: &Path) -> anyhow::Result<PathBuf> {
    let name = wanted(version)?;
    let asset = (assets.iter())
        .find(|asset| asset.name == name)
        .with_context(|| format!("updates: {version} has no {name}"))?;
    let digest = (asset.digest.as_deref())
        .and_then(|digest| digest.strip_prefix("sha256:"))
        .with_context(|| format!("updates: GitHub lists no SHA-256 for {name}"))?;
    std::fs::create_dir_all(folder)
        .with_context(|| format!("updates: cannot create {}", folder.display()))?;
    let file = folder.join(&name);
    log::info!(
        "updates: downloading {} to {}",
        asset.browser_download_url,
        file.display()
    );
    let mut command = Command::new("curl");
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(
        &mut command,
        crate::git::CREATE_NO_WINDOW,
    );
    let output = command
        .args(["--fail", "--silent", "--show-error", "--location"])
        .args(["--max-time", "900", "--output"])
        .arg(&file)
        .arg(&asset.browser_download_url)
        .stdin(Stdio::null())
        .output()
        .context("updates: curl did not start")?;
    ensure!(
        output.status.success(),
        "updates: curl failed on {name}: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    let found = sha256(&file)?;
    ensure!(
        found.eq_ignore_ascii_case(digest),
        "updates: {name}'s SHA-256 is {found}, where GitHub lists {digest}"
    );
    log::info!("updates: {name}'s SHA-256 matches GitHub's");
    Ok(file)
}

/// `file`'s SHA-256 in hex.
fn sha256(file: &Path) -> anyhow::Result<String> {
    let mut reader = std::fs::File::open(file)
        .with_context(|| format!("updates: cannot open {}", file.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 1 << 16];
    loop {
        let read = (reader.read(&mut buffer))
            .with_context(|| format!("updates: cannot read {}", file.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

/// Puts the release in `file` in this copy's place, then removes `file`.
pub fn place(file: &Path) -> anyhow::Result<()> {
    put(file, &target()?)?;
    std::fs::remove_file(file).with_context(|| format!("updates: cannot remove {}", file.display()))
}

/// This copy's place, refused where it cannot be replaced.
pub fn target() -> anyhow::Result<PathBuf> {
    #[cfg(target_os = "macos")]
    return super::macos::bundle();
    #[cfg(windows)]
    return super::windows::folder();
    #[cfg(target_os = "linux")]
    bail!("updates: this system installs no update yet")
}

/// Puts the release in `file` at `target`.
pub fn put(file: &Path, target: &Path) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    return super::macos::place(file, target);
    #[cfg(windows)]
    return super::windows::place(file, target);
    #[cfg(target_os = "linux")]
    bail!(
        "updates: this system installs no update yet: {} at {}",
        file.display(),
        target.display()
    )
}

/// Downloads and installs the newer release the last check found, off the main thread.
pub fn update(cx: &mut App) {
    let Updates::Newer {
        version, assets, ..
    } = cx.global::<Updates>().clone()
    else {
        return log::error!("updates: Update asked with no newer release found");
    };
    let folder = match dirs::cache_dir() {
        Some(cache) => cache.join("ELYTerminal").join("updates"),
        None => {
            let why = "updates: the platform names no cache folder".to_string();
            log::error!("{why}");
            return cx.set_global(Updates::Failed(why));
        }
    };
    log::info!("updates: installing {version} over {VERSION}");
    cx.set_global(Updates::Fetching(version.clone()));
    let task = cx.background_executor().spawn({
        let version = version.clone();
        async move { fetch(&assets, &version, &folder).and_then(|file| place(&file)) }
    });
    cx.spawn(async move |cx| {
        let done = task.await;
        cx.update(|cx| match done {
            Ok(()) => {
                log::info!("updates: {version} is installed; a restart runs it");
                cx.set_global(Updates::Ready(version));
            }
            Err(error) => {
                log::error!("{error:#}");
                cx.set_global(Updates::Failed(format!("{error:#}")));
            }
        });
    })
    .detach();
}

/// Runs `program` with `args`, its failure naming it with what it wrote.
#[cfg(any(target_os = "macos", windows))]
pub(super) fn run(program: &str, args: &[&std::ffi::OsStr]) -> anyhow::Result<String> {
    let mut command = Command::new(program);
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(
        &mut command,
        crate::git::CREATE_NO_WINDOW,
    );
    let output = command
        .args(args)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("updates: {program} did not start"))?;
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if !output.status.success() {
        bail!("updates: {program} failed: {}", said.trim());
    }
    Ok(said)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_s_digest_is_its_sha_256() {
        let folder = std::env::temp_dir().join(format!("ely-digest-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("a folder");
        let file = folder.join("abc");
        std::fs::write(&file, b"abc").expect("written");
        let digest = sha256(&file).expect("reads");
        std::fs::remove_dir_all(&folder).expect("the folder goes");
        let want = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(digest, want);
    }

    #[test]
    fn a_release_without_this_system_s_file_or_its_digest_is_refused() {
        let Ok(name) = wanted("9.9.9") else {
            return;
        };
        let folder = std::env::temp_dir().join("ely-never-written");
        let error = fetch(&[], "9.9.9", &folder).expect_err("no file");
        assert!(format!("{error:#}").contains(&name), "{error:#}");
        let bare = Asset {
            name,
            browser_download_url: "https://example.invalid/x".into(),
            digest: None,
        };
        let error = fetch(&[bare], "9.9.9", &folder).expect_err("no digest");
        assert!(format!("{error:#}").contains("no SHA-256"), "{error:#}");
    }
}
