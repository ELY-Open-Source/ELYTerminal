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
        read_line(&stream, PATIENCE).expect("answered")
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
    // A caller already in asks after the door shut, and runs nothing.
    let late = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).expect("the door");
    thread::sleep(Duration::from_millis(200));
    open.store(false, Ordering::SeqCst);
    let request = Request {
        token: "secret".into(),
        words: vec!["send".into(), "7".into(), "rm".into()],
    };
    write_line(&late, &request).expect("asked");
    let heard: Reply = read_line(&late, PATIENCE).expect("answered");
    assert_eq!(heard, Reply::Error(SHUT.into()));
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

#[test]
fn a_line_that_drips_in_is_cut_at_its_deadline() {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a port");
    let port = listener.local_addr().expect("its address").port();
    let drip = thread::spawn(move || {
        let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).expect("the door");
        // Each byte comes well inside the deadline; the line never ends.
        while stream.write_all(b"x").is_ok() {
            thread::sleep(Duration::from_millis(50));
        }
    });
    let (stream, _) = listener.accept().expect("a caller");
    let began = Instant::now();
    let read = read_line::<Request>(&stream, Duration::from_millis(400));
    assert!(read.is_err(), "no line came");
    assert!(
        began.elapsed() < Duration::from_secs(2),
        "{:?}",
        began.elapsed()
    );
    drop(stream);
    drip.join().expect("the drip ends");
}

#[test]
fn a_door_leaves_another_copy_s_file() {
    let dir = std::env::temp_dir().join(format!("ely-control-leave-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join(FILE);
    write_private(&file, "2 b\n").expect("written");
    leave(&file, "1 a\n");
    assert_eq!(fs::read_to_string(&file).expect("kept"), "2 b\n");
    leave(&file, "2 b\n");
    assert!(!file.exists(), "its own file goes");
    fs::remove_dir_all(&dir).expect("removed");
}

#[test]
fn a_line_holds_the_longest_and_no_byte_more() {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a port");
    let port = listener.local_addr().expect("its address").port();
    // A JSON string of `bytes` bytes, then the newline.
    let read = |bytes: usize| {
        let writer = thread::spawn(move || {
            let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).expect("the door");
            let line = format!("\"{}\"\n", "x".repeat(bytes - 2));
            // The reader may hang up before the line is through.
            let _ = stream.write_all(line.as_bytes());
        });
        let (stream, _) = listener.accept().expect("a caller");
        let read = read_line::<String>(&stream, PATIENCE);
        drop(stream);
        writer.join().expect("the writer ends");
        read
    };
    assert_eq!(read(LONGEST - 1).expect("it fits").len(), LONGEST - 3);
    let over = read(LONGEST).expect_err("a byte too long");
    assert!(format!("{over:#}").contains("over"), "{over:#}");
}
