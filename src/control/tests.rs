use super::*;

#[test]
fn tokens_match_only_when_every_byte_does() {
    assert!(same(b"abc", b"abc"));
    assert!(!same(b"abc", b"abd"));
    assert!(!same(b"abc", b"ab"));
    assert!(!same(b"", b"a"));
}

#[test]
fn a_caller_with_the_token_is_answered_and_one_without_is_refused() {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a port");
    let port = listener.local_addr().expect("its address").port();
    let open = Arc::new(AtomicBool::new(true));
    let (asks, mut asked) = unbounded::<Ask>();
    let serving = open.clone();
    let door = thread::spawn(move || serve(listener, "secret", &serving, &asks));
    // The app's side: echo the words back.
    let app = thread::spawn(move || {
        futures::executor::block_on(async {
            while let Some(ask) = asked.next().await {
                ask.answer
                    .send(Reply::Ok(ask.words.join(" ")))
                    .expect("the caller waits");
            }
        })
    });
    let call = |token: &str, words: &[&str]| -> Reply {
        let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).expect("the door");
        let request = Request {
            token: token.into(),
            words: words.iter().map(|word| word.to_string()).collect(),
        };
        write_line(&stream, &request).expect("asked");
        read_line(&stream).expect("answered")
    };
    assert_eq!(call("secret", &["read", "7"]), Reply::Ok("read 7".into()));
    assert_eq!(
        call("guess", &["read", "7"]),
        Reply::Error("the token is not this door's".into())
    );
    // A line that is no request fails that caller alone.
    let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).expect("the door");
    (&stream).write_all(b"not json\n").expect("written");
    let mut left = String::new();
    (&stream).read_to_string(&mut left).expect("closed");
    assert_eq!(left, "", "no answer, and the door goes on");
    assert_eq!(call("secret", &["sessions"]), Reply::Ok("sessions".into()));
    open.store(false, Ordering::SeqCst);
    TcpStream::connect((Ipv4Addr::LOCALHOST, port)).expect("the wake");
    door.join().expect("the door ends");
    app.join().expect("the app ends");
}

#[test]
fn the_door_s_file_is_the_user_s_alone() {
    let dir = std::env::temp_dir().join(format!("ely-control-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join(FILE);
    write_private(&file, "1 a\n").expect("written");
    write_private(&file, "2 b\n").expect("written again over the first");
    assert_eq!(fs::read_to_string(&file).expect("read"), "2 b\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&file).expect("its mode").permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    fs::remove_dir_all(&dir).expect("removed");
}
