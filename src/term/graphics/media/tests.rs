use std::path::{Path, PathBuf};

use alacritty_terminal::{
    index::{Column, Line, Point},
    term::Graphics as _,
};
use base64::{Engine, engine::general_purpose::STANDARD};

use super::{super::Graphics, range};

const UNREAD: &str = "EBADF:Failed to read image file";

fn send(graphics: &mut Graphics, keys: &str, name: &[u8]) -> Option<String> {
    let text = format!("{keys};{}", STANDARD.encode(name));
    graphics
        .command(text.as_bytes(), Point::new(Line(0), Column(0)))
        .reply
}

fn reply(id: u32, message: &str) -> Option<String> {
    Some(format!("\x1b_Gi={id};{message}\x1b\\"))
}

/// A fresh folder of this test's own under `root`.
fn folder(root: &Path, name: &str) -> PathBuf {
    let folder = root.join(format!("ely-graphics-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("a folder");
    folder
}

fn bytes(path: &Path) -> Vec<u8> {
    path.as_os_str().to_string_lossy().into_owned().into_bytes()
}

#[test]
fn a_file_reads_from_its_offset_for_its_size_and_stays() {
    let mut graphics = Graphics::new(0, (80, 24), (10, 20));
    let file = folder(&std::env::temp_dir(), "file").join("image");
    std::fs::write(&file, [9, 9, 1, 2, 3, 9]).expect("written");
    let path = bytes(&file);
    let first = send(&mut graphics, "i=1,t=f,f=24,s=1,v=1,O=2,S=3", &path);
    assert_eq!(first, reply(1, "OK"));
    assert_eq!(graphics.image(1).expect("stored").pixels, [1, 2, 3, 255]);
    let rest = send(&mut graphics, "i=2,t=f,f=24,s=1,v=1,O=3", &path);
    assert_eq!(rest, reply(2, "OK"));
    assert_eq!(graphics.image(2).expect("stored").pixels, [2, 3, 9, 255]);
    assert!(file.exists());
}

#[test]
fn every_file_that_does_not_read_answers_alike() {
    let mut graphics = Graphics::new(0, (80, 24), (10, 20));
    let folder = folder(&std::env::temp_dir(), "unread");
    let short = folder.join("short");
    std::fs::write(&short, [0; 6]).expect("written");
    let mut paths = vec![
        (bytes(&folder.join("missing")), "i=1,t=f,s=1,v=1"),
        (bytes(&folder), "i=1,t=f,s=1,v=1"),
        (b"relative/image".to_vec(), "i=1,t=f,s=1,v=1"),
        (bytes(&short), "i=1,t=f,s=1,v=1,S=10"),
        (bytes(&short), "i=1,t=f,s=1,v=1,O=7"),
        (bytes(&short), "i=1,t=f,s=1,v=1"),
        (bytes(&short), "i=1,t=f,f=100"),
    ];
    #[cfg(unix)]
    {
        let fifo = folder.join("fifo");
        let name = std::ffi::CString::new(bytes(&fifo)).expect("no NUL");
        // SAFETY: a NUL-ended path.
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        // Its open would wait for a writer; nothing opens it.
        paths.push((bytes(&fifo), "i=1,t=f,s=1,v=1"));
    }
    #[cfg(target_os = "linux")]
    paths.push((b"/proc/self/status".to_vec(), "i=1,t=f,s=1,v=1"));
    #[cfg(windows)]
    paths.extend([
        (br"\\server\share\image".to_vec(), "i=1,t=f,s=1,v=1"),
        (br"\\.\pipe\image".to_vec(), "i=1,t=f,s=1,v=1"),
        (br"C:\NUL".to_vec(), "i=1,t=f,s=1,v=1"),
    ]);
    for (path, keys) in paths {
        let shown = String::from_utf8_lossy(&path).into_owned();
        assert_eq!(
            send(&mut graphics, keys, &path),
            reply(1, UNREAD),
            "{shown} {keys}"
        );
    }
    assert!(graphics.image(1).is_none());
}

#[test]
fn a_temporary_file_is_read_then_deleted_only_where_kitty_allows() {
    let mut graphics = Graphics::new(0, (80, 24), (10, 20));
    let temporary = folder(&std::env::temp_dir(), "temporary");
    let file = temporary.join("tty-graphics-protocol-1");
    std::fs::write(&file, [1, 2, 3, 4]).expect("written");
    assert_eq!(
        send(&mut graphics, "i=1,t=t,s=1,v=1", &bytes(&file)),
        reply(1, "OK")
    );
    assert!(!file.exists());
    let plain = temporary.join("image");
    std::fs::write(&plain, [1, 2, 3, 4]).expect("written");
    assert_eq!(
        send(&mut graphics, "i=2,t=t,s=1,v=1", &bytes(&plain)),
        reply(2, UNREAD)
    );
    assert!(plain.exists());
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    let outside = folder(&target, "outside").join("tty-graphics-protocol-3");
    std::fs::write(&outside, [1, 2, 3, 4]).expect("written");
    assert_eq!(
        send(&mut graphics, "i=3,t=t,s=1,v=1", &bytes(&outside)),
        reply(3, UNREAD)
    );
    assert!(outside.exists());
    let _ = std::fs::remove_dir_all(outside.parent().expect("its folder"));
}

#[cfg(unix)]
#[test]
fn shared_memory_is_read_then_unlinked() {
    let name = format!("/ely-g{}", std::process::id());
    let c = std::ffi::CString::new(name.clone()).expect("no NUL");
    // SAFETY: a NUL-ended name; created for this test alone.
    #[cfg(target_os = "macos")]
    let fd = unsafe {
        libc::shm_open(
            c.as_ptr(),
            libc::O_CREAT | libc::O_RDWR,
            0o600 as libc::c_uint,
        )
    };
    // SAFETY: as above.
    #[cfg(not(target_os = "macos"))]
    let fd = unsafe { libc::shm_open(c.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o600) };
    assert!(fd >= 0, "{}", std::io::Error::last_os_error());
    // SAFETY: the new object, sized, then filled through a map.
    unsafe {
        assert_eq!(libc::ftruncate(fd, 6), 0);
        let map = libc::mmap(
            std::ptr::null_mut(),
            6,
            libc::PROT_WRITE,
            libc::MAP_SHARED,
            fd,
            0,
        );
        assert_ne!(map, libc::MAP_FAILED);
        std::ptr::copy_nonoverlapping([0u8, 0, 1, 2, 3, 4].as_ptr(), map.cast::<u8>(), 6);
        libc::munmap(map, 6);
        libc::close(fd);
    }
    let mut graphics = Graphics::new(0, (80, 24), (10, 20));
    assert_eq!(
        send(&mut graphics, "i=1,t=s,s=1,v=1,O=2", name.as_bytes()),
        reply(1, "OK")
    );
    assert_eq!(graphics.image(1).expect("stored").pixels, [1, 2, 3, 4]);
    // SAFETY: a NUL-ended name.
    #[cfg(target_os = "macos")]
    let again = unsafe { libc::shm_open(c.as_ptr(), libc::O_RDONLY, 0 as libc::c_uint) };
    // SAFETY: as above.
    #[cfg(not(target_os = "macos"))]
    let again = unsafe { libc::shm_open(c.as_ptr(), libc::O_RDONLY, 0) };
    assert_eq!(again, -1, "it was unlinked");
    assert_eq!(
        send(&mut graphics, "i=2,t=s,s=1,v=1", name.as_bytes()),
        reply(2, UNREAD)
    );
}

#[cfg(windows)]
#[test]
fn shared_memory_is_read_then_closed() {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        System::Memory::{
            CreateFileMappingW, FILE_MAP_WRITE, MapViewOfFile, PAGE_READWRITE, UnmapViewOfFile,
        },
    };
    let name = format!("Local\\ely-g{}", std::process::id());
    let wide: Vec<u16> = std::ffi::OsStr::new(&name)
        .encode_wide()
        .chain([0])
        .collect();
    // SAFETY: a paging-file page, named for this test.
    let handle = unsafe {
        CreateFileMappingW(
            INVALID_HANDLE_VALUE,
            std::ptr::null(),
            PAGE_READWRITE,
            0,
            4096,
            wide.as_ptr(),
        )
    };
    assert!(!handle.is_null(), "{}", std::io::Error::last_os_error());
    // SAFETY: a view of the new mapping, filled, then unmapped.
    unsafe {
        let view = MapViewOfFile(handle, FILE_MAP_WRITE, 0, 0, 0);
        assert!(!view.Value.is_null());
        std::ptr::copy_nonoverlapping([0u8, 0, 1, 2, 3, 4].as_ptr(), view.Value.cast::<u8>(), 6);
        UnmapViewOfFile(view);
    }
    let mut graphics = Graphics::new(0, (80, 24), (10, 20));
    assert_eq!(
        send(&mut graphics, "i=1,t=s,s=1,v=1,O=2", name.as_bytes()),
        reply(1, "OK")
    );
    assert_eq!(graphics.image(1).expect("stored").pixels, [1, 2, 3, 4]);
    // SAFETY: the handle made above.
    unsafe { CloseHandle(handle) };
    assert_eq!(
        send(&mut graphics, "i=2,t=s,s=1,v=1", name.as_bytes()),
        reply(2, UNREAD)
    );
}

#[test]
fn a_range_lies_inside_its_object() {
    assert_eq!(range(100, 10, 0, Some(20)), Ok(10..30));
    assert_eq!(range(100, 10, 0, None), Ok(10..100));
    assert_eq!(range(100, 10, 5, Some(20)), Ok(10..15));
    assert!(range(100, 101, 0, None).is_err());
    assert!(range(100, 10, 91, None).is_err());
    assert!(range(100, 90, 0, Some(20)).is_err());
}

#[cfg(unix)]
#[test]
fn a_folder_swapped_after_its_check_opens_nothing_and_deletes_nothing() {
    let temporary = folder(&std::env::temp_dir(), "swap");
    let held = temporary.join("held");
    std::fs::create_dir(&held).expect("a folder");
    std::fs::write(held.join("tty-graphics-protocol-1"), [1, 2, 3, 4]).expect("written");
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    let outside = folder(&target, "swap-outside");
    let kept = outside.join("tty-graphics-protocol-1");
    std::fs::write(&kept, [5, 6, 7, 8]).expect("written");
    let real = super::system::real(&held.join("tty-graphics-protocol-1")).expect("it resolves");
    std::fs::rename(&held, temporary.join("moved")).expect("moved");
    std::os::unix::fs::symlink(&outside, &held).expect("linked");
    assert!(
        super::system::open(&real, true).is_err(),
        "the swap was opened"
    );
    assert!(kept.exists());
    let _ = std::fs::remove_dir_all(&outside);
}

#[cfg(target_os = "linux")]
#[test]
fn a_pipe_in_dev_shm_answers_without_waiting_for_a_writer() {
    let name = format!("/ely-fifo-{}", std::process::id());
    let fifo = std::ffi::CString::new(format!("/dev/shm{name}")).expect("no NUL");
    // SAFETY: a NUL-ended path.
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let (sent, heard) = std::sync::mpsc::channel();
    let keys = name.clone();
    std::thread::spawn(move || {
        let mut graphics = Graphics::new(0, (80, 24), (10, 20));
        let _ = sent.send(send(&mut graphics, "i=1,t=s,s=1,v=1", keys.as_bytes()));
    });
    let answer = heard.recv_timeout(std::time::Duration::from_secs(5));
    // SAFETY: a NUL-ended path.
    unsafe { libc::unlink(fifo.as_ptr()) };
    assert_eq!(answer, Ok(reply(1, UNREAD)));
}

#[test]
fn a_local_image_past_the_quota_answers_as_one_that_does_not_read() {
    let mut graphics = Graphics::limited(0, (80, 24), (10, 20), 40);
    let file = folder(&std::env::temp_dir(), "quota").join("image");
    std::fs::write(&file, [0; 48]).expect("written");
    assert_eq!(
        send(&mut graphics, "i=1,t=f,s=4,v=3", &bytes(&file)),
        reply(1, UNREAD)
    );
}

#[cfg(unix)]
#[test]
fn a_file_in_a_folder_one_may_only_search_still_reads() {
    use std::os::unix::fs::PermissionsExt;
    let searched = folder(&std::env::temp_dir(), "search");
    let file = searched.join("image");
    std::fs::write(&file, [1, 2, 3, 4]).expect("written");
    let only = std::fs::Permissions::from_mode(0o111);
    std::fs::set_permissions(&searched, only).expect("search alone");
    let mut graphics = Graphics::new(0, (80, 24), (10, 20));
    let read = send(&mut graphics, "i=1,t=f,s=1,v=1", &bytes(&file));
    let back = std::fs::Permissions::from_mode(0o700);
    std::fs::set_permissions(&searched, back).expect("restored");
    assert_eq!(read, reply(1, "OK"));
}
