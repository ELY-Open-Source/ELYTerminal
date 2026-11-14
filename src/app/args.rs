//! The folder and the program the command line names for the first session.

use std::{ffi::OsString, path::PathBuf};

use anyhow::Context as _;

const USAGE: &str = "usage: elyterminal [FOLDER | --working-directory FOLDER | --startingDirectory FOLDER | -d FOLDER] [-e PROGRAM [ARG...]]";

/// xdg-terminal-exec's, Windows Terminal's and the short form.
const FOLDER_OPTIONS: [&str; 3] = ["--working-directory", "--startingDirectory", "-d"];

/// What the command line asks of the first session.
#[derive(Debug, PartialEq)]
pub struct Args {
    /// Bare or after a folder option.
    pub folder: Option<PathBuf>,
    /// Everything after `-e`, as written.
    pub command: Option<(String, Vec<String>)>,
}

/// Reads `args`: a folder, bare or after an option, then `-e` and a program with its arguments.
pub fn parse(mut args: impl Iterator<Item = OsString>) -> anyhow::Result<Args> {
    let mut folder = None;
    let mut command = None;
    while let Some(arg) = args.next() {
        if arg == "-e" {
            let words = args
                .by_ref()
                .map(|word| {
                    word.into_string().map_err(|word| {
                        anyhow::anyhow!("app: -e's {} is not UTF-8", word.to_string_lossy())
                    })
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            let (program, rest) = words
                .split_first()
                .with_context(|| format!("app: -e names no program; {USAGE}"))?;
            log::info!("app: the first session runs {program} {rest:?}, as asked");
            command = Some((program.clone(), rest.to_vec()));
            break;
        }
        anyhow::ensure!(folder.is_none(), "app: more than one folder; {USAGE}");
        let arg = if FOLDER_OPTIONS.iter().any(|option| arg == *option) {
            args.next()
                .with_context(|| format!("app: the folder option has no value; {USAGE}"))?
        } else {
            arg
        };
        folder = Some(named(arg)?);
    }
    Ok(Args { folder, command })
}

/// The folder `arg` names, as a full path.
fn named(arg: OsString) -> anyhow::Result<PathBuf> {
    let path = std::path::absolute(unquoted(arg, cfg!(windows)))
        .context("app: the folder argument makes no full path")?;
    anyhow::ensure!(
        path.is_dir(),
        "app: {} names no folder; {USAGE}",
        path.display()
    );
    log::info!(
        "app: the first session starts in {}, as asked",
        path.display()
    );
    Ok(path)
}

/// Explorer's `"C:\"` arrives as `C:"`; no name holds a quote.
fn unquoted(arg: OsString, windows: bool) -> PathBuf {
    match arg.to_str().and_then(|text| text.strip_suffix('"')) {
        Some(stem) if windows => format!("{stem}\\").into(),
        _ => arg.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> impl Iterator<Item = OsString> {
        list.iter()
            .map(OsString::from)
            .collect::<Vec<_>>()
            .into_iter()
    }

    fn folder(list: &[&str]) -> anyhow::Result<Option<PathBuf>> {
        parse(args(list)).map(|parsed| parsed.folder)
    }

    #[test]
    fn no_argument_names_no_folder_and_one_folder_names_itself() {
        assert_eq!(
            parse(args(&[])).expect("no argument"),
            Args {
                folder: None,
                command: None
            }
        );
        let here = std::env::temp_dir();
        let named = folder(&[here.to_str().expect("text")]).expect("a folder");
        assert_eq!(
            named,
            Some(std::path::absolute(&here).expect("a full path"))
        );
        let current = folder(&["."]).expect("the current folder");
        assert_eq!(current, Some(std::env::current_dir().expect("a folder")));
    }

    #[test]
    fn linux_and_windows_directory_options_name_the_folder() {
        let here = std::env::temp_dir();
        let here = here.to_str().expect("text");
        for option in FOLDER_OPTIONS {
            let named = folder(&[option, here]).expect(option);
            assert_eq!(
                named,
                Some(std::path::absolute(here).expect("a full path")),
                "{option}"
            );
        }
    }

    #[test]
    fn two_folders_or_a_file_are_refused() {
        let here = std::env::temp_dir();
        let here = here.to_str().expect("text");
        let error = folder(&[here, here]).expect_err("two");
        assert!(
            error.to_string().contains("more than one folder"),
            "{error}"
        );
        let file = std::env::current_exe().expect("this test");
        let error = folder(&[file.to_str().expect("text")]).expect_err("a file");
        assert!(error.to_string().contains("names no folder"), "{error}");

        let error = folder(&["-d"]).expect_err("no value");
        assert!(error.to_string().contains("has no value"), "{error}");
        let error = folder(&["-d", here, here]).expect_err("two folders");
        assert!(
            error.to_string().contains("more than one folder"),
            "{error}"
        );
    }

    #[test]
    fn e_takes_the_program_and_every_word_after_it() {
        let parsed = parse(args(&["-e", "nvim"])).expect("a program");
        assert_eq!(parsed.command, Some(("nvim".into(), Vec::new())));
        assert_eq!(parsed.folder, None);

        let here = std::env::temp_dir();
        let here = here.to_str().expect("text");
        let parsed =
            parse(args(&["--working-directory", here, "-e", "nvim", "a b"])).expect("both");
        assert_eq!(
            parsed.folder,
            Some(std::path::absolute(here).expect("a full path"))
        );
        assert_eq!(parsed.command, Some(("nvim".into(), vec!["a b".into()])));

        // The program's own -e and -d stay its own.
        let parsed = parse(args(&["-e", "sh", "-e", "-d", here])).expect("its own");
        assert_eq!(
            parsed.command,
            Some(("sh".into(), vec!["-e".into(), "-d".into(), here.into()]))
        );
    }

    #[test]
    fn e_without_a_program_is_refused() {
        let error = parse(args(&["-e"])).expect_err("no program");
        assert!(error.to_string().contains("names no program"), "{error}");
        assert!(error.to_string().contains("-e PROGRAM"), "{error}");
    }

    #[test]
    fn explorer_s_quote_after_a_drive_s_backslash_reads_back_on_windows() {
        assert_eq!(unquoted(r#"C:""#.into(), true), PathBuf::from(r"C:\"));
        assert_eq!(unquoted(r"D:\proj".into(), true), PathBuf::from(r"D:\proj"));
        assert_eq!(unquoted(r#"odd""#.into(), false), PathBuf::from(r#"odd""#));
    }
}
