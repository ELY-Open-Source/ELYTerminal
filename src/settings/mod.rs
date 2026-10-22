mod store;
pub mod window;

use std::{env, ffi::OsStr, ops::RangeInclusive, path::Path};

use anyhow::Context as _;
use serde::{Deserialize, Serialize};

use crate::themes;

pub use store::{SettingsStore, load, save};

/// Where a new session starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartIn {
    #[default]
    Home,
    Session,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorStyle {
    #[default]
    Block,
    Bar,
    Underline,
    Hollow,
}

/// Which Option keys type Alt on macOS.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptionAsAlt {
    #[default]
    Off,
    Left,
    Right,
    Both,
}

/// What the right panel shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelTab {
    #[default]
    Git,
    Files,
}

/// Everything the settings window edits. A field the file leaves out takes its default.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    /// A program path; empty runs the login shell.
    pub shell: String,
    pub start_in: StartIn,
    pub theme: String,
    /// The theme while the system is dark; none keeps `theme`.
    pub dark_theme: Option<String>,
    pub font_family: String,
    /// Families tried in order for a character Font family lacks.
    pub font_fallbacks: Vec<String>,
    pub font_size: f32,
    pub line_height: f32,
    pub ligatures: bool,
    pub cursor_style: CursorStyle,
    pub cursor_blink: bool,
    pub background_opacity: f32,
    pub background_blur: bool,
    pub padding_x: f32,
    pub padding_y: f32,
    pub scrollback: u32,
    pub option_as_alt: OptionAsAlt,
    /// Option and Command edit text by word and line, as macOS's text does.
    pub natural_text_editing: bool,
    pub copy_on_select: bool,
    /// Ely peeks over the main window's top edge.
    pub ely: bool,
    /// How far in from the pane's left edge she stands, in points, as a drag left her.
    pub ely_left: f32,
    /// A check for a newer release at start and once a day.
    pub check_updates: bool,
    pub show_sessions: bool,
    /// A drag on a session's row moves it to another place in the list.
    pub drag_sessions: bool,
    /// A right press on a session's row opens a menu that duplicates or closes it.
    pub session_menu: bool,
    pub show_panel: bool,
    pub panel: PanelTab,
    /// The sessions and right columns' widths, in pixels, as their borders were dragged.
    pub sessions_width: f32,
    pub panel_width: f32,
    /// The main window's size, unmaximized, in pixels.
    pub window_width: f32,
    pub window_height: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            shell: String::new(),
            start_in: StartIn::Home,
            theme: themes::default_id().into(),
            dark_theme: None,
            font_family: "JetBrains Mono".into(),
            font_fallbacks: Vec::new(),
            font_size: 13.0,
            line_height: 1.2,
            ligatures: true,
            cursor_style: CursorStyle::Block,
            cursor_blink: true,
            background_opacity: 0.88,
            background_blur: true,
            padding_x: 10.0,
            padding_y: 8.0,
            scrollback: 10_000,
            option_as_alt: OptionAsAlt::Off,
            natural_text_editing: false,
            copy_on_select: false,
            ely: true,
            ely_left: 140.0,
            check_updates: true,
            show_sessions: true,
            drag_sessions: true,
            session_menu: true,
            show_panel: true,
            panel: PanelTab::Git,
            sessions_width: 240.0,
            panel_width: 240.0,
            window_width: 1200.0,
            window_height: 760.0,
        }
    }
}

pub const FONT_SIZE: RangeInclusive<f32> = 6.0..=72.0;
pub const LINE_HEIGHT: RangeInclusive<f32> = 1.0..=2.0;
pub const OPACITY: RangeInclusive<f32> = 0.3..=1.0;
pub const PADDING: RangeInclusive<f32> = 0.0..=64.0;
pub const SCROLLBACK: RangeInclusive<u32> = 0..=100_000;
pub const COLUMN: RangeInclusive<f32> = 160.0..=960.0;
pub const WINDOW: RangeInclusive<f32> = 100.0..=16384.0;
pub const ELY_LEFT: RangeInclusive<f32> = 0.0..=16384.0;

impl Settings {
    /// Every value inside its range, every theme built in; the error names the field.
    pub fn validate(&self) -> anyhow::Result<()> {
        within("font_size", self.font_size, &FONT_SIZE)?;
        within("line_height", self.line_height, &LINE_HEIGHT)?;
        within("background_opacity", self.background_opacity, &OPACITY)?;
        within("padding_x", self.padding_x, &PADDING)?;
        within("padding_y", self.padding_y, &PADDING)?;
        within("scrollback", self.scrollback, &SCROLLBACK)?;
        within("sessions_width", self.sessions_width, &COLUMN)?;
        within("panel_width", self.panel_width, &COLUMN)?;
        within("window_width", self.window_width, &WINDOW)?;
        within("window_height", self.window_height, &WINDOW)?;
        within("ely_left", self.ely_left, &ELY_LEFT)?;
        for (field, id) in [
            ("theme", Some(&self.theme)),
            ("dark_theme", self.dark_theme.as_ref()),
        ] {
            if let Some(id) = id {
                anyhow::ensure!(themes::known(id), "{field} is {id:?}, no built-in theme");
            }
        }
        anyhow::ensure!(!self.font_family.is_empty(), "font_family is empty");
        anyhow::ensure!(
            !self.font_fallbacks.iter().any(String::is_empty),
            "font_fallbacks holds an empty family"
        );
        if !self.shell.is_empty() {
            program_path(&self.shell)?;
        }
        Ok(())
    }
}

/// The program `shell` names, as a session starts it: a file at a full path, or the first in `PATH`'s full folders.
pub fn program_path(shell: &str) -> anyhow::Result<String> {
    program_path_in(shell, &env::var_os("PATH").unwrap_or_default())
}

/// The program `shell` names, in `folders` as `PATH` lists them.
pub fn program_path_in(shell: &str, folders: &impl AsRef<OsStr>) -> anyhow::Result<String> {
    find(shell, folders.as_ref(), cfg!(windows))
}

/// `shell`'s program, or the first one named so in `folders`, listed as `PATH` lists them; with `exe`, as on Windows, a name without an extension means name.exe.
fn find(shell: &str, folders: &OsStr, exe: bool) -> anyhow::Result<String> {
    let path = Path::new(shell);
    // A relative path, or a relative folder, would hang on each session's own folder.
    let found = if path.is_absolute() {
        program(path).then(|| path.to_path_buf())
    } else if path.components().count() == 1 {
        let name = if exe && !shell.contains('.') {
            format!("{shell}.exe")
        } else {
            shell.to_string()
        };
        env::split_paths(folders)
            .filter(|folder| folder.is_absolute())
            .map(|folder| folder.join(&name))
            .find(|file| program(file))
    } else {
        None
    };
    let found = found.with_context(|| format!("shell is {shell:?}, which names no program"))?;
    let text = found.to_str().with_context(|| {
        format!(
            "shell is {shell:?}, found at {}, which is not UTF-8",
            found.display()
        )
    })?;
    Ok(text.to_string())
}

/// Whether `path` is a file the running user may run, as the system answers, ACLs too.
#[cfg(unix)]
fn program(path: &Path) -> bool {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let Ok(name) = CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };
    path.is_file() && unsafe { libc::access(name.as_ptr(), libc::X_OK) } == 0
}

#[cfg(not(unix))]
fn program(path: &Path) -> bool {
    path.is_file()
}

fn within<T: PartialOrd + std::fmt::Display>(
    field: &str,
    value: T,
    range: &RangeInclusive<T>,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        range.contains(&value),
        "{field} is {value}, outside {} to {}",
        range.start(),
        range.end()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
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
}
