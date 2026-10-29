use super::busy;

#[test]
fn a_clipboard_another_window_holds_is_busy() {
    use std::sync::mpsc::channel;
    use windows_sys::Win32::{
        System::DataExchange::{CloseClipboard, OpenClipboard},
        UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow, HWND_MESSAGE},
    };
    let (held, hold) = (channel(), channel::<()>());
    // Another program holds it with a window of its own.
    let holder = std::thread::spawn(move || {
        let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        // SAFETY: a message-only window, destroyed below.
        let window = unsafe {
            let none = std::ptr::null_mut();
            let (name, style) = (class.as_ptr(), 0);
            CreateWindowExW(
                0,
                name,
                name,
                style,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                none,
                none,
                none,
            )
        };
        assert!(!window.is_null());
        assert_ne!(unsafe { OpenClipboard(window) }, 0);
        held.0.send(()).unwrap();
        hold.1.recv().unwrap();
        unsafe {
            CloseClipboard();
            DestroyWindow(window);
        }
    });
    held.1.recv().unwrap();
    assert!(busy());
    hold.0.send(()).unwrap();
    holder.join().unwrap();
    assert!(!busy());
}
