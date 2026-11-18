//! A DEB or RPM copy, its update installed by the system's own tool, as root through pkexec.

use std::{ffi::OsStr, path::Path, process::Command};

use anyhow::{Context as _, bail};

use crate::updates::install::run;

/// The package manager that owns a copy.
pub enum Manager {
    Deb,
    Rpm,
}

/// The manager that owns `binary`, refused where none does.
pub fn manager(binary: &Path) -> anyhow::Result<Manager> {
    let owns = |program: &str, flag: &str| {
        Command::new(program)
            .args([flag.as_ref(), binary.as_os_str()])
            .output()
            .is_ok_and(|output| output.status.success())
    };
    if owns("dpkg-query", "-S") {
        return Ok(Manager::Deb);
    }
    if owns("rpm", "-qf") {
        return Ok(Manager::Rpm);
    }
    bail!("updates: neither dpkg nor rpm owns {}", binary.display())
}

/// Installs `file` over the package that owns `binary`: apt-get for a DEB, dnf for an RPM, through pkexec unless root.
pub fn install(file: &Path, binary: &Path) -> anyhow::Result<()> {
    let tool = match manager(binary)? {
        Manager::Deb => "apt-get",
        Manager::Rpm => "dnf",
    };
    let found = |program: &str| {
        ["/usr/bin", "/bin"]
            .iter()
            .any(|folder| Path::new(folder).join(program).exists())
    };
    if !found(tool) {
        bail!(
            "updates: this system has no {tool} to install {}",
            file.display()
        );
    }
    let install: [&OsStr; 3] = ["install".as_ref(), "-y".as_ref(), file.as_os_str()];
    // SAFETY: geteuid reads the process's own user.
    let (program, args) = match unsafe { libc::geteuid() } == 0 {
        true => (tool, install.to_vec()),
        false if found("pkexec") => (
            "pkexec",
            [tool.as_ref()].into_iter().chain(install).collect(),
        ),
        false => bail!("updates: this system has no pkexec to run {tool} as root"),
    };
    log::info!("updates: {program} installs {}", file.display());
    run(program, &args)
        .with_context(|| format!("updates: {tool} did not install {}", file.display()))?;
    log::info!("updates: {tool} installed {}", file.display());
    Ok(())
}
