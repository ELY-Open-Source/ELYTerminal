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
