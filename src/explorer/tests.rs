use super::*;

/// A throwaway key, never the ones Explorer and the user's variables read.
fn scratch(name: &str) -> String {
    format!(r"Software\ELYTerminalTest\{}-{name}", std::process::id())
}

#[test]
fn the_menu_and_path_are_added_read_and_removed_under_their_keys() {
    let (classes, environment) = (scratch("menu"), scratch("environment"));
    let keys = Keys {
        classes: &classes,
        environment: &environment,
    };
    let exe = Path::new(r"C:\Program Files\ELYTerminal\elyterminal.exe");
    registry::set_kind(
        &environment,
        Some("Path"),
        r"%USERPROFILE%\bin;".as_ref(),
        REG_EXPAND_SZ,
    )
    .expect("a PATH");
    assert_eq!(read(&keys, exe).expect("a read"), Entry::Absent);
    add(&keys, exe, "Open in ELYTerminal").expect("added");
    assert_eq!(read(&keys, exe).expect("a read"), Entry::Ours);
    let label = registry::text(&format!(r"{classes}\{}", ENTRIES[1]), None).expect("a read");
    assert_eq!(label, Some("Open in ELYTerminal".into()));
    let (path, kind) = user_path(&keys).expect("a PATH");
    assert_eq!(path, r"%USERPROFILE%\bin;;C:\Program Files\ELYTerminal");
    assert_eq!(kind, REG_EXPAND_SZ, "the variables stay unexpanded");
    add(&keys, exe, "Open in ELYTerminal").expect("added again");
    assert_eq!(user_path(&keys).expect("a PATH").0, path, "added once");
    let moved = Path::new(r"D:\Tools\elyterminal.exe");
    assert_eq!(
        read(&keys, moved).expect("a read"),
        Entry::Other(r#""C:\Program Files\ELYTerminal\elyterminal.exe" "%V""#.into())
    );
    remove(&keys, exe).expect("removed");
    assert_eq!(read(&keys, exe).expect("a read"), Entry::Absent);
    assert_eq!(user_path(&keys).expect("a PATH").0, r"%USERPROFILE%\bin;");
    add(&keys, exe, "Open in ELYTerminal").expect("added once more");
    remove(
        &keys,
        Path::new(r"C:\Program Files\ELYTerminal\\elyterminal.exe"),
    )
    .expect("removed");
    remove(&keys, exe).expect("removing nothing is fine");
    registry::delete_tree(&classes).expect("the scratch key goes");
    registry::delete_tree(&environment).expect("the scratch key goes");
}

#[test]
fn an_expandable_entry_naming_the_folder_is_the_folder() {
    let (classes, environment) = (scratch("menu-x"), scratch("environment-x"));
    let keys = Keys {
        classes: &classes,
        environment: &environment,
    };
    let root = crate::term::env::fresh().expect("variables");
    let root = root.get("SystemRoot").expect("SystemRoot").to_string();
    let exe = Path::new(&root).join(r"ELYTerminal\elyterminal.exe");
    let written = r"C:\a;%SYSTEMROOT%\ELYTerminal\;;";
    registry::set_kind(&environment, Some("Path"), written.as_ref(), REG_EXPAND_SZ)
        .expect("a PATH");
    add(&keys, &exe, "Open in ELYTerminal").expect("added");
    assert_eq!(
        user_path(&keys).expect("a PATH").0,
        written,
        "no second entry"
    );
    assert_eq!(read(&keys, &exe).expect("a read"), Entry::Ours);
    remove(&keys, &exe).expect("removed");
    assert_eq!(
        user_path(&keys).expect("a PATH").0,
        r"C:\a;;",
        "the empty entries stay"
    );
    registry::delete_tree(&classes).expect("the scratch key goes");
    registry::delete_tree(&environment).expect("the scratch key goes");
}
