use super::*;

/// A root that makes a path absolute here.
const ROOT: &str = if cfg!(windows) { "C:/" } else { "/" };

fn args(words: &str) -> Vec<String> {
    words.split(' ').map(str::to_owned).collect()
}

fn env(pairs: &[(&str, &str)]) -> HashMap<String, OsString> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), OsString::from(value)))
        .collect()
}

fn found(words: &str, env: &HashMap<String, OsString>) -> PathBuf {
    follows(&args(words), env)
        .expect("a local client")
        .expect("herdr's client")
}

#[test]
fn a_client_finds_its_server_as_herdr_does() {
    let home = env(&[("HOME", "/h")]);
    let default = PathBuf::from("/h/.config/herdr/herdr.sock");
    let work = PathBuf::from("/h/.config/herdr/sessions/work/herdr.sock");
    assert_eq!(found("herdr", &home), default);
    assert_eq!(found("herdr client", &home), default);
    assert_eq!(found("herdr --session work", &home), work);
    assert_eq!(found("herdr --session=work", &home), work);
    assert_eq!(found("herdr session attach work", &home), work);
    assert_eq!(found("herdr --session default", &home), default);
    let config = env(&[("HOME", "/h"), ("XDG_CONFIG_HOME", "/x")]);
    assert_eq!(
        found("herdr", &config),
        PathBuf::from("/x/herdr/herdr.sock")
    );
    let socket = env(&[("HOME", "/h"), ("HERDR_SOCKET_PATH", "/s.sock")]);
    assert_eq!(found("herdr", &socket), PathBuf::from("/s.sock"));
    assert_eq!(found("herdr --session work", &socket), work);
    let relative = env(&[("HOME", "/h"), ("HERDR_SOCKET_PATH", "herdr.sock")]);
    assert_eq!(found("herdr", &relative), PathBuf::from("herdr.sock"));
    let named = env(&[("HOME", "/h"), ("HERDR_SESSION", "work")]);
    assert_eq!(found("herdr", &named), work);
    assert!(follows(&args("herdr"), &env(&[])).is_err());
}

#[test]
fn only_herdr_s_terminal_client_is_followed() {
    let home = env(&[("HOME", "/h")]);
    for words in [
        "herdr pane list",
        "herdr api snapshot",
        "herdr server",
        "herdr session list",
        "herdr --version",
        "herdr --session work status",
    ] {
        assert_eq!(follows(&args(words), &home).expect(words), None, "{words}");
    }
    for words in ["herdr --remote box", "herdr --remote=box --session work"] {
        let error = follows(&args(words), &home).expect_err(words);
        assert!(error.to_string().contains("another machine"), "{error}");
    }
}

fn reply(focused_pane: &str, cwd: &str) -> String {
    format!(
        r#"{{"id":"elyterminal","result":{{"type":"session_snapshot","snapshot":{{"version":"0.9.5","protocol":22,"focused_pane_id":{focused_pane},"workspaces":[],"tabs":[],"panes":[{{"pane_id":"w1:p1","cwd":"/a","focused":false}},{{"pane_id":"w1:p2","cwd":{cwd},"focused":true}}],"layouts":[],"agents":[]}}}}}}"#
    )
}

#[test]
fn the_focused_pane_s_folder_comes_from_herdr_s_snapshot() {
    let read =
        |focused_pane: &str, cwd: &str| shows(&reply(focused_pane, cwd)).map(|(folder, _)| folder);
    let folder = format!("{ROOT}srv/b");
    assert_eq!(
        read(r#""w1:p2""#, &format!("\"{folder}\"")).expect("a folder"),
        Some(folder.clone().into())
    );
    assert_eq!(
        read("null", &format!("\"{folder}\"")).expect("no focus"),
        None
    );
    assert_eq!(read(r#""w1:p2""#, "null").expect("no folder"), None);
    let error = read(r#""w1:p9""#, r#""/b""#).expect_err("a lost pane");
    assert!(error.to_string().contains("w1:p9"), "{error}");
    let error = read(r#""w1:p2""#, r#""b""#).expect_err("a relative folder");
    assert!(error.to_string().contains("relative"), "{error}");
    let failed =
        r#"{"id":"elyterminal","error":{"code":"invalid_request","message":"no such method"}}"#;
    let error = shows(failed).expect_err("an error");
    assert_eq!(error.to_string(), "invalid_request: no such method");
    assert!(shows("").is_err());
}

#[test]
fn herdr_s_agents_whose_state_it_knows_are_records_by_pane() {
    let agent = |pane: &str, status: &str, more: &str| {
        format!(
            r#"{{"terminal_id":"t","agent_status":"{status}","workspace_id":"w1","tab_id":"w1:t1","pane_id":"{pane}","focused":false,"revision":1{more}}}"#
        )
    };
    let agents = [
        agent(
            "w1:p1",
            "working",
            r#","agent":"codex","display_agent":"Codex","name":"review","title":"codex","terminal_title_stripped":"Reading files""#,
        ),
        agent(
            "w1:p2",
            "blocked",
            r#","agent":"claude","display_agent":"Claude Code","title":"claude""#,
        ),
        agent("w1:p3", "unknown", ""),
        agent("w1:p4", "napping", ""),
        agent("w1:p5", "done", ""),
        agent("w1:p6", "idle", ""),
    ];
    let reply = reply("null", "null").replace(
        r#""agents":[]"#,
        &format!(r#""agents":[{}]"#, agents.join(",")),
    );
    let (folder, records) = shows(&reply).expect("a snapshot");
    assert_eq!(folder, None);
    let said: Vec<_> = records
        .iter()
        .map(|(id, record)| {
            (
                id.as_str(),
                record.state,
                record.app.as_deref(),
                record.title.as_deref(),
                record.msg.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        said,
        [
            (
                "herdr:w1:p1",
                State::Working,
                Some("codex"),
                Some("review"),
                Some("Reading files")
            ),
            (
                "herdr:w1:p2",
                State::Blocked,
                Some("claude"),
                Some("Claude Code"),
                Some("claude")
            ),
            ("herdr:w1:p5", State::Done, None, None, None),
            ("herdr:w1:p6", State::Idle, None, None, None),
        ]
    );
    assert!(
        records
            .iter()
            .all(|(_, record)| record.kind.is_none() && record.progress.is_none())
    );
    let older = reply.replace(&format!(r#","agents":[{}]"#, agents.join(",")), "");
    assert_eq!(shows(&older).expect("a herdr before agents").1, []);
}

#[cfg(unix)]
#[test]
fn herdr_answers_over_its_socket() {
    use std::{fs, os::unix::net::UnixListener};
    let dir = std::env::temp_dir().join(format!("elyterminal-herdr-{}", std::process::id()));
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("a stale test folder goes");
    }
    fs::create_dir_all(&dir).expect("a test folder");
    let socket = dir.join("herdr.sock");
    let listener = UnixListener::bind(&socket).expect("a socket");
    let answer = reply(r#""w1:p2""#, r#""/srv""#);
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("a client");
        let mut request = String::new();
        BufReader::new(&stream)
            .read_line(&mut request)
            .expect("a request");
        (&stream)
            .write_all(format!("{answer}\n").as_bytes())
            .expect("a reply");
        request
    });
    assert_eq!(snapshot(&socket).expect("an answer").0, Some("/srv".into()));
    assert_eq!(server.join().expect("the server").as_bytes(), SNAPSHOT);
    fs::remove_dir_all(&dir).expect("the test folder goes");
    let error = snapshot(&socket).expect_err("no server");
    assert!(format!("{error:#}").contains("asking"), "{error:#}");
    assert!(starting(&error), "{error:#}");
    assert!(!starting(&shows("").expect_err("no reply")));
}

/// macOS hides the variables of its own programs, such as `/bin/sleep`, so the test reads itself.
#[cfg(unix)]
#[test]
fn a_process_reads_with_its_path_arguments_variables_and_folder() {
    use super::unix::cwd;
    use crate::term::unix::{launch, path};
    use std::fs;
    let pid = std::process::id() as i32;
    let read = path(pid).expect("its path");
    let (args, env) = launch(pid).expect("its arguments");
    let exe = std::env::current_exe().expect("this test's path");
    assert_eq!(fs::canonicalize(read).ok(), fs::canonicalize(exe).ok());
    assert_eq!(args, std::env::args().collect::<Vec<_>>());
    assert_eq!(env.get("HOME"), std::env::var_os("HOME").as_ref());
    let folder = std::env::current_dir().expect("this test's folder");
    assert_eq!(
        fs::canonicalize(cwd(pid).expect("its folder")).ok(),
        fs::canonicalize(folder).ok()
    );
}

#[cfg(unix)]
#[test]
fn only_herdr_s_program_is_read() {
    let pid = std::process::id() as i32;
    assert_eq!(
        client((pid, None)).expect("a program that does not read"),
        None
    );
    let shell = client((pid, Some("/bin/sh".into())));
    assert_eq!(shell.expect("another program"), None);
}

#[cfg(target_os = "linux")]
#[test]
fn an_updated_herdr_still_names_itself() {
    use crate::term::unix::installed;
    let deleted = PathBuf::from("/home/u/.local/bin/herdr (deleted)");
    assert_eq!(
        installed(deleted),
        PathBuf::from("/home/u/.local/bin/herdr")
    );
    assert_eq!(
        installed("/usr/bin/herdr".into()),
        PathBuf::from("/usr/bin/herdr")
    );
}

#[cfg(target_os = "macos")]
#[test]
fn macos_lays_out_the_arguments_after_the_program_s_path() {
    use crate::term::unix::procargs;
    let mut buffer = 2i32.to_ne_bytes().to_vec();
    buffer.extend_from_slice(b"/usr/local/bin/herdr\0\0\0\0herdr\0\0HOME=/h\0X=a=b\0\0\0");
    let (args, env) = procargs(&buffer).expect("it parses");
    assert_eq!(args, ["herdr", ""]);
    assert_eq!(env.get("HOME"), Some(&OsString::from("/h")));
    assert_eq!(env.get("X"), Some(&OsString::from("a=b")));
    assert!(procargs(&[1, 0]).is_err());
}

#[cfg(windows)]
#[test]
fn a_windows_client_finds_its_server_in_app_data() {
    let roaming = PathBuf::from(r"C:\Users\u\AppData\Roaming\herdr");
    let data = env(&[
        ("APPDATA", r"C:\Users\u\AppData\Roaming"),
        ("USERPROFILE", r"C:\D"),
    ]);
    assert_eq!(found("herdr", &data), roaming.join("herdr.sock"));
    let profile = env(&[("USERPROFILE", r"C:\Users\u")]);
    assert_eq!(
        found("herdr --session work", &profile),
        roaming.join(r"sessions\work\herdr.sock")
    );
    let config = env(&[("XDG_CONFIG_HOME", r"C:\x"), ("APPDATA", r"C:\D")]);
    assert_eq!(
        found("herdr", &config),
        PathBuf::from(r"C:\x\herdr\herdr.sock")
    );
}

#[cfg(windows)]
#[test]
fn herdr_is_sought_at_and_under_the_shell_not_under_herdr() {
    use super::windows::herdrs;
    let processes = [
        (100, 50, "pwsh.exe"),
        (104, 100, "cmd.exe"),
        (108, 104, "HERDR.EXE"),
        (112, 100, "herdr.exe"),
        (116, 112, "herdr.exe"),
        (120, 100, "herdr.exe"),
        (200, 300, "a.exe"),
        (300, 200, "b.exe"),
    ]
    .map(|(pid, parent, name)| (pid, parent, name.to_owned()));
    // 116 is the server 112 started; 120 one started beside it.
    assert_eq!(herdrs(&processes, 100), [112, 120, 108]);
    assert_eq!(herdrs(&processes, 112), [112]);
    assert!(herdrs(&processes, 200).is_empty());
}

#[cfg(windows)]
#[test]
fn a_server_or_a_command_beside_herdr_s_client_is_passed_by() {
    use super::windows::pick;
    let data = env(&[("APPDATA", r"C:\D")]);
    let read = |words: &str| -> io::Result<Invocation> { Ok((args(words), data.clone())) };
    let gone = || -> io::Result<Invocation> { Err(io::ErrorKind::NotFound.into()) };
    let socket = Some(PathBuf::from(r"C:\D\herdr\herdr.sock"));
    let server = "herdr.exe server";
    let client = [(200, read(server)), (100, read("herdr.exe"))];
    assert_eq!(pick(client).expect("a client"), socket);
    let command = [(200, read(server)), (300, read("herdr.exe api snapshot"))];
    assert_eq!(pick(command).expect("no client"), None);
    let unread = [(300, gone()), (100, read("herdr.exe"))];
    assert_eq!(pick(unread).expect("a client"), socket);
    let error = pick([(300, gone()), (200, read(server))]).expect_err("an unread process");
    assert!(format!("{error:#}").contains("process 300"), "{error:#}");
}

#[cfg(windows)]
#[test]
fn a_windows_process_reads_with_its_name_arguments_and_variables() {
    use crate::term::procs::{launch, processes};
    let pid = std::process::id();
    let exe = std::env::current_exe().expect("this test's path");
    let name = exe.file_name().expect("this test's name");
    let listed = processes().expect("the process list");
    assert!(
        listed
            .iter()
            .any(|(id, _, read)| *id == pid && name.eq_ignore_ascii_case(read)),
        "process {pid} is not listed under its name"
    );
    assert!(running(pid).expect("the process list").is_empty());
    let (args, env) = launch(pid).expect("its arguments");
    assert_eq!(args, std::env::args().collect::<Vec<_>>());
    // Windows names it Path; the reader folds names to upper case.
    let path = std::env::var_os("PATH");
    assert!(env.get("PATH") == path.as_ref(), "PATH reads otherwise");
}

#[cfg(windows)]
#[test]
fn herdr_answers_over_its_pipe() {
    use std::os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    };
    use windows_sys::Win32::{
        Foundation::{ERROR_PIPE_CONNECTED, INVALID_HANDLE_VALUE},
        Storage::FileSystem::PIPE_ACCESS_DUPLEX,
        System::Pipes::{ConnectNamedPipe, CreateNamedPipeW, PIPE_TYPE_BYTE},
    };
    let socket = PathBuf::from(format!(
        r"C:\elyterminal-herdr-{}\herdr.sock",
        std::process::id()
    ));
    let name = format!(r"\\.\pipe\{}", socket.display());
    let name: Vec<u16> = OsStr::new(&name).encode_wide().chain([0]).collect();
    // SAFETY: `name` ends with a NUL; the handle is owned below.
    let pipe = unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_BYTE,
            1,
            4096,
            4096,
            0,
            std::ptr::null(),
        )
    };
    assert_ne!(pipe, INVALID_HANDLE_VALUE, "{}", io::Error::last_os_error());
    // SAFETY: the handle is new and ours.
    let pipe = unsafe { OwnedHandle::from_raw_handle(pipe) };
    let answer = reply(r#""w1:p2""#, r#""C:/srv""#);
    let server = std::thread::spawn(move || {
        // SAFETY: the pipe is open.
        let joined = unsafe { ConnectNamedPipe(pipe.as_raw_handle(), std::ptr::null_mut()) };
        let error = io::Error::last_os_error();
        assert!(
            joined != 0 || error.raw_os_error() == Some(ERROR_PIPE_CONNECTED as i32),
            "{error}"
        );
        let stream = std::fs::File::from(pipe);
        let mut request = String::new();
        BufReader::new(&stream)
            .read_line(&mut request)
            .expect("a request");
        (&stream)
            .write_all(format!("{answer}\n").as_bytes())
            .expect("a reply");
        // The pipe goes only once the client has read the reply.
        stream.sync_all().expect("the client reads the reply");
        request
    });
    assert_eq!(
        snapshot(&socket).expect("an answer").0,
        Some("C:/srv".into())
    );
    assert_eq!(server.join().expect("the server").as_bytes(), SNAPSHOT);
    let error = snapshot(&socket).expect_err("no server");
    assert!(format!("{error:#}").contains("asking"), "{error:#}");
    assert!(starting(&error), "{error:#}");
}
