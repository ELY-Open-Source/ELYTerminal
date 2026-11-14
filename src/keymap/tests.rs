use super::*;

fn chosen(pairs: &[(&str, &[&str])]) -> Shortcuts {
    pairs
        .iter()
        .map(|(name, keys)| {
            let keys = keys.iter().map(|key| key.to_string()).collect();
            (name.to_string(), keys)
        })
        .collect()
}

#[test]
fn the_commands_own_keys_clash_nowhere() {
    check(&Shortcuts::new()).expect("the table holds no clash");
    let commands = commands();
    let mut names: Vec<_> = commands.iter().map(|command| &command.name).collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), commands.len(), "each name once");
}

#[test]
fn full_screen_is_ctrl_cmd_f_on_macos_and_f11_elsewhere() {
    let full = commands()
        .into_iter()
        .find(|command| command.name == "full_screen")
        .expect("full_screen");
    let own = if cfg!(target_os = "macos") {
        "ctrl-cmd-f"
    } else {
        "f11"
    };
    assert_eq!(full.keys(&Shortcuts::new()), [own]);
}

#[test]
fn a_chosen_key_replaces_a_command_s_own() {
    let shortcuts = chosen(&[("new_session", &["ctrl-alt-n"]), ("quit", &[])]);
    check(&shortcuts).expect("a free key and an unbound command");
    let commands = commands();
    let keys = |name: &str| {
        let command = commands.iter().find(|command| command.name == name);
        command.expect("a command").keys(&shortcuts)
    };
    assert_eq!(keys("new_session"), ["ctrl-alt-n"]);
    assert!(keys("quit").is_empty());
    assert!(!keys("copy").is_empty(), "the rest keep their own");
}

#[test]
fn a_key_two_commands_of_one_window_hold_is_refused() {
    let find = commands()
        .into_iter()
        .find(|command| command.name == "find")
        .expect("find");
    let taken = find.keys(&Shortcuts::new())[0].clone();
    let clash = check(&chosen(&[("clear", &[taken.as_str()])])).expect_err("find holds it");
    assert!(format!("{clash:#}").contains("find"), "{clash:#}");
    // The same key in another order of modifiers is the same key.
    check(&chosen(&[
        ("clear", &["shift-ctrl-alt-x"]),
        ("copy", &["alt-ctrl-shift-x"]),
    ]))
    .expect_err("one key spelled twice");
    // Settings and the main window never hold focus together.
    let close = commands()
        .into_iter()
        .find(|command| command.name == "close_session")
        .expect("close_session");
    let shared = close.keys(&Shortcuts::new())[0].clone();
    check(&chosen(&[("close_settings", &[shared.as_str()])])).expect("apart");
    // An app-wide key meets both.
    check(&chosen(&[("quit", &[shared.as_str()])])).expect_err("quit runs everywhere");
}

#[test]
fn what_is_no_command_or_no_key_is_refused() {
    assert!(check(&chosen(&[("fly", &["ctrl-alt-f"])])).is_err());
    for key in [
        "",
        "ctrl-",
        "ctrl-shift",
        "ctrl-a b",
        "hyper-c",
        "f6",
        "shift-tab",
    ] {
        assert!(check(&chosen(&[("copy", &[key])])).is_err(), "{key:?}");
    }
}
