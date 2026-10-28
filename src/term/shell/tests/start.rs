use super::*;

#[test]
fn a_session_starts_at_home_or_in_the_folder_given() {
    let dir = Path::new("/nonexistent");
    let home = launch("/bin/sh", Start::Home, dir).expect("a launch");
    assert_eq!(home.cwd, dirs::home_dir().expect("a home"));
    let there = launch("/bin/sh", Start::In(PathBuf::from("/tmp")), dir).expect("a launch");
    assert_eq!(there.cwd, PathBuf::from("/tmp"));
    assert_eq!(there.program, home.program, "no other program for either");
}
