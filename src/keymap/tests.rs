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
    assert_eq!(full.keys(&Shortcuts::new(), &commands()), [own]);
}

#[test]
fn a_chosen_key_replaces_a_command_s_own() {
    let shortcuts = chosen(&[("new_session", &["ctrl-alt-n"]), ("quit", &[])]);
    check(&shortcuts).expect("a free key and an unbound command");
    let commands = commands();
    let keys = |name: &str| {
        let command = commands.iter().find(|command| command.name == name);
        command.expect("a command").keys(&shortcuts, &commands)
    };
    assert_eq!(keys("new_session"), ["ctrl-alt-n"]);
    assert!(keys("quit").is_empty());
    assert!(!keys("copy").is_empty(), "the rest keep their own");
}

#[test]
fn a_key_chosen_twice_in_one_window_is_refused() {
    check(&chosen(&[
        ("clear", &["shift-ctrl-alt-x"]),
        ("copy", &["alt-ctrl-shift-x"]),
    ]))
    .expect_err("one key spelled twice");
    // Settings and the main window never hold focus together.
    check(&chosen(&[
        ("close_settings", &["ctrl-alt-q"]),
        ("clear", &["ctrl-alt-q"]),
    ]))
    .expect("apart");
    // An app-wide key meets both.
    check(&chosen(&[
        ("quit", &["ctrl-alt-q"]),
        ("close_settings", &["ctrl-alt-q"]),
    ]))
    .expect_err("quit runs everywhere");
}

#[test]
fn a_command_s_own_key_yields_to_one_the_settings_give_another() {
    let commands = commands();
    let own = |name: &str| {
        let command = commands.iter().find(|command| command.name == name);
        command
            .expect("a command")
            .keys(&Shortcuts::new(), &commands)[0]
            .clone()
    };
    // A 0.2.4 file may give a key a newer release takes, as F11 or a pane's.
    let (full, split) = (own("full_screen"), own("split_right"));
    let shortcuts = chosen(&[("clear", &[full.as_str()]), ("copy", &[split.as_str()])]);
    check(&shortcuts).expect("the settings' keys win");
    let keys = |name: &str| {
        let command = commands.iter().find(|command| command.name == name);
        command.expect("a command").keys(&shortcuts, &commands)
    };
    assert_eq!(keys("clear"), [full]);
    assert!(keys("full_screen").is_empty(), "{:?}", keys("full_screen"));
    assert!(!keys("split_right").contains(&split));
    // Another window's own key stays.
    let close = own("close_session");
    let apart = chosen(&[("close_settings", &[close.as_str()])]);
    let command = commands
        .iter()
        .find(|command| command.name == "close_session");
    assert_eq!(
        command.expect("close_session").keys(&apart, &commands),
        [close]
    );
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
