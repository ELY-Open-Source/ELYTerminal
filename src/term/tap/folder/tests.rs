use super::*;

#[test]
fn conemu_s_folder_names_a_windows_path_as_it_stands() {
    let raw = |body: &str| reported_on(body.as_bytes(), true);
    let odd = PathBuf::from(r"C:\work\release%20one");
    assert_eq!(raw(r"9;9;C:\work\release%20one"), Some(odd));
    assert_eq!(
        raw(r#"9;9;"C:\Program Files""#),
        Some(r"C:\Program Files".into())
    );
    assert_eq!(
        raw(r"9;9;\\server\share\x"),
        Some(r"\\server\share\x".into())
    );
    assert_eq!(raw("9;9;relative"), None);
    assert_eq!(
        reported_on(br"9;9;C:\x", false),
        None,
        "a Windows path elsewhere names nothing"
    );
    let uri = reported_on(b"7;file:///C:/a%20b", true);
    assert_eq!(uri, Some(r"C:/a b".into()), "OSC 7 decodes as before");
}

#[test]
fn another_host_s_folder_comes_with_its_host_and_this_host_s_without() {
    let mut last = None;
    let folder = |event: Option<PtyEvent>| match event {
        Some(PtyEvent::Folder(folder, host)) => Some((folder, host)),
        _ => None,
    };
    let there = folder(event(b"7;file://far.example/home/me/a%20b", &mut last, 1));
    assert_eq!(
        there,
        Some(("/home/me/a b".into(), Some("far.example".into())))
    );
    assert_eq!(last, None, "a remote folder keeps no local one");
    let here = folder(event(b"7;file://localhost/tmp", &mut last, 1));
    if cfg!(windows) {
        assert_eq!(here, None, "/tmp names no Windows folder");
    } else {
        assert_eq!(here, Some(("/tmp".into(), None)));
        assert_eq!(last, Some("/tmp".into()));
    }
    assert!(folder(event(b"7;file://far.example/", &mut last, 1)).is_some());
    assert!(folder(event(b"0;title", &mut last, 1)).is_none());
}
