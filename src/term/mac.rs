use std::ffi::c_void;

use anyhow::Context as _;
use block::ConcreteBlock;
use objc::{
    class, msg_send,
    runtime::{BOOL, NO, Object},
    sel, sel_impl,
};

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    static kTISPropertyUnicodeKeyLayoutData: *const c_void;
    fn TISCopyCurrentKeyboardLayoutInputSource() -> *mut c_void;
    fn TISGetInputSourceProperty(source: *mut c_void, key: *const c_void) -> *const c_void;
    fn LMGetKbdType() -> u16;
    fn UCKeyTranslate(
        layout: *const c_void,
        code: u16,
        action: u16,
        modifiers: u32,
        keyboard: u32,
        options: u32,
        dead: *mut u32,
        most: usize,
        length: *mut usize,
        text: *mut u16,
    ) -> u32;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFDataGetBytePtr(data: *const c_void) -> *const u8;
    fn CFRelease(object: *const c_void);
}

/// AppKit's event types for a key going down and coming up.
const KEY_DOWN: u64 = 10;
const KEY_UP: u64 = 11;

/// Shift, Option and Command bits in an event's modifier flags.
const SHIFT: u64 = 1 << 17;
const OPTION: u64 = 1 << 19;
const COMMAND: u64 = 1 << 20;

/// Command in a layout's modifier state, as `UCKeyTranslate` takes it.
const LAYOUT_COMMAND: u32 = 1;

/// The device bits of the left and right Option keys in an event's modifier flags.
const LEFT_OPTION: u64 = 0x20;
const RIGHT_OPTION: u64 = 0x40;

/// What gpui drops from the key event AppKit is handling.
#[derive(Clone, Copy, Debug, Default)]
pub struct KeyFacts {
    pub left_option: bool,
    pub right_option: bool,
    /// Held down; gpui loses this for keys AppKit routes through commands.
    pub repeat: bool,
    /// The key's own character, where gpui named the one Shift made and dropped Shift.
    pub unshifted: Option<char>,
    /// AppKit's key code, which Shift does not change.
    pub code: Option<u16>,
}

/// The facts of the key event AppKit is handling now, which gpui named `name`, with `shift` or not.
pub fn key_facts(name: &str, shift: bool) -> anyhow::Result<KeyFacts> {
    let (flags, repeat, code) = unsafe {
        let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
        let event: *mut Object = msg_send![app, currentEvent];
        anyhow::ensure!(!event.is_null(), "macOS: no key event to read");
        let kind: u64 = msg_send![event, type];
        anyhow::ensure!(
            kind == KEY_DOWN || kind == KEY_UP,
            "macOS: the current event is type {kind}, not a key"
        );
        let flags: u64 = msg_send![event, modifierFlags];
        let repeat: BOOL = msg_send![event, isARepeat];
        let code: u16 = msg_send![event, keyCode];
        (flags, repeat != NO, code)
    };
    let one = name.chars().count() == 1;
    let unshifted = if flags & SHIFT != 0 && !shift && one {
        let own = own(code, flags)?;
        log::debug!("macOS: Shift made {name:?} of key {code}, whose own is {own:?}");
        own
    } else {
        None
    };
    Ok(KeyFacts {
        left_option: flags & LEFT_OPTION != 0,
        right_option: flags & RIGHT_OPTION != 0,
        repeat,
        unshifted,
        code: Some(code),
    })
}

/// The Option keys AppKit's current key holds; none between keys.
pub fn held_options() -> Option<(bool, bool)> {
    let flags: u64 = unsafe {
        let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
        let event: *mut Object = msg_send![app, currentEvent];
        if event.is_null() {
            return None;
        }
        let kind: u64 = msg_send![event, type];
        if kind != KEY_DOWN && kind != KEY_UP {
            return None;
        }
        msg_send![event, modifierFlags]
    };
    (flags & OPTION != 0).then_some((flags & LEFT_OPTION != 0, flags & RIGHT_OPTION != 0))
}

/// Key `code`'s character without Shift, read as gpui reads key names: Command's layout while Command is down or the plain one types no ASCII.
fn own(code: u16, flags: u64) -> anyhow::Result<Option<char>> {
    let command =
        flags & COMMAND != 0 || (!typed(0, 0)?.is_ascii() && typed(0, LAYOUT_COMMAND)?.is_ascii());
    let own = typed(code, if command { LAYOUT_COMMAND } else { 0 })?;
    let mut chars = own.chars();
    Ok(match (chars.next(), chars.next()) {
        (Some(ch), None) if !ch.is_control() => Some(ch),
        _ => None,
    })
}

/// What key `code` types under `modifiers` in the current layout; a dead key types its own mark.
fn typed(code: u16, modifiers: u32) -> anyhow::Result<String> {
    const SPACE: u16 = 49;
    let mut text = [0u16; 4];
    let mut length = 0;
    unsafe {
        let source = TISCopyCurrentKeyboardLayoutInputSource();
        anyhow::ensure!(!source.is_null(), "macOS: no keyboard layout");
        let data = TISGetInputSourceProperty(source, kTISPropertyUnicodeKeyLayoutData);
        if !data.is_null() {
            let layout = CFDataGetBytePtr(data).cast::<c_void>();
            let keyboard = u32::from(LMGetKbdType());
            let mut dead = 0;
            for key in [code, SPACE] {
                UCKeyTranslate(
                    layout,
                    key,
                    0,
                    modifiers,
                    keyboard,
                    0,
                    &mut dead,
                    text.len(),
                    &mut length,
                    text.as_mut_ptr(),
                );
                if dead == 0 {
                    break;
                }
            }
        }
        CFRelease(source);
        anyhow::ensure!(
            !data.is_null(),
            "macOS: the keyboard layout has no key table"
        );
    }
    String::from_utf16(&text[..length]).context("macOS: the layout typed no UTF-16")
}

/// Turns off the accent menu a held key opens, so held keys repeat as in other terminals.
pub fn repeat_held_keys() {
    unsafe {
        let key: *mut Object = msg_send![
            class!(NSString),
            stringWithUTF8String: c"ApplePressAndHoldEnabled".as_ptr()
        ];
        let off: *mut Object = msg_send![class!(NSNumber), numberWithBool: NO];
        let defaults: *mut Object =
            msg_send![class!(NSDictionary), dictionaryWithObject: off forKey: key];
        let store: *mut Object = msg_send![class!(NSUserDefaults), standardUserDefaults];
        let _: () = msg_send![store, registerDefaults: defaults];
    }
    log::info!("macOS: held keys repeat");
}

/// Hands the focused view the Command key-ups AppKit swallows, as Ghostty does.
pub fn deliver_command_key_ups() -> anyhow::Result<()> {
    let handler = ConcreteBlock::new(|event: *mut Object| -> *mut Object {
        unsafe {
            let flags: u64 = msg_send![event, modifierFlags];
            let window: *mut Object = msg_send![event, window];
            if flags & COMMAND == 0 || window.is_null() {
                return event;
            }
            let view: *mut Object = msg_send![window, firstResponder];
            let _: () = msg_send![view, keyUp: event];
            std::ptr::null_mut()
        }
    })
    .copy();
    unsafe {
        let monitor: *mut Object = msg_send![
            class!(NSEvent),
            addLocalMonitorForEventsMatchingMask: 1u64 << KEY_UP
            handler: &*handler
        ];
        anyhow::ensure!(
            !monitor.is_null(),
            "macOS: AppKit refused the key-up monitor"
        );
    }
    log::info!("macOS: Command key-ups reach the focused view");
    Ok(())
}
