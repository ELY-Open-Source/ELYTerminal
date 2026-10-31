//! The folder the command line names for the first session.

use std::{ffi::OsString, path::PathBuf};

use anyhow::Context as _;

const USAGE: &str = "usage: elyterminal [FOLDER]";

/// The one folder `args` names; anything else is an error.
pub fn folder(mut args: impl Iterator<Item = OsString>) -> anyhow::Result<Option<PathBuf>> {
    let Some(arg) = args.next() else {
        return Ok(None);
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
    }

    #[test]
    fn explorer_s_quote_after_a_drive_s_backslash_reads_back_on_windows() {
        assert_eq!(unquoted(r#"C:""#.into(), true), PathBuf::from(r"C:\"));
        assert_eq!(unquoted(r"D:\proj".into(), true), PathBuf::from(r"D:\proj"));
        assert_eq!(unquoted(r#"odd""#.into(), false), PathBuf::from(r#"odd""#));
    }
}
