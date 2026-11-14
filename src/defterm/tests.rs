use super::*;

/// Throwaway keys, never the ones Windows reads: the root, its classes and its startup key.
fn scratch(name: &str) -> (String, String, String) {
    let root = format!(
        r"Software\ELYTerminalTest\{}-defterm-{name}",
        std::process::id()
    );
    (format!(r"{root}\Classes"), format!(r"{root}\Startup"), root)
}

#[test]
fn the_default_is_made_read_and_restored_under_its_keys() {
    let (classes, startup, root) = scratch("made");
    let keys = Keys {
        classes: &classes,
        startup: &startup,
    };
    crate::term::beside_test().expect("ConPTY beside the test");
    let exe = std::env::current_exe().expect("this test");
    assert_eq!(read(&keys, &exe).expect("a read"), Default::Elsewhere);
    make(&keys, &exe).expect("made");
    assert_eq!(read(&keys, &exe).expect("a read"), Default::Ours);
    let stub = registry::text(
        &format!(r"{classes}\Interface\{}\ProxyStubClsid32", INTERFACES[1]),
        None,
    );
    assert_eq!(stub.expect("a read"), Some(PROXY.into()));
    let moved = Path::new(r"D:\Tools\elyterminal.exe");
    assert_eq!(
        read(&keys, moved).expect("a read"),
        Default::Copy(command(&exe).to_string_lossy().into())
    );
    restore(&keys).expect("restored");
    assert_eq!(read(&keys, &exe).expect("a read"), Default::Elsewhere);
    let console = registry::text(&startup, Some("DelegationConsole")).expect("a read");
    assert_eq!(console, None);
    registry::delete_tree(&root).expect("the scratch key goes");
}

#[test]
fn restore_leaves_another_terminal_chosen() {
    let (classes, startup, root) = scratch("other");
    let keys = Keys {
        classes: &classes,
        startup: &startup,
    };
    let other = "{E12CFF52-A866-4C77-9A90-F570A7AA2C6B}";
    set(&startup, Some("DelegationTerminal"), other.as_ref()).expect("set");
    restore(&keys).expect("restored");
    let kept = registry::text(&startup, Some("DelegationTerminal")).expect("a read");
    assert_eq!(kept, Some(other.into()));
    registry::delete_tree(&root).expect("the scratch key goes");
}

#[test]
fn a_program_without_openconsole_beside_it_is_refused() {
    let (classes, startup, _) = scratch("refused");
    let keys = Keys {
        classes: &classes,
        startup: &startup,
    };
    let error = make(&keys, Path::new(r"C:\nowhere\elyterminal.exe")).expect_err("no ConPTY");
    assert!(error.to_string().contains("OpenConsole.exe"), "{error}");
}
