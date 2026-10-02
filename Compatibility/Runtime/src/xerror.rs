use std::ffi::{c_char, c_void};

use windows_core::{BOOL, HRESULT, IUnknown, interface};

// XErrorCallback
pub type XErrorCallback =
    unsafe extern "system" fn(_hr: HRESULT, _msg: *const c_char, _context: *mut c_void) -> BOOL;

#[repr(u32)]
pub enum XErrorOptions {
    None = 0x00,
    OutputDebugStringOnError = 0x01,
    DebugBreakOnError = 0x02,
    FailFastOnError = 0x04,
}

// Class _GUID_8ca467f7_22e8_4096_8456_bb8aa13f79d8
// IID _GUID_8ca467f7_22e8_4096_8456_bb8aa13f79d8
#[interface("8ca467f7-22e8-4096-8456-bb8aa13f79d8")]
pub unsafe trait IXError: IUnknown {
    pub unsafe fn __reserved_slot_3(&self);
    // XErrorSetCallback
    pub unsafe fn x_error_set_callback(
        self: &Self,
        _callback: Option<XErrorCallback>,
        _context: *mut c_void,
    ) -> ();
    // XErrorSetOptions
    pub unsafe fn x_error_set_options(
        self: &Self,
        _options_debugger_present: XErrorOptions,
        _options_debugger_not_present: XErrorOptions,
    ) -> ();
}
