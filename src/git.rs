use std::{
    ffi::OsStr,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use anyhow::{Context as _, anyhow, bail, ensure};
use ely_gpui_component::{
    git::{ChangeSection, Commit},
    lists::GitStatus,
};

mod diff;
pub mod panel;
#[cfg(any(windows, test))]
pub(crate) mod wsl;

pub use diff::diff;

/// Git's arguments on one line, for messages.
fn line<A: AsRef<OsStr>>(args: &[A]) -> String {
    let args: Vec<_> = args
        .iter()
        .map(|arg| arg.as_ref().to_string_lossy())
        .collect();
    args.join(" ")
}

/// Keeps Windows from opening a console window for git and curl.
#[cfg(windows)]
pub(crate) const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Runs git in `folder`: English messages, literal paths, no lock the user's own git could meet; `input` feeds stdin.
fn git<A: AsRef<OsStr>>(folder: &Path, args: &[A], input: &[u8]) -> anyhow::Result<Output> {
    let named = || format!("git: `git {}` in {}", line(args), folder.display());
    let mut command = command(folder);
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(&mut command, CREATE_NO_WINDOW);
    let mut child = command
        .args(["--no-optional-locks", "--literal-pathspecs"])
        .args(args)
        .env("LC_ALL", "C")
        .stdin(if input.is_empty() {
            Stdio::null()
        } else {
            Stdio::piped()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("{} did not start", named()))?;
    let wrote = child
        .stdin
        .take()
        .map_or(Ok(()), |mut stdin| stdin.write_all(input));
    let output = child
        .wait_with_output()
        .with_context(|| format!("{} did not end", named()))?;
    if output.status.success() {
        wrote.with_context(|| format!("{} took no input", named()))?;
    }
    Ok(output)
}

/// git in `folder`: inside its WSL distribution where it lies in one, since Windows' git refuses that share as another's.
fn command(folder: &Path) -> Command {
    #[cfg(windows)]
    if let Some((share, inside)) = wsl::split(folder) {
        let shared =
            std::env::var("WSLENV").map_or("LC_ALL".into(), |list| format!("{list}:LC_ALL"));
        let mut command = Command::new("wsl.exe");
        command
            .args(["-d", wsl::distro(share), "--exec", "git", "-C", &inside])
            .env("WSLENV", shared)
            .env("WSL_UTF8", "1");
        return command;
    }
    let mut command = Command::new("git");
    command.arg("-C").arg(folder);
    command
}

/// What git said about a failure: its errors, else its output, else its exit status.
fn said(output: &Output) -> String {
    [&output.stderr, &output.stdout]
        .into_iter()
        .map(|text| String::from_utf8_lossy(text).trim().to_string())
        .find(|text| !text.is_empty())
        .unwrap_or_else(|| output.status.to_string())
}

/// Logs what git warned of while it still did its work.
fn warnings<A: AsRef<OsStr>>(folder: &Path, args: &[A], output: &Output) {
    let warned = String::from_utf8_lossy(&output.stderr);
    if !warned.trim().is_empty() {
        let (command, place) = (line(args), folder.display());
        log::warn!("git: `git {command}` in {place} warned: {}", warned.trim());
    }
}

/// Runs git and gives what it printed; a failure is an error carrying what git said.
fn run<A: AsRef<OsStr>>(folder: &Path, args: &[A], input: &[u8]) -> anyhow::Result<Vec<u8>> {
    let output = git(folder, args, input)?;
    let (command, place) = (line(args), folder.display());
    ensure!(
        output.status.success(),
        "git: `git {command}` failed in {place}: {}",
        said(&output)
    );
    warnings(folder, args, &output);
    Ok(output.stdout)
}

/// A path from git's bytes: as they are on Unix; elsewhere UTF-8, as git writes them.
fn path_of(bytes: &[u8]) -> anyhow::Result<PathBuf> {
    #[cfg(unix)]
    let path = <OsStr as std::os::unix::ffi::OsStrExt>::from_bytes(bytes);
    #[cfg(not(unix))]
    let path = std::str::from_utf8(bytes).with_context(|| {
        let shown = String::from_utf8_lossy(bytes);
        format!("git: path {shown:?} is not UTF-8")
    })?;
    Ok(PathBuf::from(path))
}

/// A record of `git status --porcelain=v2 -z` in git's bytes: index and work tree codes, its path from the top, a rename's or copy's old path.
#[derive(Debug, PartialEq)]
struct Record {
    x: u8,
    y: u8,
    path: Vec<u8>,
    from: Option<Vec<u8>>,
}

/// HEAD as status reads it: its branch, none when detached; its commit, none before the first.
#[derive(Debug, Default, PartialEq)]
struct Head {
    branch: Option<String>,
    commit: Option<String>,
}

/// Porcelain v2 records, each ended by NUL: headers name HEAD; a rename's or copy's old path follows it.
fn parse(out: &[u8]) -> anyhow::Result<(Head, Vec<Record>)> {
    let mut fields = out
        .split(|&byte| byte == 0)
        .filter(|field| !field.is_empty());
    let (mut head, mut records) = (Head::default(), Vec::new());
    while let Some(field) = fields.next() {
        let malformed = || {
            let shown = String::from_utf8_lossy(field);
            anyhow!("git: status record {shown:?} is malformed")
        };
        let mut words = field.splitn(2, |&byte| byte == b' ');
        let (Some(kind), Some(rest)) = (words.next(), words.next()) else {
            return Err(malformed());
        };
        // Fields before the path, which may hold spaces.
        let count = match kind {
            b"#" => {
                match String::from_utf8_lossy(rest).split_once(' ') {
                    Some(("branch.oid", oid)) => {
                        head.commit = (oid != "(initial)").then(|| oid.into())
                    }
                    Some(("branch.head", name)) => {
                        head.branch = (name != "(detached)").then(|| name.into())
                    }
                    _ => {}
                }
                continue;
            }
            b"?" => 0,
            b"1" => 7,
            b"2" => 8,
            b"u" => 9,
            _ => return Err(malformed()),
        };
        let parts: Vec<&[u8]> = rest.splitn(count + 1, |&byte| byte == b' ').collect();
        let (codes, path) = match (kind, parts.as_slice()) {
            (b"?", [path]) => (b"??".as_slice(), *path),
            (_, [codes, .., path]) if parts.len() == count + 1 => (*codes, *path),
            _ => return Err(malformed()),
        };
        let (&[x, y], false) = (codes, path.is_empty()) else {
            return Err(malformed());
        };
        let from = match kind {
            b"2" => Some(fields.next().ok_or_else(malformed)?.to_vec()),
            _ => None,
        };
        let path = path.to_vec();
        records.push(Record { x, y, path, from });
    }
    Ok((head, records))
}

/// One mark for an index and a work tree code, `.` for none; a copy is a new file.
fn kind(x: u8, y: u8) -> Option<GitStatus> {
    Some(match (x, y) {
        (b'?', b'?') => GitStatus::Untracked,
        (b'U', _) | (_, b'U') | (b'A', b'A') | (b'D', b'D') => GitStatus::Conflicted,
        (b'D', _) | (_, b'D') => GitStatus::Deleted,
        (b'R', _) | (_, b'R') => GitStatus::Renamed,
        (b'A' | b'C', _) | (_, b'A' | b'C') => GitStatus::Added,
        (b'M' | b'T', _) | (_, b'M' | b'T') => GitStatus::Modified,
        _ => return None,
    })
}

/// A record's mark from `x` and `y`; codes git never writes are an error.
fn mark(record: &Record, x: u8, y: u8) -> anyhow::Result<GitStatus> {
    let (codes, path) = (
        format!("{}{}", x as char, y as char),
        String::from_utf8_lossy(&record.path),
    );
    kind(x, y).with_context(|| format!("git: {path} has codes {codes}"))
}

/// Each change at or under `folder`, by its full path; none outside a work tree.
pub fn changes(folder: &Path) -> anyhow::Result<Option<Vec<(PathBuf, GitStatus)>>> {
    let place = ["rev-parse", "--is-inside-work-tree", "--show-prefix"];
    let place = git(folder, &place, &[])?;
    if !place.status.success() {
        let error = said(&place);
        if error.contains("not a git repository") {
            return Ok(None);
        }
        bail!("git: rev-parse in {} failed: {error}", folder.display());
    }
    // The prefix may hold newlines of its own.
    let answer = place.stdout.strip_suffix(b"\n").and_then(|out| {
        let at = out.iter().position(|&byte| byte == b'\n')?;
        Some((&out[..at], &out[at + 1..]))
    });
    let prefix = match answer {
        Some((b"true", prefix)) => prefix,
        Some((b"false", _)) => return Ok(None),
        _ => {
            let printed = String::from_utf8_lossy(&place.stdout);
            bail!("git: rev-parse in {} printed {printed:?}", folder.display())
        }
    };
    let status = ["status", "--porcelain=v2", "-z", "--", "."];
    let (_, records) = parse(&run(folder, &status, &[])?)?;
    // WSL's git names files as Linux does; Files reads them through the share.
    #[cfg(windows)]
    let shared = wsl::split(folder).is_some();
    records
        .into_iter()
        .map(|record| {
            let status = mark(&record, record.x, record.y)?;
            let Some(rest) = record.path.strip_prefix(prefix) else {
                let path = String::from_utf8_lossy(&record.path);
                let prefix = String::from_utf8_lossy(prefix);
                bail!("git: {path} lies outside {prefix}");
            };
            let mut path = folder.to_path_buf();
            for part in rest.split(|&byte| byte == b'/') {
                if !part.is_empty() {
                    let part = path_of(part)?;
                    #[cfg(windows)]
                    let part = match shared {
                        true => PathBuf::from(wsl::to_windows(&part.display().to_string())),
                        false => part,
                    };
                    path.push(part);
                }
            }
            Ok((path, status))
        })
        .collect::<anyhow::Result<_>>()
        .map(Some)
}

/// A changed file, by its path from the work tree's top; a staged rename keeps its old path.
#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    pub path: PathBuf,
    pub from: Option<PathBuf>,
    pub status: GitStatus,
}

/// A work tree as the Git panel shows it.
#[derive(Debug, PartialEq)]
pub struct Repo {
    /// The work tree's top folder.
    pub top: PathBuf,
    /// The branch checked out; none when HEAD is detached.
    pub branch: Option<String>,
    /// HEAD's commit; none before the first.
    pub commit: Option<String>,
    pub conflicted: Vec<Change>,
    pub staged: Vec<Change>,
    pub unstaged: Vec<Change>,
    pub untracked: Vec<Change>,
}

/// The work tree holding `folder`, read whole; none outside one.
pub fn repo(folder: &Path) -> anyhow::Result<Option<Repo>> {
    let answer = git(folder, &["rev-parse", "--show-toplevel"], &[])?;
    if !answer.status.success() {
        let error = said(&answer);
        if error.contains("not a git repository") || error.contains("must be run in a work tree") {
            return Ok(None);
        }
        bail!("git: rev-parse in {} failed: {error}", folder.display());
    }
    let top = answer.stdout.strip_suffix(b"\n").with_context(|| {
        let printed = String::from_utf8_lossy(&answer.stdout);
        format!("git: rev-parse in {} printed {printed:?}", folder.display())
    })?;
    let top = path_of(top)?;
    // WSL's git names the top inside its distribution.
    #[cfg(windows)]
    let top = match wsl::split(folder) {
        Some((share, _)) => wsl::join(share, &top),
        None => top,
    };
    let status = ["status", "--porcelain=v2", "--branch", "-z"];
    let (head, records) = parse(&run(&top, &status, &[])?)?;
    let mut repo = Repo {
        top,
        branch: head.branch,
        commit: head.commit,
        conflicted: Vec::new(),
        staged: Vec::new(),
        unstaged: Vec::new(),
        untracked: Vec::new(),
    };
    for record in records {
        let path = path_of(&record.path)?;
        let change = |status, from| Change {
            path: path.clone(),
            from,
            status,
        };
        match mark(&record, record.x, record.y)? {
            GitStatus::Untracked => repo.untracked.push(change(GitStatus::Untracked, None)),
            GitStatus::Conflicted => repo.conflicted.push(change(GitStatus::Conflicted, None)),
            _ => {
                if record.x != b'.' {
                    // A copy's source is no part of it.
                    let from = match (record.x, &record.from) {
                        (b'R', Some(from)) => Some(path_of(from)?),
                        _ => None,
                    };
                    repo.staged
                        .push(change(mark(&record, record.x, b'.')?, from));
                }
                if record.y != b'.' {
                    repo.unstaged
                        .push(change(mark(&record, b'.', record.y)?, None));
                }
            }
        }
    }
    Ok(Some(repo))
}

/// Paths for git's `--pathspec-from-file`: each change's and a rename's old one, NUL-ended, as git's bytes.
fn pathspecs(changes: &[Change]) -> anyhow::Result<Vec<u8>> {
    // No path would take the whole tree.
    ensure!(!changes.is_empty(), "git: no file named");
    let mut out = Vec::new();
    for change in changes {
        for path in std::iter::once(&change.path).chain(&change.from) {
            out.extend_from_slice(path.as_os_str().as_encoded_bytes());
            out.push(0);
        }
    }
    Ok(out)
}

/// The first few changes by name, for a log line, and how many more.
fn names(changes: &[Change]) -> String {
    let mut names: Vec<String> = changes
        .iter()
        .take(5)
        .map(|change| change.path.display().to_string())
        .collect();
    if changes.len() > names.len() {
        names.push(format!("{} more", changes.len() - names.len()));
    }
    names.join(", ")
}

/// Stages `changes`: new, changed and gone files alike.
pub fn stage(top: &Path, changes: &[Change]) -> anyhow::Result<()> {
    let args = ["add", "-A", "--pathspec-from-file=-", "--pathspec-file-nul"];
    run(top, &args, &pathspecs(changes)?)?;
    log::info!("git: staged {} in {}", names(changes), top.display());
    Ok(())
}

/// Unstages `changes`, before the first commit too.
pub fn unstage(top: &Path, changes: &[Change]) -> anyhow::Result<()> {
    let args = [
        "reset",
        "-q",
        "--pathspec-from-file=-",
        "--pathspec-file-nul",
    ];
    run(top, &args, &pathspecs(changes)?)?;
    log::info!("git: unstaged {} in {}", names(changes), top.display());
    Ok(())
}

/// Commits what is staged with `message`.
pub fn commit(top: &Path, message: &str, amend: bool) -> anyhow::Result<()> {
    let args: &[&str] = match amend {
        true => &["commit", "-q", "--amend", "-F", "-"],
        false => &["commit", "-q", "-F", "-"],
    };
    run(top, args, message.as_bytes())?;
    let amended = if amend { ", amending HEAD" } else { "" };
    log::info!("git: committed in {}{amended}", top.display());
    Ok(())
}

/// Discards `change` from the work tree: an edit goes back to the index; a new file, untracked or only intended to be added, goes.
pub fn discard(top: &Path, change: &Change, section: ChangeSection) -> anyhow::Result<()> {
    let shown = change.path.display();
    let one = || pathspecs(std::slice::from_ref(change));
    match (section, change.status) {
        (ChangeSection::Untracked, _) => {
            let flags = ["clean", "-f", "-d", "--"].map(OsStr::new);
            let args: Vec<&OsStr> = flags.into_iter().chain([change.path.as_os_str()]).collect();
            // Git passes over a repository of its own without a word.
            let removed = run(top, &args, &[])?;
            ensure!(!removed.is_empty(), "git: clean removed nothing at {shown}");
        }
        (ChangeSection::Unstaged, GitStatus::Added | GitStatus::Renamed) => {
            let args = [
                "rm",
                "-f",
                "-q",
                "--pathspec-from-file=-",
                "--pathspec-file-nul",
            ];
            run(top, &args, &one()?)?;
        }
        (ChangeSection::Unstaged, _) => {
            let args = [
                "restore",
                "--worktree",
                "--pathspec-from-file=-",
                "--pathspec-file-nul",
            ];
            run(top, &args, &one()?)?;
        }
        _ => bail!("git: {shown} has nothing to discard in {section:?}"),
    }
    log::info!("git: discarded {shown} in {}", top.display());
    Ok(())
}

/// The newest `count` commits from HEAD, which must have one.
pub fn log(top: &Path, count: usize) -> anyhow::Result<Vec<Commit>> {
    let count = format!("-n{count}");
    let format = "--format=%H%x00%P%x00%s%x00%an%x00%ar%x00%D";
    let args = ["log", &count, "-z", "--no-show-signature", format];
    let out = run(top, &args, &[])?;
    let out = String::from_utf8_lossy(&out);
    let fields: Vec<&str> = out.split_terminator('\0').collect();
    ensure!(
        fields.len().is_multiple_of(6),
        "git: log in {} printed {} fields, not six for each commit",
        top.display(),
        fields.len()
    );
    let listed = |text: &str, by: &str| -> Vec<_> {
        let parts = text.split(by).filter(|part| !part.is_empty());
        parts.map(|part| part.to_string().into()).collect()
    };
    let commits = fields.chunks(6).map(|commit| Commit {
        id: commit[0].to_string().into(),
        parents: listed(commit[1], " "),
        subject: commit[2].to_string().into(),
        author: commit[3].to_string().into(),
        when: commit[4].to_string().into(),
        refs: listed(commit[5], ", "),
    });
    Ok(commits.collect())
}

#[cfg(test)]
mod tests;
