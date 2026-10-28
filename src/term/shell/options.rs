use std::path::Path;

use super::name;

/// How a shell leading its own prompt reads its options.
pub(crate) struct Options {
    /// Characters that start an option.
    marks: &'static [char],
    /// Short options may share one argument, as `-lo vi`.
    bundles: bool,
    /// Short options taking a value.
    short: &'static str,
    /// Of those, the ones whose value may be left out.
    optional: &'static str,
    /// Bundled values each take the next argument, as bash does.
    consume: bool,
    /// Short options that run a command, not a prompt.
    command_short: &'static str,
    /// Short options after whose argument a script follows.
    stop_short: &'static str,
    /// Arguments after which a script follows.
    stops: &'static [&'static str],
    /// Every long option, by name or by a unique prefix.
    long: &'static [&'static str],
    /// A unique prefix names a long option.
    prefixes: bool,
    /// Options taking every following argument that is no option.
    many: &'static [&'static str],
    /// Options taking the next argument, unless `=` holds one.
    valued: &'static [&'static str],
    /// Options that run a command or a file.
    command: &'static [&'static str],
    /// Names match ignoring case, as PowerShell's do.
    fold: bool,
}

impl Options {
    /// Whether `args`, past the program's name, start an interactive shell.
    pub(crate) fn interactive(&self, args: &[String]) -> bool {
        let mut args = args.iter();
        while let Some(arg) = args.next() {
            // PowerShell takes `--name` and `/name` as `-name`.
            let switch = if self.fold {
                // PowerShell trims blanks and reads Unicode dashes as `-`.
                let trimmed = arg.trim();
                let dashes = ['-', '\u{2013}', '\u{2014}', '\u{2015}'];
                let rest = trimmed.strip_prefix('/').or_else(|| {
                    let first = trimmed.chars().next().filter(|c| dashes.contains(c))?;
                    let rest = &trimmed[first.len_utf8()..];
                    // A second same dash goes too, as PowerShell drops it.
                    Some(rest.strip_prefix(first).unwrap_or(rest))
                });
                rest.map_or_else(|| trimmed.to_string(), |rest| format!("-{rest}"))
            } else {
                arg.clone()
            };
            let arg = &switch;
            if self.stops.contains(&arg.as_str()) {
                return args.next().is_none();
            }
            let Some(rest) = arg.strip_prefix(self.marks) else {
                return false;
            };
            if !self.bundles || arg.starts_with("--") || self.long.contains(&arg.as_str()) {
                let Some(name) = self.resolve(arg.split('=').next().unwrap_or(arg)) else {
                    return false;
                };
                if self.command.iter().any(|command| self.same(command, name)) {
                    return false;
                }
                if self.many.contains(&name) && !arg.contains('=') {
                    let mut rest = args.clone();
                    while rest
                        .next()
                        .is_some_and(|value| !value.starts_with(self.marks))
                    {
                        args.next();
                    }
                    continue;
                }
                let valued = self.valued.iter().any(|valued| self.same(valued, name));
                if valued && !arg.contains('=') && args.next().is_none() {
                    return false;
                }
                continue;
            }
            let (mut values, mut stop) = (Vec::new(), false);
            for (at, option) in rest.char_indices() {
                if self.command_short.contains(option) {
                    return false;
                }
                stop |= self.stop_short.contains(option);
                if self.short.contains(option) {
                    let last = at + option.len_utf8() == rest.len();
                    if self.consume || last {
                        values.push(option);
                    }
                    if !self.consume {
                        break;
                    }
                }
            }
            for option in values {
                if args.next().is_none() && !self.optional.contains(option) {
                    return false;
                }
            }
            if stop {
                return args.next().is_none();
            }
        }
        true
    }

    fn same(&self, name: &str, arg: &str) -> bool {
        if self.fold {
            name.eq_ignore_ascii_case(arg)
        } else {
            name == arg
        }
    }

    /// The long option `name` names, whole or by a unique prefix.
    fn resolve<'a>(&self, name: &'a str) -> Option<&'a str> {
        if self.fold {
            let lower = name.to_ascii_lowercase();
            // The first switch it starts, as PowerShell matches, wins.
            return PWSH
                .iter()
                .find(|(full, least)| lower.len() >= least.len() && full.starts_with(&lower))
                .map(|(full, _)| *full);
        }
        let known = |list: &[&str]| list.iter().any(|known| self.same(known, name));
        if !self.prefixes || known(self.long) || known(self.valued) || known(self.command) {
            return Some(name);
        }
        let lower = name.to_ascii_lowercase();
        let mut matches = self.long.iter().filter(|long| {
            if self.fold {
                long.to_ascii_lowercase().starts_with(&lower)
            } else {
                long.starts_with(name)
            }
        });
        match (matches.next(), matches.next()) {
            (Some(long), None) => Some(long),
            _ => None,
        }
    }
}

/// PowerShell's switches in its parser's order, each with its least prefix.
const PWSH: &[(&str, &str)] = &[
    ("-version", "-v"),
    ("-help", "-h"),
    ("-?", "-?"),
    ("-login", "-l"),
    ("-noexit", "-noe"),
    ("-noprofile", "-nop"),
    ("-nologo", "-nol"),
    ("-noninteractive", "-noni"),
    ("-socketservermode", "-so"),
    ("-servermode", "-s"),
    ("-namedpipeservermode", "-nam"),
    ("-sshservermode", "-sshs"),
    ("-noprofileloadtime", "-noprofileloadtime"),
    ("-interactive", "-i"),
    ("-configurationfile", "-configurationfile"),
    ("-configurationname", "-config"),
    ("-custompipename", "-cus"),
    ("-commandwithargs", "-commandwithargs"),
    ("-cwa", "-cwa"),
    ("-command", "-c"),
    ("-windowstyle", "-w"),
    ("-file", "-f"),
    ("-isswait", "-isswait"),
    ("-outputformat", "-o"),
    ("-of", "-o"),
    ("-inputformat", "-inp"),
    ("-if", "-if"),
    ("-executionpolicy", "-ex"),
    ("-ep", "-ep"),
    ("-encodedcommand", "-e"),
    ("-ec", "-e"),
    ("-encodedarguments", "-encodeda"),
    ("-ea", "-ea"),
    ("-settingsfile", "-settings"),
    ("-sta", "-sta"),
    ("-mta", "-mta"),
    ("-workingdirectory", "-wo"),
    ("-wd", "-wd"),
    (
        "-removeworkingdirectorytrailingcharacter",
        "-removeworkingdirectorytrailingcharacter",
    ),
];

/// A shell's options where it leads its prompt; none elsewhere.
pub(crate) fn options(path: &Path) -> Option<Options> {
    let plain = Options {
        marks: &['-'],
        bundles: false,
        short: "",
        optional: "",
        consume: false,
        command_short: "",
        stop_short: "",
        stops: &["--", "-"],
        long: &[],
        prefixes: false,
        many: &[],
        valued: &[],
        command: &[],
        fold: false,
    };
    let options = match name(path).as_str() {
        "zsh" => Options {
            marks: &['-', '+'],
            bundles: true,
            short: "o",
            command_short: "c",
            stop_short: "b-",
            stops: &["--", "-", "+", "+-"],
            valued: &["--emulate"],
            ..plain
        },
        "bash" => Options {
            marks: &['-', '+'],
            bundles: true,
            short: "oO",
            optional: "oO",
            consume: true,
            command_short: "c",
            long: &[
                "-rcfile",
                "-init-file",
                "-login",
                "-noprofile",
                "-norc",
                "-posix",
                "-verbose",
                "-noediting",
                "-restricted",
                "-debugger",
                "-protected",
                "-wordexp",
            ],
            valued: &["--rcfile", "--init-file", "-rcfile", "-init-file"],
            ..plain
        },
        "fish" => Options {
            bundles: true,
            short: "CdopfD",
            command_short: "c",
            stops: &["--"],
            long: &[
                "--command",
                "--init-command",
                "--features",
                "--debug",
                "--debug-output",
                "--debug-stack-frames",
                "--interactive",
                "--login",
                "--no-config",
                "--no-execute",
                "--print-rusage-self",
                "--print-debug-categories",
                "--install",
                "--profile",
                "--profile-startup",
                "--private",
                "--help",
                "--version",
            ],
            prefixes: true,
            valued: &[
                "--init-command",
                "--debug",
                "--debug-output",
                "--profile",
                "--profile-startup",
                "--features",
                "--debug-stack-frames",
            ],
            command: &["--command"],
            ..plain
        },
        "nu" => Options {
            bundles: true,
            short: "emI",
            command_short: "c",
            many: &["--log-include", "--log-exclude", "--experimental-options"],
            valued: &[
                "-e",
                "--execute",
                "-m",
                "--table-mode",
                "--error-style",
                "-I",
                "--include-path",
                "--config",
                "--env-config",
                "--config-home",
                "--log-level",
                "--log-target",
                "--log-file",
                "--log-include",
                "--log-exclude",
                "--experimental-options",
                "--plugin-config",
                "--plugins",
                "--mcp-transport",
                "--mcp-port",
                "--mcp-host",
            ],
            command: &["-c", "--commands"],
            ..plain
        },
        "pwsh" | "powershell" => Options {
            valued: &[
                "-EncodedArguments",
                "-ea",
                "-ExecutionPolicy",
                "-ex",
                "-ep",
                "-WorkingDirectory",
                "-wd",
                "-ConfigurationName",
                "-config",
                "-ConfigurationFile",
                "-CustomPipeName",
                "-SettingsFile",
                "-settings",
                "-OutputFormat",
                "-o",
                "-of",
                "-InputFormat",
                "-inp",
                "-if",
                "-WindowStyle",
                "-w",
            ],
            command: &[
                "-Command",
                "-c",
                "-File",
                "-f",
                "-EncodedCommand",
                "-e",
                "-ec",
                "-CommandWithArgs",
                "-cwa",
            ],
            long: &[
                "-Command",
                "-CommandWithArgs",
                "-ConfigurationName",
                "-ConfigurationFile",
                "-CustomPipeName",
                "-EncodedCommand",
                "-ExecutionPolicy",
                "-File",
                "-Help",
                "-InputFormat",
                "-Interactive",
                "-Login",
                "-MTA",
                "-NoExit",
                "-NoLogo",
                "-NonInteractive",
                "-NoProfile",
                "-NoProfileLoadTime",
                "-OutputFormat",
                "-SettingsFile",
                "-SSHServerMode",
                "-STA",
                "-Version",
                "-WindowStyle",
                "-WorkingDirectory",
                "-nol",
                "-nop",
                "-noni",
                "-noe",
                "-h",
                "-?",
                "-l",
                "-v",
                "-sta",
                "-mta",
            ],
            prefixes: true,
            fold: true,
            ..plain
        },
        _ => return None,
    };
    Some(options)
}

/// Whether `path` runs fish or nu, whose own lines mark clicks.
#[cfg(unix)]
pub(crate) fn clicks(path: &Path) -> bool {
    matches!(name(path).as_str(), "fish" | "nu")
}

/// Whether `path` relays another host's, user's or container's terminal.
#[cfg(unix)]
pub(crate) fn relays(path: &Path) -> bool {
    matches!(
        name(path).as_str(),
        "ssh" | "mosh-client" | "sudo" | "su" | "doas" | "docker" | "podman" | "kubectl"
    )
}
