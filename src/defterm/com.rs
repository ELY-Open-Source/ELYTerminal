//! ELYTerminal's class for COM: OpenConsole hands it each console session, `ITerminalHandoff3`.

use std::{
    ffi::c_void,
    os::windows::io::{FromRawHandle, OwnedHandle},
    sync::{OnceLock, mpsc},
    thread,
};

use alacritty_terminal::tty;
use anyhow::Context as _;
use futures::channel::mpsc::UnboundedSender;
use windows_sys::{
    Win32::{
        Foundation::{
            CLASS_E_NOAGGREGATION, CloseHandle, DUPLICATE_SAME_ACCESS, DuplicateHandle, E_FAIL,
            E_NOINTERFACE, HANDLE, S_OK, SysStringLen,
        },
        System::{
            Com::{
                CLSCTX_LOCAL_SERVER, COINIT_MULTITHREADED, CoInitializeEx, CoRegisterClassObject,
                REGCLS_MULTIPLEUSE,
            },
            Threading::{
                GetCurrentProcess, GetProcessId, PROCESS_NAME_WIN32, QueryFullProcessImageNameW,
            },
        },
    },
    core::{BSTR, GUID, HRESULT, IID_IUnknown, IUnknown_Vtbl},
};

/// A session OpenConsole handed over, and the title it gave.
pub struct Handoff {
    pub pty: tty::Pty,
    /// The program that started the console, held so its id stays the session's own.
    pub launcher: OwnedHandle,
    pub pid: u32,
    /// Its executable, when Windows still names it.
    pub name: Option<String>,
    pub title: String,
}

const CLASS: GUID = GUID::from_u128(0x79b54338_a0fa_474f_b304_4932b36b6f55);
const IID_ICLASSFACTORY: GUID = GUID::from_u128(0x00000001_0000_0000_c000_000000000046);
const IID_ITERMINALHANDOFF3: GUID = GUID::from_u128(0x6f23da90_15c5_4203_9db0_64e73f1b1b00);

/// Where each handoff goes: the app's main thread.
static SENDER: OnceLock<UnboundedSender<Handoff>> = OnceLock::new();

/// Windows Terminal's `TERMINAL_STARTUP_INFO`.
#[repr(C)]
struct StartupInfo {
    title: BSTR,
    icon_path: BSTR,
    icon_index: i32,
    place: [u32; 8],
    show_window: u16,
}

#[repr(C)]
struct FactoryVtbl {
    unknown: IUnknown_Vtbl,
    create: unsafe extern "system" fn(
        *mut c_void,
        *mut c_void,
        *const GUID,
        *mut *mut c_void,
    ) -> HRESULT,
    lock: unsafe extern "system" fn(*mut c_void, i32) -> HRESULT,
}

#[repr(C)]
struct HandoffVtbl {
    unknown: IUnknown_Vtbl,
    #[allow(clippy::type_complexity)]
    establish: unsafe extern "system" fn(
        *mut c_void,
        *mut HANDLE,
        *mut HANDLE,
        HANDLE,
        HANDLE,
        HANDLE,
        HANDLE,
        *const StartupInfo,
    ) -> HRESULT,
}

/// A COM object that lives as long as the process, so it counts no references.
#[repr(C)]
struct Object<V: 'static> {
    vtable: &'static V,
}

static FACTORY: Object<FactoryVtbl> = Object {
    vtable: &FactoryVtbl {
        unknown: IUnknown_Vtbl {
            QueryInterface: factory_query,
            AddRef: kept,
            Release: kept,
        },
        create,
        lock,
    },
};

static HANDOFF: Object<HandoffVtbl> = Object {
    vtable: &HandoffVtbl {
        unknown: IUnknown_Vtbl {
            QueryInterface: handoff_query,
            AddRef: kept,
            Release: kept,
        },
        establish,
    },
};

fn same(a: &GUID, b: &GUID) -> bool {
    (a.data1, a.data2, a.data3, a.data4) == (b.data1, b.data2, b.data3, b.data4)
}

/// Gives `object` as `out` when `iid` is one of `served`.
unsafe fn answer<V>(
    object: &'static Object<V>,
    served: [GUID; 2],
    iid: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    if out.is_null() || iid.is_null() {
        return E_FAIL;
    }
    // SAFETY: COM hands valid pointers, checked for null above.
    unsafe {
        if served.iter().any(|known| same(known, &*iid)) {
            *out = object as *const Object<V> as *mut c_void;
            S_OK
        } else {
            *out = std::ptr::null_mut();
            E_NOINTERFACE
        }
    }
}

unsafe extern "system" fn kept(_: *mut c_void) -> u32 {
    1
}

unsafe extern "system" fn factory_query(
    _: *mut c_void,
    iid: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    // SAFETY: COM's own pointers.
    unsafe { answer(&FACTORY, [IID_IUnknown, IID_ICLASSFACTORY], iid, out) }
}

unsafe extern "system" fn handoff_query(
    _: *mut c_void,
    iid: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    // SAFETY: COM's own pointers.
    unsafe { answer(&HANDOFF, [IID_IUnknown, IID_ITERMINALHANDOFF3], iid, out) }
}

unsafe extern "system" fn create(
    _: *mut c_void,
    outer: *mut c_void,
    iid: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    if !outer.is_null() {
        return CLASS_E_NOAGGREGATION;
    }
    log::info!("defterm: OpenConsole asks for the terminal");
    // SAFETY: COM's own pointers.
    unsafe { handoff_query(std::ptr::null_mut(), iid, out) }
}

unsafe extern "system" fn lock(_: *mut c_void, _: i32) -> HRESULT {
    S_OK
}

#[allow(clippy::too_many_arguments)]
unsafe extern "system" fn establish(
    _: *mut c_void,
    input: *mut HANDLE,
    output: *mut HANDLE,
    signal: HANDLE,
    reference: HANDLE,
    server: HANDLE,
    client: HANDLE,
    startup: *const StartupInfo,
) -> HRESULT {
    // SAFETY: COM's own pointers and handles, live for the call.
    match unsafe { take(input, output, [signal, reference, server, client], startup) } {
        Ok(()) => S_OK,
        Err(error) => {
            log::error!("{error:#}");
            E_FAIL
        }
    }
}

/// Our own copy of `handle`: COM closes the one it lent once the call returns.
fn duplicate(handle: HANDLE) -> anyhow::Result<HANDLE> {
    let mut copy = std::ptr::null_mut();
    // SAFETY: `handle` is live for the call; `copy` is written.
    let done = unsafe {
        let process = GetCurrentProcess();
        DuplicateHandle(
            process,
            handle,
            process,
            &mut copy,
            0,
            0,
            DUPLICATE_SAME_ACCESS,
        )
    };
    anyhow::ensure!(
        done != 0,
        "defterm: a handed handle does not copy: {}",
        std::io::Error::last_os_error()
    );
    Ok(copy)
}

unsafe fn take(
    input: *mut HANDLE,
    output: *mut HANDLE,
    lent: [HANDLE; 4],
    startup: *const StartupInfo,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        !input.is_null() && !output.is_null() && !startup.is_null(),
        "defterm: a handoff without its pipes or startup info"
    );
    // SAFETY: checked for null; a BSTR holds its length.
    let title = unsafe {
        let title = (*startup).title;
        if title.is_null() {
            String::new()
        } else {
            String::from_utf16_lossy(std::slice::from_raw_parts(
                title,
                SysStringLen(title) as usize,
            ))
        }
    };
    let [signal, reference, server, client] = lent;
    // SAFETY: COM lends a live process handle for the call.
    let pid = unsafe { GetProcessId(client) };
    anyhow::ensure!(
        pid != 0,
        "defterm: the handed program has no id: {}",
        std::io::Error::last_os_error()
    );
    let launcher = unsafe { OwnedHandle::from_raw_handle(duplicate(client)? as _) };
    let name = image(client, pid);
    let mut owned = Vec::with_capacity(3);
    for handle in [server, reference, signal] {
        match duplicate(handle) {
            Ok(copy) => owned.push(copy),
            Err(error) => {
                // SAFETY: each copy is ours and given to no one.
                owned.into_iter().for_each(|copy| unsafe {
                    CloseHandle(copy);
                });
                return Err(error);
            }
        }
    }
    let (pty, conin, conout) = tty::handoff(owned[0], owned[1], owned[2])
        .context("defterm: the handed session does not open")?;
    let sender = SENDER.get().context("defterm: no app takes the handoff")?;
    if let Err(error) = sender.unbounded_send(Handoff {
        pty,
        launcher,
        pid,
        name,
        title: title.clone(),
    }) {
        // SAFETY: both are ours and given to no one.
        unsafe {
            CloseHandle(conin);
            CloseHandle(conout);
        }
        anyhow::bail!("defterm: the app is gone: {error}");
    }
    // SAFETY: checked for null; COM takes both handles on.
    unsafe {
        *input = conin;
        *output = conout;
    }
    log::info!("defterm: process {pid}'s console session handed over, titled {title:?}");
    Ok(())
}

/// The executable `process` runs; none once Windows no longer names it, logged.
fn image(process: HANDLE, pid: u32) -> Option<String> {
    let mut path = [0u16; 1024];
    let mut length = path.len() as u32;
    // SAFETY: `path` holds `length` units; the handle is live for the call.
    let named = unsafe {
        QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, path.as_mut_ptr(), &mut length)
    };
    if named == 0 {
        log::warn!(
            "defterm: process {pid} names no executable: {}",
            std::io::Error::last_os_error()
        );
        return None;
    }
    Some(String::from_utf16_lossy(&path[..length as usize]))
}

/// Serves the class from a thread of COM's free apartment, so calls need no message loop.
pub fn register(sender: UnboundedSender<Handoff>) -> anyhow::Result<()> {
    SENDER
        .set(sender)
        .map_err(|_| anyhow::anyhow!("defterm: the class is served already"))?;
    let (done, result) = mpsc::channel();
    thread::Builder::new()
        .name("defterm".into())
        .spawn(move || {
            // SAFETY: this thread joins the free apartment and stays in it.
            let joined = unsafe { CoInitializeEx(std::ptr::null(), COINIT_MULTITHREADED as u32) };
            if joined < 0 {
                let _ = done.send(Err(format!("CoInitializeEx: {joined:#x}")));
                return;
            }
            let mut cookie = 0u32;
            // SAFETY: FACTORY lives for the process.
            let registered = unsafe {
                CoRegisterClassObject(
                    &CLASS,
                    &FACTORY as *const Object<FactoryVtbl> as *mut c_void,
                    CLSCTX_LOCAL_SERVER,
                    REGCLS_MULTIPLEUSE as u32,
                    &mut cookie,
                )
            };
            if registered < 0 {
                let _ = done.send(Err(format!("CoRegisterClassObject: {registered:#x}")));
                return;
            }
            let _ = done.send(Ok(()));
            // The apartment lives while the thread does.
            loop {
                thread::park();
            }
        })
        .context("defterm: no thread for COM")?;
    result
        .recv()
        .context("defterm: the COM thread ended")?
        .map_err(|error| anyhow::anyhow!("defterm: COM refused the class: {error}"))?;
    log::info!("defterm: serving the terminal class to OpenConsole");
    Ok(())
}
