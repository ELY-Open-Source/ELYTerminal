use anyhow::Context as _;
use core_graphics::{
    event::{CGEvent, CGEventFlags, CGEventType, CGMouseButton, EventField, ScrollEventUnit},
    event_source::{CGEventSource, CGEventSourceStateID},
    geometry::{CGPoint, CGRect},
};
use foreign_types::ForeignType;
use objc::{
    class, msg_send,
    runtime::{BOOL, NO, Object, Sel, YES},
    sel, sel_impl,
};

/// US layout key codes and what each key types.
const KEYS: [(&str, u16, &str); 38] = [
    (",", 43, ","),
    ("a", 0, "a"),
    ("b", 11, "b"),
    ("c", 8, "c"),
    ("d", 2, "d"),
    ("e", 14, "e"),
    ("f", 3, "f"),
    ("i", 34, "i"),
    ("k", 40, "k"),
    ("s", 1, "s"),
    ("v", 9, "v"),
    ("t", 17, "t"),
    ("w", 13, "w"),
    ("1", 18, "1"),
    ("2", 19, "2"),
    ("5", 23, "5"),
    ("9", 25, "9"),
    ("0", 29, "0"),
    ("=", 24, "="),
    ("minus", 27, "-"),
    ("m", 46, "m"),
    ("n", 45, "n"),
    ("q", 12, "q"),
    ("[", 33, "["),
    ("]", 30, "]"),
    ("space", 49, " "),
    ("tab", 48, "\t"),
    ("enter", 36, "\r"),
    ("backspace", 51, "\u{7f}"),
    ("escape", 53, "\u{1b}"),
    ("up", 126, "\u{F700}"),
    ("down", 125, "\u{F701}"),
    ("left", 123, "\u{F702}"),
    ("right", 124, "\u{F703}"),
    ("delete", 117, "\u{F728}"),
    ("f1", 122, "\u{F704}"),
    ("f5", 96, "\u{F708}"),
    ("f6", 97, "\u{F709}"),
];

/// AppKit's `NSRange`.
#[repr(C)]
struct Range {
    location: u64,
    length: u64,
}

fn string(text: &str) -> *mut Object {
    let text = std::ffi::CString::new(text).expect("no NUL in key text");
    unsafe { msg_send![class!(NSString), stringWithUTF8String: text.as_ptr()] }
}

/// An input method's `mark` or `commit` of `text`.
fn compose(call: &str, text: &str, number: u32) -> anyhow::Result<()> {
    let none = Range {
        location: i64::MAX as u64,
        length: 0,
    };
    unsafe {
        let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
        let window: *mut Object = msg_send![app, windowWithWindowNumber: i64::from(number)];
        anyhow::ensure!(!window.is_null(), "no window {number}");
        let client: *mut Object = msg_send![window, firstResponder];
        match call {
            "mark" => {
                let end = Range {
                    location: text.encode_utf16().count() as u64,
                    length: 0,
                };
                let _: () = msg_send![client, setMarkedText: string(text) selectedRange: end replacementRange: none];
            }
            "commit" => {
                let _: () = msg_send![client, insertText: string(text) replacementRange: none];
            }
            other => anyhow::bail!("no input method call {other}"),
        }
    }
    log::info!("shot: {call} {text:?}");
    Ok(())
}

unsafe extern "C" {
    fn CGEventSetLocation(event: *mut core_graphics::sys::CGEvent, location: CGPoint);
}

/// A gesture's steps: each one's name, point and click count.
fn steps(gesture: &str, points: &[CGPoint]) -> anyhow::Result<Vec<(String, CGPoint, i64)>> {
    let step = |name: &str, at: CGPoint, clicks| (name.to_string(), at, clicks);
    let (at, to) = (points[0], points.get(1).copied());
    Ok(match gesture {
        "click" | "click2" | "click3" => (1..=gesture[5..].parse().unwrap_or(1))
            .flat_map(|clicks| [step("ldown", at, clicks), step("lup", at, clicks)])
            .collect(),
        "rclick" => vec![step("rdown", at, 1), step("rup", at, 1)],
        "mclick" => vec![step("mdown", at, 1), step("mup", at, 1)],
        "drag" => {
            let to = to.context("drag needs a second point")?;
            vec![
                step("ldown", at, 1),
                step("ldrag", to, 1),
                step("lup", to, 1),
            ]
        }
        name => vec![step(name, at, 1)],
    })
}

/// A step's event type, button and gpui's selector.
fn kind(step: &str) -> anyhow::Result<(CGEventType, CGMouseButton, &'static str)> {
    Ok(match step {
        "ldown" => (
            CGEventType::LeftMouseDown,
            CGMouseButton::Left,
            "mouseDown:",
        ),
        "lup" => (CGEventType::LeftMouseUp, CGMouseButton::Left, "mouseUp:"),
        "ldrag" => (
            CGEventType::LeftMouseDragged,
            CGMouseButton::Left,
            "mouseDragged:",
        ),
        "rdown" => (
            CGEventType::RightMouseDown,
            CGMouseButton::Right,
            "rightMouseDown:",
        ),
        "rup" => (
            CGEventType::RightMouseUp,
            CGMouseButton::Right,
            "rightMouseUp:",
        ),
        "mdown" => (
            CGEventType::OtherMouseDown,
            CGMouseButton::Center,
            "otherMouseDown:",
        ),
        "mup" => (
            CGEventType::OtherMouseUp,
            CGMouseButton::Center,
            "otherMouseUp:",
        ),
        "move" => (CGEventType::MouseMoved, CGMouseButton::Left, "mouseMoved:"),
        other => anyhow::bail!("no mouse gesture {other}"),
    })
}

/// Hands gpui's view a gesture as AppKit events.
fn mouse(name: &str, flags: u64, number: u32) -> anyhow::Result<()> {
    let mut fields = name.split('@');
    let gesture = fields.next().context("a gesture")?;
    let numbers: Vec<f64> = fields
        .flat_map(|field| field.split(','))
        .map(str::parse)
        .collect::<Result<_, _>>()
        .with_context(|| format!("numbers in {name}"))?;
    anyhow::ensure!(numbers.len() >= 2, "{name} needs a point");
    unsafe {
        let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
        let window: *mut Object = msg_send![app, windowWithWindowNumber: i64::from(number)];
        anyhow::ensure!(!window.is_null(), "no window {number}");
        let view: *mut Object = msg_send![window, firstResponder];
        let content: *mut Object = msg_send![window, contentView];
        let frame: CGRect = msg_send![content, frame];
        let height = frame.size.height;
        let points: Vec<CGPoint> = numbers
            .chunks_exact(2)
            .map(|pair| CGPoint::new(pair[0], height - pair[1]))
            .collect();
        let screens: *mut Object = msg_send![class!(NSScreen), screens];
        let main: *mut Object = msg_send![screens, objectAtIndex: 0u64];
        let screen: CGRect = msg_send![main, frame];
        let source = || {
            CGEventSource::new(CGEventSourceStateID::Private)
                .map_err(|_| anyhow::anyhow!("no event source"))
        };
        let deliver = |event: CGEvent, at: CGPoint, selector: &str| -> anyhow::Result<()> {
            CGEventSetLocation(
                event.as_ptr(),
                CGPoint::new(at.x, screen.size.height - at.y),
            );
            event.set_flags(CGEventFlags::from_bits_truncate(flags));
            let native: *mut Object = msg_send![class!(NSEvent), eventWithCGEvent: event.as_ptr()];
            anyhow::ensure!(!native.is_null(), "AppKit refused {name}");
            let selector = Sel::register(selector);
            let _: *mut Object = msg_send![view, performSelector: selector withObject: native];
            Ok(())
        };
        if let Some(up) = gesture.strip_prefix("scroll") {
            let lines = *numbers.get(2).context("a scroll needs lines")? as i32;
            let lines = if up == "up" { lines } else { -lines };
            let event = CGEvent::new_scroll_event(source()?, ScrollEventUnit::LINE, 1, lines, 0, 0)
                .map_err(|_| anyhow::anyhow!("no scroll event"))?;
            deliver(event, points[0], "scrollWheel:")?;
        } else {
            for (step, at, clicks) in steps(gesture, &points)? {
                let (kind, button, selector) = kind(&step)?;
                let event = CGEvent::new_mouse_event(source()?, kind, at, button)
                    .map_err(|_| anyhow::anyhow!("no mouse event"))?;
                event.set_integer_value_field(EventField::MOUSE_EVENT_CLICK_STATE, clicks);
                deliver(event, at, selector)?;
            }
        }
    }
    log::info!("shot: mouse {name}");
    Ok(())
}

/// Posts one spec on window `number`.
pub fn post(spec: &str, number: u32) -> anyhow::Result<()> {
    if let Some((call, text)) = spec.split_once(':') {
        return match call {
            "down" => key(text, number, &[10]),
            "up" => key(text, number, &[11]),
            call => compose(call, text, number),
        };
    }
    if let "activate" | "deactivate" = spec {
        unsafe {
            let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
            if spec == "activate" {
                let _: () = msg_send![app, activateIgnoringOtherApps: YES];
            } else {
                let _: () = msg_send![app, deactivate];
            }
        }
        log::info!("shot: {spec}");
        return Ok(());
    }
    key(spec, number, &[10, 11])
}

/// Posts key `spec`, or a mouse gesture, as AppKit events of `kinds`: 10 down, 11 up.
fn key(spec: &str, number: u32, kinds: &[u64]) -> anyhow::Result<()> {
    let mut parts: Vec<&str> = spec.split('-').collect();
    let name = parts.pop().context("an empty key")?;
    let mut flags: u64 = 0;
    let mut repeat = NO;
    for modifier in &parts {
        flags |= match *modifier {
            "repeat" => {
                repeat = YES;
                0
            }
            "shift" => 0x20000 | 0x02,
            "ctrl" => 0x40000 | 0x01,
            "lopt" => 0x80000 | 0x20,
            "ropt" => 0x80000 | 0x40,
            "cmd" => 0x100000 | 0x08,
            other => anyhow::bail!("no modifier {other}"),
        };
    }
    if name.contains('@') {
        return mouse(name, flags, number);
    }
    let (_, code, plain) = KEYS
        .iter()
        .find(|(key, ..)| *key == name)
        .with_context(|| format!("no key code for {name}"))?;
    if plain
        .chars()
        .next()
        .is_some_and(|ch| ('\u{F700}'..='\u{F8FF}').contains(&ch))
    {
        flags |= 0x800000 | 0x200000;
    }
    let ignoring = if flags & 0x20000 != 0 {
        plain.to_uppercase()
    } else {
        plain.to_string()
    };
    let typed = match plain.chars().next() {
        _ if flags & 0x80000 != 0 => String::new(),
        Some(letter @ 'a'..='z') if flags & 0x40000 != 0 => {
            char::from(letter as u8 - b'a' + 1).to_string()
        }
        _ => ignoring.clone(),
    };
    for &kind in kinds {
        unsafe {
            let event: *mut Object = msg_send![
                class!(NSEvent),
                keyEventWithType: kind
                location: CGPoint::new(0.0, 0.0)
                modifierFlags: flags
                timestamp: 0.0f64
                windowNumber: i64::from(number)
                context: std::ptr::null_mut::<Object>()
                characters: string(&typed)
                charactersIgnoringModifiers: string(&ignoring)
                isARepeat: if kind == 10 { repeat } else { NO }
                keyCode: *code
            ];
            anyhow::ensure!(!event.is_null(), "AppKit refused the key {spec}");
            let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
            let _: () = msg_send![app, postEvent: event atStart: NO];
        }
    }
    let active: BOOL = unsafe {
        let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
        msg_send![app, isActive]
    };
    let state = if active == YES { "active" } else { "inactive" };
    log::info!("shot: typed {spec}, the app {state}");
    Ok(())
}

/// A file drop of `paths`, one a line, at `at`, `x,y` from the content's top-left, as gpui's AppKit view sends it.
pub fn drop(at: &str, paths: &str) -> anyhow::Result<[gpui::PlatformInput; 4]> {
    use gpui::{ExternalPaths, FileDropEvent, PlatformInput, point, px};
    let (x, y) = at
        .split_once(',')
        .with_context(|| format!("a point in {at}"))?;
    let position = point(px(x.parse()?), px(y.parse()?));
    let paths = ExternalPaths(paths.lines().map(Into::into).collect());
    Ok([
        FileDropEvent::Entered { position, paths },
        FileDropEvent::Pending { position },
        FileDropEvent::Submit { position },
        FileDropEvent::Ended,
    ]
    .map(PlatformInput::FileDrop))
}
