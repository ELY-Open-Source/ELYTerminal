use std::path::PathBuf;

use base64_simd::STANDARD;
use gpui::{ClipboardEntry, ClipboardItem, ClipboardString, ExternalPaths, Image, ImageFormat};

use super::{
    mime::{holds, uris},
    *,
};

/// A folder and its `file://` form on this system.
#[cfg(unix)]
const ROOT: (&str, &str) = ("/tmp/", "file:///tmp/");
#[cfg(windows)]
const ROOT: (&str, &str) = ("C:\\tmp\\", "file:///C:/tmp/");

fn local(name: &str) -> PathBuf {
    PathBuf::from(format!("{}{name}", ROOT.0))
}

fn b64(text: &str) -> String {
    STANDARD.encode_to_string(text)
}

fn packet(body: &str) -> Option<Packet> {
    read(body.as_bytes())
}

/// Each packet's step in one session, its answers as text.
fn run(session: &mut Session, bodies: &[String], primary: bool) -> Vec<String> {
    bodies
        .iter()
        .map(
            |body| match session.heard(packet(body).expect("a packet"), primary) {
                Step::Nothing => "-".to_string(),
                Step::Answer(bytes) => String::from_utf8(bytes).unwrap(),
                Step::Store(place, id, item) => {
                    format!("STORE {place:?} {id} {}", types(&item).join(" "))
                }
                Step::Read(read) => format!("READ {}", read.mimes.join(" ")),
            },
        )
        .collect()
}

fn data(mime: &str, payload: &str) -> String {
    format!("type=wdata:mime={};{payload}", b64(mime))
}

#[test]
fn packets_read_as_kitty_reads_them() {
    let pass = Pass {
        word: "secret".into(),
        name: "vim".into(),
    };
    let read = format!(
        "type=read:loc=primary:id=a<b>c:pw={}:name={};{}",
        b64("secret"),
        b64("vim"),
        b64(" text/plain\nimage/png ")
    );
    let expected = Read {
        place: Place::Primary,
        mimes: vec!["text/plain".into(), "image/png".into()],
        id: "abc".into(),
        pass: Some(pass),
    };
    assert_eq!(packet(&read), Some(Packet::Read(expected)));
    // A password without a name is none.
    let Some(Packet::Read(alone)) = packet(&format!("type=read:pw={};{}", b64("x"), b64(".")))
    else {
        panic!("a read")
    };
    assert_eq!((alone.pass, alone.mimes), (None, vec![".".to_string()]));
    assert_eq!(
        packet("type=wdata"),
        Some(Packet::Data {
            mime: None,
            payload: vec![]
        })
    );
    assert_eq!(
        packet(&format!(
            "type=walias:mime={};{}",
            b64("text/plain"),
            b64("UTF8_STRING text/x")
        )),
        Some(Packet::Alias {
            mime: "text/plain".into(),
            aliases: vec!["UTF8_STRING".into(), "text/x".into()]
        })
    );
    // A write's broken packet fails it; a read's passes by.
    assert_eq!(
        packet("type=wdata:mime=dGV4dC9wbGFpbg;AAAA"),
        Some(Packet::Broken)
    );
    assert_eq!(packet("type=wdata:oops;AAAA"), Some(Packet::Broken));
    assert_eq!(packet("type=walias;dGV4dA=="), Some(Packet::Broken));
    assert_eq!(packet("type=read;not base64"), None);
    assert_eq!(packet("type=read:oops;Lg=="), None);
    assert_eq!(packet("type=paste;Lg=="), None);
    assert_eq!(long(b"type=wdata:mime=x;AAA"), Some(Packet::Broken));
    assert_eq!(long(b"type=read;AAA"), None);
}

#[test]
fn a_write_decodes_its_chunks_as_one_stream() {
    let mut session = Session::default();
    let text = b64("hello, clipboard");
    let (head, tail) = text.split_at(5);
    let bodies = [
        "type=write:id=w1".to_string(),
        data("text/plain", head),
        data("text/plain", tail),
        data("image/png", &b64("PNG")),
        format!(
            "type=walias:mime={};{}",
            b64("text/plain"),
            b64("text/plain;charset=utf-8")
        ),
        "type=wdata".into(),
    ];
    assert_eq!(
        run(&mut session, &bodies, false),
        [
            "-",
            "-",
            "-",
            "-",
            "-",
            "STORE Clipboard w1 text/plain image/png"
        ]
    );
    // Files go as a uri-list; an unknown type passes by.
    let list = format!("# kept\r\n{}a%20b\r\nhttp://elsewhere/x\r\n", ROOT.1);
    let bodies = [
        "type=write".to_string(),
        data("text/html", &b64("<b>")),
        data("text/uri-list", &b64(&list)),
        "type=wdata".into(),
    ];
    assert_eq!(
        run(&mut session, &bodies, false)[3],
        "STORE Clipboard  text/uri-list"
    );
}

#[test]
fn a_broken_write_fails_once_and_its_rest_passes_by() {
    let einval = |id: &str| answer("write", "EINVAL", &[("id", id)], b"");
    let text = |bytes: Vec<u8>| String::from_utf8(bytes).unwrap();
    for (case, bodies) in [
        ("not base64", vec![data("text/plain", "AB!D")]),
        (
            "unpadded",
            vec![data("text/plain", "QUI"), "type=wdata".into()],
        ),
        (
            "past padding",
            vec![data("text/plain", "QQ=="), data("text/plain", "QQ==")],
        ),
        (
            "a type again",
            vec![
                data("text/plain", "QQ=="),
                data("image/png", "QQ=="),
                data("text/plain", "QQ=="),
            ],
        ),
        (
            "no type kept",
            vec![data("text/html", "QQ=="), "type=wdata".into()],
        ),
        (
            "not UTF-8",
            vec![data("text/plain", "/w=="), "type=wdata".into()],
        ),
        ("broken", vec!["type=wdata:mime=x;QQ==".into()]),
        ("nothing", vec!["type=wdata".into()]),
    ] {
        let mut session = Session::default();
        let mut bodies = bodies;
        bodies.insert(0, "type=write:id=7".into());
        let heard = run(&mut session, &bodies, false);
        assert_eq!(heard.last().unwrap(), &text(einval("7")), "{case}");
        assert!(
            heard[..heard.len() - 1].iter().all(|step| step == "-"),
            "{case}: {heard:?}"
        );
        // Until the next write, its packets pass by.
        assert_eq!(
            run(
                &mut session,
                &[data("text/plain", "QQ=="), "type=wdata".into()],
                false
            ),
            ["-", "-"],
            "{case}"
        );
    }
    let mut session = Session::default();
    let big = b64(&"x".repeat(LIMIT + 1));
    let heard = run(
        &mut session,
        &["type=write".into(), data("text/plain", &big)],
        false,
    );
    assert_eq!(heard[1], text(answer("write", "EFBIG", &[], b"")));
    let heard = run(
        &mut session,
        &[
            "type=write:loc=primary:id=p".into(),
            data("text/plain", "QQ=="),
        ],
        false,
    );
    assert_eq!(
        heard,
        [
            text(answer("write", "ENOSYS", &[("id", "p")], b"")),
            "-".into()
        ]
    );
    let heard = run(
        &mut session,
        &[
            "type=write:loc=primary".into(),
            data("text/plain", "QQ=="),
            "type=wdata".into(),
        ],
        true,
    );
    assert_eq!(heard[2], "STORE Primary  text/plain");
}

#[test]
fn a_read_answers_ok_its_data_in_chunks_then_done() {
    let read = Read {
        place: Place::Primary,
        mimes: vec![],
        id: "r".into(),
        pass: None,
    };
    let data = vec![7u8; 5000];
    let out = String::from_utf8(answers(
        &read,
        &[("image/png".into(), data.clone()), (LIST.into(), vec![])],
        "pw",
    ))
    .unwrap();
    let (pw, mime) = (b64("pw"), b64("image/png"));
    let expected = [
        format!("\x1b]5522;type=read:status=OK:loc=primary:id=r:pw={pw}\x1b\\"),
        format!(
            "\x1b]5522;type=read:status=DATA:id=r:mime={mime}:pw={pw};{}\x1b\\",
            STANDARD.encode_to_string(&data[..4096])
        ),
        format!(
            "\x1b]5522;type=read:status=DATA:id=r:mime={mime}:pw={pw};{}\x1b\\",
            STANDARD.encode_to_string(&data[4096..])
        ),
        format!("\x1b]5522;type=read:status=DATA:id=r:mime=Lg==:pw={pw}\x1b\\"),
        format!("\x1b]5522;type=read:status=DONE:id=r:pw={pw}\x1b\\"),
    ];
    assert_eq!(out, expected.concat());
    let plain = Read {
        place: Place::Clipboard,
        ..read
    };
    assert!(
        String::from_utf8(answers(&plain, &[], ""))
            .unwrap()
            .starts_with("\x1b]5522;type=read:status=OK:id=r\x1b\\")
    );
}

#[test]
fn a_read_finds_each_type_and_lists_them() {
    let png = Image::from_bytes(ImageFormat::Png, vec![1, 2, 3]);
    let paths = ExternalPaths(vec![local("a b"), local("é")].into());
    let item = ClipboardItem {
        entries: vec![
            ClipboardEntry::String(ClipboardString::new("hi".into())),
            ClipboardEntry::Image(png),
            ClipboardEntry::ExternalPaths(paths),
        ],
    };
    let asked: Vec<String> = [
        ".",
        "text/plain;charset=utf-8",
        "image/jpeg",
        "image/png",
        "text/uri-list",
    ]
    .map(String::from)
    .into();
    let got = found(Some(&item), &asked);
    let names: Vec<&str> = got.iter().map(|(mime, _)| mime.as_str()).collect();
    assert_eq!(
        names,
        [
            ".",
            "text/plain;charset=utf-8",
            "image/png",
            "text/uri-list"
        ]
    );
    assert_eq!(got[0].1, b"text/plain image/png text/uri-list\n");
    assert_eq!(got[1].1, b"hi");
    assert_eq!(got[2].1, [1, 2, 3]);
    let uris = format!("{0}a%20b\r\n{0}%C3%A9\r\n", ROOT.1);
    assert_eq!(got[3].1, uris.as_bytes());
    assert_eq!(found(None, &asked), [(".".to_string(), vec![])]);
}

#[test]
fn a_uri_list_names_the_files_it_was_made_of() {
    #[cfg_attr(unix, allow(unused_mut))]
    let mut paths: Vec<PathBuf> = ["plain", "with space", "100%/é#;", "a=b"].map(local).into();
    #[cfg(windows)]
    paths.push(PathBuf::from(r"\\server\share\a b"));
    let list = uris(&paths);
    let parts = [(
        "text/uri-list".to_string(),
        std::rc::Rc::new(list.into_bytes()),
    )];
    let item = item(&parts).expect("files");
    let [ClipboardEntry::ExternalPaths(back)] = item.entries.as_slice() else {
        panic!("one file list")
    };
    assert_eq!(back.paths(), paths.as_slice());
}

#[test]
fn aliases_share_their_data_and_their_names_count() {
    let mut write = Write::new(Place::Clipboard, String::new());
    write.add("text/plain", b"QUJD").unwrap();
    write
        .alias(
            "text/plain".into(),
            vec!["UTF8_STRING".into(), "text/x".into()],
        )
        .unwrap();
    let parts = write.finish().unwrap();
    assert!(
        std::rc::Rc::ptr_eq(&parts[0].1, &parts[1].1)
            && std::rc::Rc::ptr_eq(&parts[0].1, &parts[2].1)
    );
    let mut write = Write::new(Place::Clipboard, String::new());
    write.add("text/plain", b"QUJD").unwrap();
    let names = (0..20).map(|n| format!("text/alias-{n}")).collect();
    assert_eq!(write.alias("text/plain".into(), names), Err(Fail::TooBig));
    // Each alias holds its type's name too.
    let mut write = Write::new(Place::Clipboard, String::new());
    let few = vec!["a".to_string(), "b".to_string()];
    assert_eq!(write.alias("t".repeat(200), few), Err(Fail::TooBig));
}

#[test]
fn an_overlong_packet_fails_a_write_unless_it_reads() {
    let id = "x".repeat(BODY);
    assert_eq!(
        long(format!("type=write:id={id}").as_bytes()),
        Some(Packet::Broken)
    );
    assert_eq!(long(format!("id={id}").as_bytes()), Some(Packet::Broken));
    assert_eq!(long(b"type=read;AAAA"), None);
}

#[test]
fn a_write_holds_only_what_the_clipboard_read_back() {
    let text = |text: &str| ClipboardEntry::String(ClipboardString::new(text.into()));
    let png = ClipboardEntry::Image(Image::from_bytes(ImageFormat::Png, vec![1]));
    let tiff = ClipboardEntry::Image(Image::from_bytes(ImageFormat::Tiff, vec![2]));
    let files = |name| ClipboardEntry::ExternalPaths(ExternalPaths(vec![local(name)].into()));
    let item = |entries| ClipboardItem { entries };
    let written = item(vec![text("a"), png, files("x")]);
    assert!(holds(
        Some(&item(vec![files("x"), text("a"), tiff.clone()])),
        &written
    ));
    assert!(!holds(
        Some(&item(vec![files("x"), text("old"), tiff.clone()])),
        &written
    ));
    assert!(!holds(
        Some(&item(vec![files("y"), text("a"), tiff.clone()])),
        &written
    ));
    assert!(!holds(Some(&item(vec![files("x"), text("a")])), &written));
    // An old image of the same format is no proof.
    let old = ClipboardEntry::Image(Image::from_bytes(ImageFormat::Png, vec![9]));
    assert!(!holds(
        Some(&item(vec![files("x"), text("a"), old])),
        &written
    ));
    assert!(!holds(None, &written));
}

#[cfg(unix)]
#[test]
fn a_unix_file_name_not_in_utf8_goes_and_comes_back() {
    use std::os::unix::ffi::OsStringExt;
    let name = PathBuf::from(std::ffi::OsString::from_vec(b"/tmp/a\xffb".to_vec()));
    let list = uris(std::slice::from_ref(&name));
    assert_eq!(list, "file:///tmp/a%FFb\r\n");
    let parts = [(
        "text/uri-list".to_string(),
        std::rc::Rc::new(list.into_bytes()),
    )];
    let item = item(&parts).expect("the file");
    let [ClipboardEntry::ExternalPaths(back)] = item.entries.as_slice() else {
        panic!("one file list")
    };
    assert_eq!(back.paths(), [name]);
}

#[test]
fn empty_types_count_against_a_write() {
    let mut write = Write::new(Place::Clipboard, String::new());
    let names: Vec<String> = (0..100).map(|n| format!("type/{n}")).collect();
    let failed = names.iter().find_map(|mime| write.add(mime, b"").err());
    assert_eq!(failed, Some(Fail::TooBig));
}

#[test]
fn a_read_answers_each_kind_once() {
    let png = Image::from_bytes(ImageFormat::Png, vec![1]);
    let item = ClipboardItem {
        entries: vec![
            ClipboardEntry::String(ClipboardString::new("hi".into())),
            ClipboardEntry::Image(png),
        ],
    };
    let asked = [
        "text/plain",
        "text/plain",
        "text/plain;charset=utf-8",
        ".",
        ".",
        "image/png",
        "image/png",
    ];
    let asked: Vec<String> = asked.map(String::from).into();
    let got = found(Some(&item), &asked);
    let names: Vec<&str> = got.iter().map(|(mime, _)| mime.as_str()).collect();
    assert_eq!(names, ["text/plain", ".", "image/png"]);
}
