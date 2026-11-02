use super::*;

#[test]
fn a_session_starts_at_home_or_in_the_folder_given() {
    let dir = Path::new("/nonexistent");
    let home = launch("/bin/sh", &[], Start::Home, dir).expect("a launch");
    assert_eq!(home.cwd, dirs::home_dir().expect("a home"));
    let there = launch("/bin/sh", &[], Start::In(PathBuf::from("/tmp")), dir).expect("a launch");
    assert_eq!(there.cwd, PathBuf::from("/tmp"));
    assert_eq!(there.program, home.program, "no other program for either");
}

#[test]
fn a_shell_named_bare_runs_from_its_full_path() {
    let started =
        launch("sh", &[], PathBuf::from("/"), Path::new("/nonexistent")).expect("a launch");
    let (program, _) = started.program.expect("a program");
    assert_eq!(program, crate::settings::program_path("sh").expect("sh"));
    assert!(Path::new(&program).is_absolute(), "{program}");
    assert!(launch("no-such-shell", &[], PathBuf::from("/"), Path::new("/")).is_err());
}

#[test]
fn a_profile_s_arguments_run_as_written_without_integration() {
    let dir = Path::new("/nonexistent");
    let args = ["-l".to_string()];
    let bash = launch("bash", &args, Start::Home, dir).expect("a launch");
    let (program, given) = bash.program.expect("bash");
    assert_eq!(
        program,
        crate::settings::program_path("bash").expect("bash")
    );
    assert_eq!(given, args, "no --rcfile beside them");
    assert!(!bash.reports && bash.env.is_empty());
    let wsl = ["-d".to_string(), "Ubuntu".to_string()];
    let sh = launch("/bin/sh", &wsl, Start::Home, dir).expect("a launch");
    assert_eq!(sh.program.expect("sh").1, wsl, "-d is wsl's alone");
    assert!(launch("", &args, Start::Home, dir).is_err());
}
