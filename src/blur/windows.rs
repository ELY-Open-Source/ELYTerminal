//! On Windows 10 a blurred window's acrylic clears while it moves or sizes: that DWM samples the blur again at each step, and the window trails the pointer by seconds.

use std::sync::{
    OnceLock,
    atomic::{AtomicIsize, Ordering},
};

use anyhow::{Context as _, bail};
use gpui::Window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows_sys::{
    Wdk::System::SystemServices::RtlGetVersion,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::{
            LibraryLoader::{GetModuleHandleW, GetProcAddress},
            SystemInformation::OSVERSIONINFOW,
        },
        UI::WindowsAndMessaging::{
            CallWindowProcW, GWLP_WNDPROC, GetPropW, RemovePropW, SWP_FRAMECHANGED, SWP_NOACTIVATE,
            SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SetPropW, SetWindowLongPtrW, SetWindowPos,
            WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE, WM_NCDESTROY, WNDPROC,
        },
    },
    core::{BOOL, PCWSTR},
    s, w,
};

/// Windows 11's first build, whose DWM moves an acrylic window as fast as a clear one.
const ELEVEN: u32 = 22000;
/// Windows 11 22H2, the first build that draws Mica.
const MICA: u32 = 22621;

/// gpui's window procedure, which the main windows' pass every message on to.
static GPUI: AtomicIsize = AtomicIsize::new(0);

/// A window's property while it is blurred, which its moves read.
const BLURRED: PCWSTR = w!("ElyTerminalBlurred");
/// A window's property while it moves or sizes, which a change of its blur reads.
const MOVING: PCWSTR = w!("ElyTerminalMoving");

/// gpui's accents: acrylic for Blurred, a clear gradient for Transparent.
const ACRYLIC: u32 = 4;
const CLEAR: u32 = 2;

/// A main window on Windows 10, whose moves clear its acrylic.
pub struct Moves {
    window: HWND,
}

impl Moves {
    /// Takes the window's moves on Windows 10; none on 11 and later.
    pub fn on(window: &Window) -> anyhow::Result<Option<Self>> {
        let build = build()?;
        if build >= ELEVEN {
            log::info!("blur: Windows build {build} moves acrylic as it is");
            return Ok(None);
        }
        let window = native(window)?;
        let gpui = unsafe { SetWindowLongPtrW(window, GWLP_WNDPROC, moving as *const () as isize) };
        anyhow::ensure!(gpui != 0, "blur: the window's procedure does not read");
        let kept = GPUI.swap(gpui, Ordering::Relaxed);
        anyhow::ensure!(
            kept == 0 || kept == gpui,
            "blur: gpui's windows have two procedures"
        );
        log::info!("blur: on Windows build {build} acrylic clears while the window moves");
        Ok(Some(Self { window }))
    }

    /// Notes whether the window is blurred, as its moves read it; one blurred mid-move stays clear until the move ends.
    pub fn blurred(&self, on: bool) {
        if !on {
            unsafe { RemovePropW(self.window, BLURRED) };
            return;
        }
        if unsafe { SetPropW(self.window, BLURRED, 1 as _) } == 0 {
            log::error!("blur: the window's blurred mark does not set");
        }
        if !unsafe { GetPropW(self.window, MOVING) }.is_null() {
            match accent(self.window, CLEAR) {
                Ok(()) => log::debug!("blur: blurred mid-move, the window moves clear"),
                Err(error) => log::error!("{error:#}"),
            }
        }
    }
}

/// The window's procedure: a blurred window clears its acrylic while it moves; all else is gpui's.
unsafe extern "system" fn moving(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let gpui: WNDPROC = unsafe { std::mem::transmute(GPUI.load(Ordering::Relaxed)) };
    let blurred = !unsafe { GetPropW(window, BLURRED) }.is_null();
    match message {
        WM_ENTERSIZEMOVE => {
            if unsafe { SetPropW(window, MOVING, 1 as _) } == 0 {
                log::error!("blur: the window's moving mark does not set");
            }
            if blurred {
                match accent(window, CLEAR) {
                    Ok(()) => log::debug!("blur: the window moves clear"),
                    Err(error) => log::error!("{error:#}"),
                }
            }
        }
        WM_EXITSIZEMOVE => {
            unsafe { RemovePropW(window, MOVING) };
            if blurred {
                match accent(window, ACRYLIC) {
                    Ok(()) => log::debug!("blur: the window rests blurred again"),
                    Err(error) => log::error!("{error:#}"),
                }
                let flags =
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED;
                if unsafe { SetWindowPos(window, std::ptr::null_mut(), 0, 0, 0, 0, flags) } == 0 {
                    log::error!("blur: DWM hears of no new frame after the move");
                }
            }
        }
        WM_NCDESTROY => {
            unsafe { RemovePropW(window, BLURRED) };
            unsafe { RemovePropW(window, MOVING) };
        }
        _ => {}
    }
    unsafe { CallWindowProcW(gpui, window, message, wparam, lparam) }
}

#[repr(C)]
struct AccentPolicy {
    state: u32,
    flags: u32,
    color: u32,
    animation: u32,
}

#[repr(C)]
struct CompositionData {
    attribute: u32,
    data: *mut AccentPolicy,
    size: usize,
}

/// Sets the window's accent as gpui's `set_window_composition_attribute` sets it.
fn accent(window: HWND, state: u32) -> anyhow::Result<()> {
    type Set = unsafe extern "system" fn(HWND, *mut CompositionData) -> BOOL;
    let user32 = unsafe { GetModuleHandleW(w!("user32.dll")) };
    anyhow::ensure!(!user32.is_null(), "blur: user32.dll is not loaded");
    let found = unsafe { GetProcAddress(user32, s!("SetWindowCompositionAttribute")) }
        .context("blur: user32.dll has no SetWindowCompositionAttribute")?;
    let set: Set = unsafe { std::mem::transmute(found) };
    let acrylic = state == ACRYLIC;
    let mut policy = AccentPolicy {
        state,
        flags: if acrylic { 0 } else { 2 },
        color: if acrylic { 1 << 24 } else { 0 },
        animation: 0,
    };
    let mut data = CompositionData {
        attribute: 0x13,
        data: &mut policy,
        size: size_of::<AccentPolicy>(),
    };
    anyhow::ensure!(
        unsafe { set(window, &mut data) } != 0,
        "blur: the window's accent {state} does not set"
    );
    Ok(())
}

/// Whether this build draws Mica; read once, logged.
pub fn mica() -> bool {
    static MICA_DRAWN: OnceLock<bool> = OnceLock::new();
    *MICA_DRAWN.get_or_init(|| match build() {
        Ok(build) => {
            let drawn = build >= MICA;
            log::info!("blur: Windows build {build} draws Mica: {drawn}; else Backdrop is Acrylic");
            drawn
        }
        Err(error) => {
            log::error!("{error:#}; Backdrop is Acrylic");
            false
        }
    })
}

/// The running Windows' build.
fn build() -> anyhow::Result<u32> {
    let mut version: OSVERSIONINFOW = unsafe { std::mem::zeroed() };
    version.dwOSVersionInfoSize = size_of::<OSVERSIONINFOW>() as u32;
    let status = unsafe { RtlGetVersion(&mut version) };
    anyhow::ensure!(status == 0, "blur: RtlGetVersion fails with {status:#x}");
    Ok(version.dwBuildNumber)
}

/// The HWND gpui made for `window`.
fn native(window: &Window) -> anyhow::Result<HWND> {
    let handle = HasWindowHandle::window_handle(window)
        .map_err(|error| anyhow::anyhow!("blur: no native handle: {error:?}"))?;
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        bail!("blur: not a Win32 window");
    };
    Ok(win32.hwnd.get() as HWND)
}
