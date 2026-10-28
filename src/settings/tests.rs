use super::*;

#[test]
fn defaults_are_valid() {
    Settings::default().validate().expect("defaults pass");
}

#[test]
fn a_file_without_the_session_switches_leaves_them_on() {
    let settings: Settings = serde_json::from_str("{}").expect("every field has a default");
    assert!(settings.drag_sessions, "a drag reorders a row by default");
    assert!(
        settings.session_menu,
        "a right press opens the menu by default"
    );
}

#[test]
fn a_value_out_of_range_names_its_field() {
    let settings = Settings {
        line_height: 2.5,
        ..Settings::default()
    };
    let error = settings.validate().expect_err("2.5 is too tall");
    assert_eq!(error.to_string(), "line_height is 2.5, outside 1 to 2");
    let settings = Settings {
        scrollback: 100_001,
        ..Settings::default()
    };
    let error = settings.validate().expect_err("too many lines");
    assert_eq!(
        error.to_string(),
        "scrollback is 100001, outside 0 to 100000"
    );
    let settings = Settings {
        window_width: 50.0,
        ..Settings::default()
    };
    let error = settings.validate().expect_err("too narrow");
    assert_eq!(
        error.to_string(),
        "window_width is 50, outside 100 to 16384"
    );
    let settings = Settings {
        panel_width: 1000.0,
        ..Settings::default()
    };
    let error = settings.validate().expect_err("too wide");
    assert_eq!(error.to_string(), "panel_width is 1000, outside 160 to 960");
}

#[test]
fn fallback_fonts_are_none_unless_named_and_none_is_empty() {
    let older: Settings = serde_json::from_str(r#"{"font_family":"Menlo"}"#).expect("parses");
    assert!(older.font_fallbacks.is_empty(), "a file from before");
    let named: Settings =
        serde_json::from_str(r#"{"font_fallbacks":["Kaiti SC","Noto Sans CJK SC"]}"#)
            .expect("parses");
    assert_eq!(named.font_fallbacks, ["Kaiti SC", "Noto Sans CJK SC"]);
    named.validate().expect("named families pass");
    let empty = Settings {
        font_fallbacks: vec!["Kaiti SC".into(), String::new()],
        ..Settings::default()
    };
    let error = empty.validate().expect_err("an empty family");
    assert_eq!(error.to_string(), "font_fallbacks holds an empty family");
}

#[test]
fn a_shell_names_a_program_or_none() {
    let shell = |shell: &str| Settings {
        shell: shell.into(),
        ..Settings::default()
    };
    assert!(shell("").validate().is_ok(), "the login shell");
    let named = if cfg!(windows) { "cmd" } else { "sh" };
    assert!(shell(named).validate().is_ok(), "a name on PATH");
    let error = shell("/no/such/shell").validate().expect_err("no file");
    let said = "shell is \"/no/such/shell\", which names no program";
    assert_eq!(error.to_string(), said);
    assert!(shell("bin/sh").validate().is_err(), "a relative path");
    #[cfg(unix)]
    {
        let sh = find("sh", OsStr::new("/bin"), false).expect("sh");
        assert_eq!(sh, "/bin/sh");
        let root = OsStr::new("/");
        assert!(
            find("bin/sh", root, false).is_err(),
            "a relative path, though / holds bin/sh"
        );
        assert!(shell("/bin/sh").validate().is_ok());
        assert!(shell("/bin").validate().is_err(), "a folder");
        assert!(
            shell("/etc/hosts").validate().is_err(),
            "a file that does not run"
        );
    }
}

#[cfg(unix)]
#[test]
fn a_shell_runs_for_this_user_from_a_full_folder() {
    use std::{fs, os::unix::fs::PermissionsExt};
    let folder = std::env::temp_dir().join(format!("elyterminal-shell-{}", std::process::id()));
    fs::create_dir_all(&folder).expect("a test folder");
    let file = folder.join("review-shell");
    fs::write(&file, "#!/bin/sh\n").expect("a script");
    // Root may run what any execute bit allows.
    let root = unsafe { libc::geteuid() } == 0;
    for (mode, runs) in [(0o644, false), (0o641, root), (0o755, true)] {
        fs::set_permissions(&file, fs::Permissions::from_mode(mode)).expect("a mode");
        assert_eq!(program(&file), runs, "mode {mode:o}");
    }
    let found = find("review-shell", folder.as_os_str(), false).expect("a program");
    assert_eq!(Path::new(&found), file);
}

#[test]
fn exe_joins_only_a_name_without_an_extension() {
    use std::fs;
    let folder = env::temp_dir().join(format!("elyterminal-exe-{}", std::process::id()));
    fs::create_dir_all(&folder).expect("a test folder");
    let file = folder.join("review.exe");
    fs::write(&file, "").expect("a program");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("755");
    }
    let folders = folder.as_os_str();
    for name in ["review", "review.exe"] {
        let found = find(name, folders, true).expect("review.exe");
        assert_eq!(Path::new(&found), file, "{name}");
    }
    assert!(
        find("review.com", folders, true).is_err(),
        "its own extension"
    );
    assert!(
        find("review", folders, false).is_err(),
        "no .exe off Windows"
    );
    fs::remove_dir_all(&folder).expect("cleanup");
}

#[cfg(target_os = "linux")]
#[test]
fn a_program_at_a_path_not_in_utf_8_counts_for_none() {
    use std::{
        ffi::OsString,
        fs,
        os::unix::{ffi::OsStringExt, fs::PermissionsExt},
        path::PathBuf,
    };
    let folder = env::temp_dir().join(format!("elyterminal-bytes-{}-", std::process::id()));
    let mut name = folder.into_os_string().into_vec();
    name.push(0xff);
    let folder = PathBuf::from(OsString::from_vec(name));
    fs::create_dir_all(&folder).expect("a folder named in raw bytes");
    let file = folder.join("review-shell");
    fs::write(&file, "#!/bin/sh\n").expect("a script");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("755");
    assert!(program(&file), "the file runs");
    let error = find("review-shell", folder.as_os_str(), false).expect_err("not UTF-8");
    assert!(error.to_string().ends_with("which is not UTF-8"), "{error}");
    fs::remove_dir_all(&folder).expect("cleanup");
}

#[test]
fn a_relative_folder_counts_for_none() {
    // Tests run in the crate, whose scripts folder holds a program.
    let scripts = OsStr::new("scripts");
    assert!(program(Path::new("scripts/check.sh")));
    assert!(find("check.sh", scripts, false).is_err());
}

#[test]
fn a_nan_is_out_of_range() {
    let settings = Settings {
        font_size: f32::NAN,
        ..Settings::default()
    };
    assert!(settings.validate().is_err());
}

#[test]
fn an_unknown_theme_fails() {
    let settings = Settings {
        dark_theme: Some(String::new()),
        ..Settings::default()
    };
    assert_eq!(
        settings.validate().expect_err("empty").to_string(),
        "dark_theme is \"\", no built-in theme"
    );
    let settings = Settings {
        theme: "mocha".into(),
        ..Settings::default()
    };
    assert_eq!(
        settings.validate().expect_err("unknown").to_string(),
        "theme is \"mocha\", no built-in theme"
    );
}

#[test]
fn start_in_a_folder_needs_one_that_exists_at_a_full_path() {
    let mut settings: Settings =
        serde_json::from_str(r#"{"start_in": "folder"}"#).expect("a known choice");
    assert!(settings.validate().is_err(), "Folder with none named");
    let gone = env::temp_dir().join(format!("elyterminal-gone-{}", std::process::id()));
    for folder in ["relative/folder".into(), gone.display().to_string()] {
        settings.start_folder = folder;
        assert!(settings.validate().is_err(), "{:?}", settings.start_folder);
    }
    settings.start_folder = env::temp_dir().display().to_string();
    settings.validate().expect("a folder that exists");
    settings.start_in = StartIn::Home;
    settings.start_folder = gone.display().to_string();
    assert!(
        settings.validate().is_err(),
        "a folder kept for later is checked too"
    );
    settings.start_folder.clear();
    settings.validate().expect("home, no folder");
}
