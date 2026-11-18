//! On Linux an AppImage in its file's place, or a tarball's binary in this copy's place.

use std::{
    ffi::OsStr,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, bail, ensure};

use super::install::run;

/// How this copy was installed.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    /// An AppImage, at the path its runtime names.
    AppImage(PathBuf),
    /// The tarball's binary, where the user put it.
    Tarball(PathBuf),
    /// A DEB or RPM, in `/usr/bin`.
    Package(PathBuf),
}

/// What a restart opens: the AppImage's file, else this binary as it started, before an update replaced its file.
pub fn restart_path() -> anyhow::Result<PathBuf> {
    Ok(match kind()? {
        Kind::AppImage(file) | Kind::Tarball(file) | Kind::Package(file) => file,
    })
}

/// How this copy was installed: an AppImage runs from the folder its runtime names, a package's binary lies in `/usr/bin`.
pub fn kind() -> anyhow::Result<Kind> {
    let exe = std::env::current_exe().context("updates: this copy's path does not read")?;
    // A program started in an AppImage's session inherits its variables, so the binary must run from its mount.
    let mount = std::env::var_os("APPDIR").filter(|mount| exe.starts_with(mount));
    if let (Some(_), Some(image)) = (mount, std::env::var_os("APPIMAGE")) {
        return Ok(Kind::AppImage(PathBuf::from(image)));
    }
    Ok(match exe.parent() == Some(Path::new("/usr/bin")) {
        true => Kind::Package(exe),
        false => Kind::Tarball(exe),
    })
}

/// The name of `version`'s file for `kind`.
pub fn wanted(kind: &Kind, version: &str) -> anyhow::Result<String> {
    let arch = std::env::consts::ARCH;
    Ok(match kind {
        Kind::AppImage(_) => format!("ELYTerminal-{version}-linux-{arch}.AppImage"),
        Kind::Tarball(_) => format!("ELYTerminal-{version}-linux-{arch}.tar.gz"),
        Kind::Package(exe) => bail!("updates: {} came from a package", exe.display()),
    })
}

/// The file this copy's update replaces, refused where its folder cannot be written.
pub fn target() -> anyhow::Result<PathBuf> {
    let file = match kind()? {
        Kind::AppImage(file) | Kind::Tarball(file) => file,
        Kind::Package(exe) => bail!("updates: {} came from a package", exe.display()),
    };
    let folder = file
        .parent()
        .context("updates: this copy lies in no folder")?;
    let path = std::ffi::CString::new(folder.as_os_str().as_encoded_bytes())?;
    // SAFETY: `path` is a NUL-ended path.
    let writable = unsafe { libc::access(path.as_ptr(), libc::W_OK) } == 0;
    ensure!(writable, "updates: {} is not writable", folder.display());
    Ok(file)
}

/// Puts the release in `file`, an AppImage or a tarball, at `target`.
pub fn place(file: &Path, target: &Path) -> anyhow::Result<()> {
    let name = file.file_name().and_then(OsStr::to_str).unwrap_or_default();
    let staged = sibling(target, &format!(".elyterminal-new-{}", std::process::id()));
    if name.ends_with(".AppImage") {
        if let Err(error) = std::fs::copy(file, &staged) {
            // A copy cut short leaves part of a file.
            if staged.exists() {
                std::fs::remove_file(&staged)
                    .with_context(|| format!("updates: cannot remove {}", staged.display()))?;
            }
            return Err(error).with_context(|| {
                format!("updates: cannot copy {name} beside {}", target.display())
            });
        }
    } else if let Some(folder) = name.strip_suffix(".tar.gz") {
        let unpacked = sibling(
            target,
            &format!(".elyterminal-update-{}", std::process::id()),
        );
        std::fs::create_dir(&unpacked)
            .with_context(|| format!("updates: cannot create {}", unpacked.display()))?;
        let untar = [
            "-xzf".as_ref(),
            file.as_os_str(),
            "-C".as_ref(),
            unpacked.as_os_str(),
        ];
        let unpacked_binary = run("tar", &untar).map(|_| unpacked.join(folder).join("elyterminal"));
        let moved = unpacked_binary.and_then(|binary| {
            std::fs::rename(&binary, &staged)
                .with_context(|| format!("updates: {name} holds no {}", binary.display()))
        });
        std::fs::remove_dir_all(&unpacked)
            .with_context(|| format!("updates: cannot remove {}", unpacked.display()))?;
        moved?;
    } else {
        bail!("updates: {name} is neither an AppImage nor a tarball");
    }
    // A running binary's file is replaced, not written over, so the running copy goes on.
    let put = std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))
        .with_context(|| format!("updates: cannot make {} run", staged.display()))
        .and_then(|()| {
            std::fs::rename(&staged, target)
                .with_context(|| format!("updates: cannot put the release at {}", target.display()))
        });
    if put.is_err() && staged.exists() {
        std::fs::remove_file(&staged)
            .with_context(|| format!("updates: cannot remove {}", staged.display()))?;
    }
    put?;
    log::info!("updates: {} holds the release", target.display());
    Ok(())
}

fn sibling(target: &Path, suffix: &str) -> PathBuf {
    let mut name = target.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_install_takes_its_own_file() {
        let arch = std::env::consts::ARCH;
        let image = Kind::AppImage("/x".into());
        assert_eq!(
            wanted(&image, "1.2.3").expect("one"),
            format!("ELYTerminal-1.2.3-linux-{arch}.AppImage")
        );
        let tarball = Kind::Tarball("/x".into());
        assert_eq!(
            wanted(&tarball, "1.2.3").expect("one"),
            format!("ELYTerminal-1.2.3-linux-{arch}.tar.gz")
        );
    }
}
