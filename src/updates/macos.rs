//! On macOS a release's disk image: its app checked as the owner's signed one, then swapped in for this bundle.

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, ensure};

use super::install::run;

/// The team that signs every release.
const TEAM: &str = "9PDD8M2SMS";

/// The app bundle this copy runs from, refused where it cannot be replaced.
pub fn bundle() -> anyhow::Result<PathBuf> {
    let exe = std::env::current_exe().context("updates: this copy's path does not read")?;
    let bundle = (exe.ancestors().nth(3))
        .filter(|bundle| bundle.extension() == Some(OsStr::new("app")))
        .with_context(|| format!("updates: {} runs outside an app bundle", exe.display()))?;
    ensure!(
        !bundle.to_string_lossy().contains("/AppTranslocation/"),
        "updates: macOS runs this copy translocated; move ELYTerminal to Applications first"
    );
    let folder = bundle
        .parent()
        .context("updates: the bundle lies in no folder")?;
    let path = std::ffi::CString::new(folder.as_os_str().as_encoded_bytes())?;
    // SAFETY: `path` is a NUL-ended path.
    let writable = unsafe { libc::access(path.as_ptr(), libc::W_OK) } == 0;
    ensure!(writable, "updates: {} is not writable", folder.display());
    Ok(bundle.to_path_buf())
}

/// Mounts `image`, checks its app, and puts it in `bundle`'s place.
pub fn place(image: &Path, bundle: &Path) -> anyhow::Result<()> {
    let mount = image.with_extension("mount");
    std::fs::create_dir_all(&mount)
        .with_context(|| format!("updates: cannot create {}", mount.display()))?;
    let attach = [
        "attach".as_ref(),
        "-nobrowse".as_ref(),
        "-readonly".as_ref(),
        "-noautoopen".as_ref(),
        "-mountpoint".as_ref(),
        mount.as_os_str(),
        image.as_os_str(),
    ];
    run("hdiutil", &attach)?;
    let placed = swap(&mount.join("ELYTerminal.app"), bundle);
    let detached = run("hdiutil", &["detach".as_ref(), mount.as_os_str()]);
    placed?;
    detached?;
    std::fs::remove_dir(&mount)
        .with_context(|| format!("updates: cannot remove {}", mount.display()))
}

/// Checks `app`'s signature and team, copies it beside `bundle`, and swaps the two.
fn swap(app: &Path, bundle: &Path) -> anyhow::Result<()> {
    run(
        "codesign",
        &[
            "--verify".as_ref(),
            "--deep".as_ref(),
            "--strict".as_ref(),
            app.as_os_str(),
        ],
    )?;
    let said = run(
        "codesign",
        &["-dv".as_ref(), "--verbose=2".as_ref(), app.as_os_str()],
    )?;
    ensure!(
        said.lines()
            .any(|line| line == format!("TeamIdentifier={TEAM}")),
        "updates: the release's app is not signed by team {TEAM}"
    );
    log::info!("updates: the release's app is signed by team {TEAM}");
    let staged = bundle.with_extension("app.new");
    let old = bundle.with_extension("app.old");
    for left in [&staged, &old] {
        if left.exists() {
            std::fs::remove_dir_all(left)
                .with_context(|| format!("updates: cannot remove {}", left.display()))?;
        }
    }
    run("ditto", &[app.as_os_str(), staged.as_os_str()])?;
    std::fs::rename(bundle, &old)
        .with_context(|| format!("updates: cannot move {} aside", bundle.display()))?;
    if let Err(error) = std::fs::rename(&staged, bundle) {
        std::fs::rename(&old, bundle)
            .with_context(|| format!("updates: {} could not come back", bundle.display()))?;
        return Err(error)
            .with_context(|| format!("updates: cannot put the release at {}", bundle.display()));
    }
    std::fs::remove_dir_all(&old)
        .with_context(|| format!("updates: cannot remove {}", old.display()))?;
    log::info!("updates: {} holds the release", bundle.display());
    Ok(())
}
