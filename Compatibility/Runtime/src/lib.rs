use std::ffi::{CStr, CString, c_char, c_void};
use std::ptr::null_mut;
use std::result::Result;
use std::sync::{Mutex, Once};

use windows::minwindef::LPARAM;
use windows::windef::HWND;
use windows::winuser::{EnumWindows, MB_OK, MessageBoxW};

use crate::user::{IXUser3, XUserPlatformRemoteConnectEventHandler};
use windows_core::{GUID, HRESULT, Interface};
use windows_sys::libloaderapi::{FreeLibrary, GetProcAddress, LoadLibraryA};
use windows_sys::minwindef::HMODULE;

#[cfg(feature = "xuser")]
mod authenticator;
mod com;
pub mod stub;
pub mod results;
pub mod threading;
pub mod user;
pub mod xasync;
pub mod xbox_utils;
pub mod xerror;
pub mod xgameactivation;
pub mod xgameinvite;
pub mod xgameprotocol;
pub mod xgamesave;
pub mod xgamestreaming;
pub mod xlaunch;
pub mod xnetworking;
pub mod xpackage;
pub mod xpersistedlocalstorage;
pub mod xstore;
pub mod xsystem;

type Ulong = u32;
type Char = i8;

type Lpcstr = *const c_char;

pub(crate) fn diag(_msg: &str) {}

const S_OK: HRESULT = HRESULT(0);
const E_FAIL: HRESULT = HRESULT(0x80004005u32 as i32);
const E_NOTIMPL: HRESULT = HRESULT(0x80004001u32 as i32);

#[repr(C)]
pub struct InitializeOptions;

type InitializeApiImplEx2Fn =
    unsafe extern "system" fn(Ulong, Ulong, Char, *mut InitializeOptions) -> HRESULT;
type QueryApiImplFn =
    unsafe extern "system" fn(*const GUID, *const GUID, *mut *mut c_void) -> HRESULT;
type UninitializeApiImplFn = unsafe extern "system" fn() -> HRESULT;

struct DelegatedApi {
    module: HMODULE,
    initialize_api_impl_ex2: InitializeApiImplEx2Fn,
    query_api_impl: QueryApiImplFn,
    uninitialize_api_impl: UninitializeApiImplFn,
}

unsafe impl Send for DelegatedApi {}

struct DelegatedApiState {
    ref_count: usize,
    api: Option<DelegatedApi>,
}

static DELEGATED_API_STATE: Mutex<DelegatedApiState> = Mutex::new(DelegatedApiState {
    ref_count: 0,
    api: None,
});

#[cfg(test)]
static TEST_DELEGATED_DLL_PATH: Mutex<Option<CString>> = Mutex::new(None);

fn delegated_state() -> std::sync::MutexGuard<'static, DelegatedApiState> {
    DELEGATED_API_STATE
        .lock()
        .expect("delegated xgameruntime state poisoned")
}

#[cfg(test)]
fn delegated_dll_name() -> CString {
    TEST_DELEGATED_DLL_PATH
        .lock()
        .expect("delegated xgameruntime test path poisoned")
        .clone()
        .unwrap_or_else(|| CString::new("xgameruntime.gdk.dll").expect("static dll name"))
}

#[cfg(not(test))]
fn delegated_dll_name() -> CString {
    CString::new("xgameruntime.gdk.dll").expect("static dll name")
}

#[cfg(test)]
pub fn set_delegated_dll_path_for_test(path: Option<&str>) {
    let mut slot = TEST_DELEGATED_DLL_PATH
        .lock()
        .expect("delegated xgameruntime test path poisoned");
    *slot = path.map(|path| CString::new(path).expect("dll path contains interior NUL"));
}

unsafe fn load_symbol<T>(module: HMODULE, symbol: &'static [u8]) -> Result<T, HRESULT>
where
    T: Copy,
{
    let proc = unsafe { GetProcAddress(module, symbol.as_ptr()) };
    if let Some(proc) = proc {
        Ok(unsafe { std::mem::transmute_copy(&proc) })
    } else {
        Err(E_FAIL)
    }
}

unsafe extern "system" fn find_window(hwnd: HWND, lp: LPARAM) -> windows_core::BOOL {
    unsafe {
        let result: &mut HWND = &mut *(lp.0 as *mut HWND);
        *result = hwnd;
    }
    return false.into();
}

unsafe extern "system" fn show(
    _context: *const c_void,
    _user_identifierr: u32,
    _operation: u32,
    url: *const c_char,
    code: *const c_char,
    _qr_code_size: usize,
    _qr_code: *const c_char,
) {
    unsafe {
        let url = CStr::from_ptr(url);
        let code = CStr::from_ptr(code);
        let mut search: HWND = HWND(null_mut());
        _ = EnumWindows(
            Some(find_window),
            LPARAM((&mut search as *mut HWND) as isize),
        );
        MessageBoxW(
            if search.0.is_null() {
                None
            } else {
                Some(search)
            },
            windows_strings::PCWSTR::from_raw(
                windows::core::HSTRING::from(format!(
                    "{} {}",
                    url.to_string_lossy(),
                    code.to_string_lossy()
                ))
                .as_ptr(),
            ),
            windows::core::h!("Xbox Live Remote Login"),
            MB_OK,
        );
    }
}

unsafe extern "system" fn hide() {}

unsafe fn load_delegated_api() -> Result<DelegatedApi, HRESULT> {
    let dll_name = delegated_dll_name();
    let module = unsafe { LoadLibraryA(dll_name.as_ptr().cast()) };
    if module.is_null() {
        return Err(E_FAIL);
    }

    let initialize_api_impl_ex2 =
        match unsafe { load_symbol::<InitializeApiImplEx2Fn>(module, b"InitializeApiImplEx2\0") } {
            Ok(symbol) => symbol,
            Err(error) => {
                unsafe {
                    FreeLibrary(module);
                }
                return Err(error);
            }
        };
    let query_api_impl = match unsafe { load_symbol::<QueryApiImplFn>(module, b"QueryApiImpl\0") } {
        Ok(symbol) => symbol,
        Err(error) => {
            unsafe {
                FreeLibrary(module);
            }
            return Err(error);
        }
    };
    let uninitialize_api_impl =
        match unsafe { load_symbol::<UninitializeApiImplFn>(module, b"UninitializeApiImpl\0") } {
            Ok(symbol) => symbol,
            Err(error) => {
                unsafe {
                    FreeLibrary(module);
                }
                return Err(error);
            }
        };

    Ok(DelegatedApi {
        module,
        initialize_api_impl_ex2,
        query_api_impl,
        uninitialize_api_impl,
    })
}

static START: Once = Once::new();

fn initialize_delegate(
    gdk_ver: Ulong,
    gs_ver: Ulong,
    mode: Char,
    options: *mut InitializeOptions,
) -> HRESULT {
    diag("initialize_delegate entered");
    let mut state = delegated_state();
    if state.ref_count > 0 {
        state.ref_count += 1;
        return S_OK;
    }
    START.call_once(|| {
        rustls::crypto::ring::default_provider()
            .install_default().unwrap();
            // Production launcher discards raw runtime diagnostics. No token-bearing logger.
    });
    println!("Loading delegated API...");

    let api = match unsafe { load_delegated_api() } {
        Ok(api) => {
            diag("delegate load OK");
            api
        },
        Err(error) => {
            diag(&format!("delegate load FAILED hr={:#X}", error.0));
            println!("Failed to load delegated API: {:#X}", error.0);
            return error;
        }
    };

    println!("Initializing delegated API...");

    let hr = unsafe {
        (api.initialize_api_impl_ex2)(gdk_ver, gs_ver, mode | 8 /* xplat mode */, options)
    };
    diag(&format!("delegate init hr={:#X}", hr.0));
    if hr != S_OK {
        println!("Failed to initialize delegated API: {:#X}", hr.0);
        // panic!("Failed to initialize delegated API");
        // unsafe {
        //     FreeLibrary(api.module);
        // }
        // return hr;
    }

    println!("Delegated API initialized successfully.");

    // let mut out: *mut c_void = std::ptr::null_mut();

    // let xuserguid = GUID::from_u128(0x01acd177_91f9_4763_a38e_ccbb55ce32e0);

    // let hr = unsafe { (api.query_api_impl)(&xuserguid, &IXUser3::IID, &mut out) };

    // assert_eq!(hr, HRESULT(0));
    // assert!(!out.is_null());

    // if let Some(platform) = unsafe { IXUser3::from_raw_borrowed(&out) } {
    //     let mut callback = XUserPlatformRemoteConnectEventHandler {
    //         show: Some(show),
    //         close: Some(hide),
    //         context: std::ptr::null_mut(),
    //     };
    //     let hr = unsafe {
    //         platform.x_user_platform_remote_connect_set_event_handlers(
    //             std::ptr::null_mut(),
    //             &mut callback,
    //         )
    //     };
    //     assert_eq!(hr, HRESULT(0));
    // }

    state.ref_count = 1;
    state.api = Some(api);
    diag("initialize_delegate finished");
    S_OK
}

pub fn delegated_query_api_impl(
    runtime_class_id: *const GUID,
    interface_id: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    let state = delegated_state();
    let Some(api) = state.api.as_ref() else {
        unsafe {
            *out = std::ptr::null_mut();
        }
        return E_NOTIMPL;
    };

    unsafe { (api.query_api_impl)(runtime_class_id, interface_id, out) }
}

fn uninitialize_delegate() -> HRESULT {
    let mut state = delegated_state();
    if state.ref_count == 0 {
        return E_NOTIMPL;
    }

    state.ref_count -= 1;
    if state.ref_count > 0 {
        return S_OK;
    }

    let Some(api) = state.api.take() else {
        return E_FAIL;
    };

    let hr = unsafe { (api.uninitialize_api_impl)() };
    unsafe {
        FreeLibrary(api.module);
    }
    hr
}

#[unsafe(no_mangle)]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    S_OK
}

#[unsafe(no_mangle)]
pub extern "system" fn InitializeApiImplEx2(
    gdk_ver: Ulong,
    gs_ver: Ulong,
    mode: Char,
    options: *mut InitializeOptions,
) -> HRESULT {
    initialize_delegate(gdk_ver, gs_ver, mode, options)
}

#[unsafe(no_mangle)]
pub extern "system" fn InitializeApiImplEx(gdk_ver: Ulong, gs_ver: Ulong, mode: Char) -> HRESULT {
    InitializeApiImplEx2(gdk_ver, gs_ver, mode, std::ptr::null_mut())
}

#[unsafe(no_mangle)]
pub extern "system" fn InitializeApiImpl(gdk_ver: Ulong, gs_ver: Ulong) -> HRESULT {
    InitializeApiImplEx(gdk_ver, gs_ver, 0)
}

#[unsafe(no_mangle)]
pub extern "system" fn QueryApiImpl(
    runtime_class_id: *const GUID,
    interface_id: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    com::query_api_impl(runtime_class_id, interface_id, out)
}

#[unsafe(no_mangle)]
pub extern "system" fn UninitializeApiImpl() -> HRESULT {
    uninitialize_delegate()
}

#[unsafe(no_mangle)]
pub extern "system" fn XErrorReport(_status: HRESULT, _message: Lpcstr) -> HRESULT {
    E_NOTIMPL
}
