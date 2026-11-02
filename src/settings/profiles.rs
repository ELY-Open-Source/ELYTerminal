//! Profiles: named command lines the new-session menu starts, as Windows Terminal's.

use anyhow::Context as _;
use serde::{Deserialize, Serialize};

use super::{program_path, start_folder};

/// A program and its arguments under a name, and where it starts.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Profile {
    pub name: String,
    /// Split as the system's shell splits a line, or as Windows splits one.
    pub command: String,
    /// Where it starts; empty keeps Start in.
    pub folder: String,
}

impl Profile {
    /// Its name, program and folder are sound; the error names it.
    pub fn check(&self) -> anyhow::Result<()> {
        anyhow::ensure!(!self.name.trim().is_empty(), "a profile has no name");
        let name = &self.name;
        let words = words(&self.command).with_context(|| format!("profile {name}"))?;
        program_path(&words[0]).with_context(|| format!("profile {name}"))?;
        if !self.folder.is_empty() {
            start_folder(&self.folder).with_context(|| format!("profile {name}"))?;
        }
        Ok(())
    }
}

/// `program` as a command line of one word, which `words` gives back.
#[cfg(unix)]
pub fn command_line(program: &str) -> anyhow::Result<String> {
    let quoted = shlex::try_quote(program).with_context(|| format!("{program:?} quotes no way"))?;
    Ok(quoted.into_owned())
}

/// `program` as a command line of one word; Windows reads no quote inside a path.
#[cfg(windows)]
pub fn command_line(program: &str) -> anyhow::Result<String> {
    anyhow::ensure!(!program.contains('"'), "{program:?} holds a quote");
    Ok(format!("\"{program}\""))
}

/// `command`'s program and arguments, as a POSIX shell splits it.
#[cfg(unix)]
pub fn words(command: &str) -> anyhow::Result<Vec<String>> {
    let words = shlex::split(command)
        .with_context(|| format!("command {command:?} leaves a quote open"))?;
    anyhow::ensure!(!words.is_empty(), "command is empty");
    Ok(words)
}

/// `command`'s program and arguments, as `CommandLineToArgvW` splits it.
#[cfg(windows)]
pub fn words(command: &str) -> anyhow::Result<Vec<String>> {
    use windows_sys::Win32::{Foundation::LocalFree, UI::Shell::CommandLineToArgvW};
    // An empty line names this program itself.
    anyhow::ensure!(!command.trim().is_empty(), "command is empty");
    let wide: Vec<u16> = command.trim_start().encode_utf16().chain([0]).collect();
    let mut count = 0;
    // SAFETY: `wide` ends in a NUL and outlives the call.
    let argv = unsafe { CommandLineToArgvW(wide.as_ptr(), &mut count) };
    anyhow::ensure!(
        !argv.is_null(),
        "command {command:?}: {}",
        std::io::Error::last_os_error()
    );
    // SAFETY: `argv` holds `count` NUL-ended strings until it is freed.
    let words = (0..count as usize)
        .map(|at| unsafe {
            let word = *argv.add(at);
            let len = (0..).take_while(|&i| *word.add(i) != 0).count();
            String::from_utf16_lossy(std::slice::from_raw_parts(word, len))
        })
        .collect();
    // SAFETY: CommandLineToArgvW allocated `argv` with LocalAlloc.
    unsafe { LocalFree(argv.cast()) };
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_command_splits_into_its_program_and_arguments() {
        let split = |command: &str| words(command).expect(command);
        assert_eq!(
            split("ssh me@host -p 2222"),
            ["ssh", "me@host", "-p", "2222"]
        );
        assert_eq!(split("wsl -d Ubuntu"), ["wsl", "-d", "Ubuntu"]);
        assert!(words("").is_err());
        assert!(words("   ").is_err());
        if cfg!(windows) {
            let path = r#""C:\Program Files\Git\bin\bash.exe" -l"#;
            assert_eq!(split(path), [r"C:\Program Files\Git\bin\bash.exe", "-l"]);
            assert_eq!(split(r"C:\tools\x.exe a\b"), [r"C:\tools\x.exe", r"a\b"]);
        } else {
            assert_eq!(split("sh -c 'echo \"a b\"'"), ["sh", "-c", "echo \"a b\""]);
            assert!(words("ssh 'me@host").is_err());
        }
    }

    #[test]
    fn a_program_s_command_line_splits_back_into_it() {
        let paths = if cfg!(windows) {
            [
                r"C:\Program Files\PowerShell\7\pwsh.exe",
                r"C:\Windows\System32\cmd.exe",
            ]
        } else {
            ["/tmp/shell with space/sh", "/bin/it's"]
        };
        for path in paths {
            let line = command_line(path).expect(path);
            assert_eq!(words(&line).expect(&line), [path]);
        }
    }

    #[test]
    fn a_profile_needs_a_name_a_program_and_a_real_folder() {
        let shell = if cfg!(windows) { "cmd" } else { "sh" };
        let profile = |name: &str, command: &str, folder: &str| Profile {
            name: name.into(),
            command: command.into(),
            folder: folder.into(),
        };
        assert!(profile("Shell", shell, "").check().is_ok());
        let home = dirs::home_dir().expect("a home");
        let home = home.to_str().expect("text");
        assert!(
            profile("Shell", &format!("{shell} -x"), home)
                .check()
                .is_ok()
        );
        assert!(profile(" ", shell, "").check().is_err());
        let error = profile("Gone", "no-such-program-here", "").check();
        assert!(format!("{:#}", error.expect_err("no program")).contains("profile Gone"));
        assert!(profile("Shell", shell, "relative/folder").check().is_err());
        assert!(profile("Shell", "", "").check().is_err());
    }
}
