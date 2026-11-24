//! The ssh a pane runs, and the options and destination that reach its host again.

use anyhow::Context as _;

use super::{Pty, TerminalView};

impl TerminalView {
    /// The options and destination of the ssh leading its foreground.
    pub fn reach(&self) -> anyhow::Result<Option<Vec<String>>> {
        reach(&self.pty)
    }
}

/// Options that take a value and reach the host: bind, cipher, config, identity, jump, login, MAC, tag, port, control path, PKCS#11, `-o`.
const KEPT: &str = "BbcFiIJlmPpSo";
/// Options that take a value and serve the session alone: forwards, escape, log, control command, stdio, tunnel.
const DROPPED: &str = "DEeLORWw";
/// Flags that reach the host: address family, compression.
const FLAGS: &str = "46C";

/// The options and destination of the ssh leading the pane's foreground, none while no ssh leads it.
pub fn reach(pty: &Pty) -> anyhow::Result<Option<Vec<String>>> {
    let Some(args) = ssh_args(pty)? else {
        return Ok(None);
    };
    let reach = options(&args).with_context(|| format!("ssh: {args:?} names no destination"))?;
    log::info!("ssh: pane {} reaches {reach:?}", pty.pid);
    Ok(Some(reach))
}

#[cfg(unix)]
fn ssh_args(pty: &Pty) -> anyhow::Result<Option<Vec<String>>> {
    use super::unix::{args, path};
    let Some(group) = pty.foreground()? else {
        return Ok(None);
    };
    let exe = path(group).with_context(|| format!("ssh: process {group} does not read"))?;
    if exe.file_name().is_none_or(|name| name != "ssh") {
        return Ok(None);
    }
    Ok(Some(args(group).with_context(|| {
        format!("ssh: process {group}'s arguments")
    })?))
}

/// Windows has no foreground group: the nearest ssh under the session's shell.
#[cfg(windows)]
fn ssh_args(pty: &Pty) -> anyhow::Result<Option<Vec<String>>> {
    use super::procs::{launch, processes};
    let all = processes()?;
    let (mut under, mut at) = (vec![pty.pid], 0);
    while let Some(&pid) = under.get(at) {
        at += 1;
        let ssh = all
            .iter()
            .any(|(id, _, name)| *id == pid && name.eq_ignore_ascii_case("ssh.exe"));
        if ssh && pid != pty.pid {
            let (args, _) = launch(pid).with_context(|| format!("ssh: process {pid}"))?;
            return Ok(Some(args));
        }
        for (child, ..) in all.iter().filter(|(_, of, _)| *of == pid) {
            if !under.contains(child) {
                under.push(*child);
            }
        }
    }
    Ok(None)
}

/// `args`, ssh's own first, less forwards, flags and the command after the destination.
fn options(args: &[String]) -> Option<Vec<String>> {
    let mut kept = Vec::new();
    let mut words = args.iter().skip(1);
    while let Some(word) = words.next() {
        if word == "--" {
            return words.next().map(|destination| {
                kept.push(destination.clone());
                kept
            });
        }
        let Some(bundle) = word.strip_prefix('-').filter(|bundle| !bundle.is_empty()) else {
            kept.push(word.clone());
            return Some(kept);
        };
        for (at, flag) in bundle.char_indices() {
            if KEPT.contains(flag) || DROPPED.contains(flag) {
                let rest = &bundle[at + flag.len_utf8()..];
                let value = if rest.is_empty() {
                    words.next()?.clone()
                } else {
                    rest.to_string()
                };
                if KEPT.contains(flag) {
                    kept.extend([format!("-{flag}"), value]);
                }
                break;
            }
            if FLAGS.contains(flag) {
                kept.push(format!("-{flag}"));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::options;

    fn reach(line: &str) -> Option<Vec<String>> {
        let args: Vec<String> = line.split(' ').map(String::from).collect();
        options(&args)
    }

    #[test]
    fn the_destination_keeps_what_reaches_it_and_drops_forwards_and_the_command() {
        assert_eq!(reach("ssh host").unwrap(), ["host"]);
        assert_eq!(
            reach("ssh -p 2222 -i key -L 8080:x:80 -tA me@host htop").unwrap(),
            ["-p", "2222", "-i", "key", "me@host"]
        );
        assert_eq!(
            reach("ssh -4Cp22 -oUser=me -J jump host").unwrap(),
            [
                "-4", "-C", "-p", "22", "-o", "User=me", "-J", "jump", "host"
            ]
        );
        assert_eq!(
            reach("ssh -D 1080 -N -- host").unwrap(),
            ["host"],
            "a dynamic forward and -N go"
        );
        assert!(reach("ssh -p").is_none(), "an option without its value");
        assert!(reach("ssh -v").is_none(), "no destination");
    }
}
