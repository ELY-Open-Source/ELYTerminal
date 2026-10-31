//! The folder the command line names for the first session.

use std::{ffi::OsString, path::PathBuf};

use anyhow::Context as _;

const USAGE: &str = "usage: elyterminal [FOLDER | --working-directory FOLDER | --startingDirectory FOLDER | -d FOLDER]";

const FOLDER_OPTIONS: [&str; 3] = ["--working-directory", "--startingDirectory", "-d"];

/// The explicit folder, else Windows Terminal's inherited directory.
pub fn folder(args: impl Iterator<Item = OsString>) -> anyhow::Result<Option<PathBuf>> {
    opening_folder(
        args,
        cfg!(windows) && std::env::var_os("WT_SESSION").is_some(),
    )
}

fn opening_folder(
    mut args: impl Iterator<Item = OsString>,
    windows_terminal: bool,
) -> anyhow::Result<Option<PathBuf>> {
    let Some(arg) = args.next() else {
        if windows_terminal {
            let path = std::env::current_dir()
                .context("app: cannot read Windows Terminal's starting folder")?;
            log::info!(
                "app: the first session starts in {}, from Windows Terminal",
                path.display()
            );
            return Ok(Some(path));
        }
        return Ok(None);
    };
    let arg = if FOLDER_OPTIONS.iter().any(|option| arg == *option) {
        args.next()
            .with_context(|| format!("app: the folder option has no value; {USAGE}"))?
    } else {
        arg
    };
    anyhow::ensure!(
        args.next().is_none(),
        "app: more than one argument; {USAGE}"
    );
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
    Ok(Some(path))
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

    fn folder(args: impl Iterator<Item = OsString>) -> anyhow::Result<Option<PathBuf>> {
        opening_folder(args, false)
    }

    #[test]
    fn windows_terminal_passes_its_directory_without_arguments() {
        assert_eq!(
            opening_folder(args(&[]), true).expect("inherited folder"),
            Some(std::env::current_dir().expect("current folder"))
        );
        let here = std::env::temp_dir();
        let here = here.to_str().expect("text");
        for arguments in [vec![here], vec!["-d", here]] {
            assert_eq!(
                opening_folder(args(&arguments), true).expect("explicit folder"),
                Some(std::path::absolute(here).expect("full path"))
            );
        }
        assert!(opening_folder(args(&["-d"]), true).is_err());
    }

    fn args(list: &[&str]) -> impl Iterator<Item = OsString> {
        list.iter()
            .map(OsString::from)
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[test]
    fn no_argument_names_no_folder_and_one_folder_names_itself() {
        assert_eq!(folder(args(&[])).expect("no argument"), None);
        let here = std::env::temp_dir();
        let named = folder(args(&[here.to_str().expect("text")])).expect("a folder");
        assert_eq!(
            named,
            Some(std::path::absolute(&here).expect("a full path"))
        );
        let current = folder(args(&["."])).expect("the current folder");
        assert_eq!(current, Some(std::env::current_dir().expect("a folder")));
    }

    #[test]
    fn linux_and_windows_directory_options_name_the_folder() {
        let here = std::env::temp_dir();
        let here = here.to_str().expect("text");
        for option in FOLDER_OPTIONS {
            let named = folder(args(&[option, here])).expect(option);
            assert_eq!(
                named,
                Some(std::path::absolute(here).expect("a full path")),
                "{option}"
            );
        }
    }

    #[test]
    fn two_arguments_or_a_file_are_refused() {
        let here = std::env::temp_dir();
        let here = here.to_str().expect("text");
        let error = folder(args(&[here, here])).expect_err("two");
        assert!(
            error.to_string().contains("more than one argument"),
            "{error}"
        );
        let file = std::env::current_exe().expect("this test");
        let error = folder(args(&[file.to_str().expect("text")])).expect_err("a file");
        assert!(error.to_string().contains("names no folder"), "{error}");

        let error = folder(args(&["-d"])).expect_err("no value");
        assert!(error.to_string().contains("has no value"), "{error}");
        let error = folder(args(&["-d", here, here])).expect_err("two folders");
        assert!(
            error.to_string().contains("more than one argument"),
            "{error}"
        );
    }

    #[test]
    fn explorer_s_quote_after_a_drive_s_backslash_reads_back_on_windows() {
        assert_eq!(unquoted(r#"C:""#.into(), true), PathBuf::from(r"C:\"));
        assert_eq!(unquoted(r"D:\proj".into(), true), PathBuf::from(r"D:\proj"));
        assert_eq!(unquoted(r#"odd""#.into(), false), PathBuf::from(r#"odd""#));
    }
}
