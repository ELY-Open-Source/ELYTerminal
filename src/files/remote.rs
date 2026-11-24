//! A folder on another host, listed through an ssh channel of its own.

use std::{
    ffi::OsString,
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio},
    sync::Mutex,
    time::SystemTime,
};

use anyhow::Context as _;

use super::list::{Entry, Listing, sort};

/// The remote shell's lister: each entry `d` or `f` and its name, NUL-ended, then `Z`; `E` when the folder does not read.
const LISTER: &str = r#"l() { cd -- "$1" 2>/dev/null || { printf 'E\0'; return; }; for f in * .*; do case $f in .|..) continue;; esac; [ -e "$f" ] || [ -L "$f" ] || continue; if [ -d "$f" ]; then printf 'd%s\0' "$f"; else printf 'f%s\0' "$f"; fi; done; printf 'Z\0'; }"#;

/// An ssh to a host whose shell lists folders on request.
pub struct Remote {
    pub host: String,
    reach: Vec<String>,
    channel: Mutex<Channel>,
}

struct Channel {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    errors: Option<ChildStderr>,
}

impl Remote {
    /// Starts ssh with `reach`, its options and destination, with no terminal, forward or password.
    pub fn open(host: String, reach: Vec<String>) -> anyhow::Result<Self> {
        let mut command = Command::new("ssh");
        #[cfg(windows)]
        std::os::windows::process::CommandExt::creation_flags(
            &mut command,
            crate::git::CREATE_NO_WINDOW,
        );
        let mut child = command
            .args(["-T", "-o", "BatchMode=yes", "-o", "ClearAllForwardings=yes"])
            .args(["-o", "ControlMaster=no", "-o", "RemoteCommand=none"])
            .args(["-o", "ConnectTimeout=10"])
            .args(&reach)
            .arg("sh")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("files: ssh to {host} does not start"))?;
        let (input, output) = (child.stdin.take(), child.stdout.take());
        let (Some(mut input), Some(output)) = (input, output) else {
            anyhow::bail!("files: ssh to {host} gave no pipes");
        };
        writeln!(input, "{LISTER}").with_context(|| format!("files: ssh to {host}"))?;
        log::info!("files: an ssh lists {host}'s folders through {reach:?}");
        let errors = child.stderr.take();
        let output = BufReader::new(output);
        let channel = Mutex::new(Channel {
            child,
            input,
            output,
            errors,
        });
        Ok(Self {
            host,
            reach,
            channel,
        })
    }

    /// Whether it reaches `host` as `reach` does.
    pub fn reaches(&self, host: &str, reach: &[String]) -> bool {
        self.host == host && self.reach == reach
    }

    /// `folder`'s entries on the host; once ssh ends, what it said.
    pub(super) fn list(&self, folder: &Path) -> Listing {
        let mut channel = self.channel.lock().map_err(|_| "the ssh channel broke")?;
        let host = &self.host;
        let Some(path) = folder.to_str() else {
            return Err(format!("{} is not UTF-8", folder.display()));
        };
        let quoted = format!("'{}'", path.replace('\'', r"'\''"));
        let asked = writeln!(channel.input, "l {quoted}").and_then(|()| channel.input.flush());
        if let Err(error) = asked {
            return Err(format!("ssh to {host}: {error}; {}", channel.ended()));
        }
        let mut entries = Vec::new();
        let mut record = Vec::new();
        loop {
            record.clear();
            match channel.output.read_until(0, &mut record) {
                Ok(_) if record.last() == Some(&0) => {}
                Ok(_) => return Err(format!("ssh to {host} ended: {}", channel.ended())),
                Err(error) => return Err(format!("ssh to {host}: {error}")),
            }
            record.pop();
            let (kind, name) = record.split_first().ok_or("an empty record")?;
            let name = String::from_utf8_lossy(name).into_owned();
            match kind {
                b'Z' => break,
                b'E' => return Err(format!("{path} does not read on {host}")),
                b'd' | b'f' => entries.push(Entry {
                    name: OsString::from(name),
                    folder: *kind == b'd',
                    modified: SystemTime::UNIX_EPOCH,
                }),
                _ => return Err(format!("{host} answered {record:?}")),
            }
        }
        sort(&mut entries);
        Ok(entries)
    }
}

impl Channel {
    /// What ssh said as it ended.
    fn ended(&mut self) -> String {
        let mut said = String::new();
        if let Some(mut errors) = self.errors.take()
            && let Err(error) = errors.read_to_string(&mut said)
        {
            said = format!("its errors do not read: {error}");
        }
        match self.child.wait() {
            Ok(status) => format!("{}; {status}", said.trim()),
            Err(error) => format!("{}; it does not end: {error}", said.trim()),
        }
    }
}

impl Drop for Channel {
    fn drop(&mut self) {
        match self.child.kill().and_then(|()| self.child.wait()) {
            Ok(status) => log::debug!("files: an ssh lister ends, {status}"),
            Err(error) => log::debug!("files: an ssh lister had ended: {error}"),
        }
    }
}
