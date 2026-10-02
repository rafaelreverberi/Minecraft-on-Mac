use std::ffi::{c_char, c_void};

use windows_core::{BOOL, IUnknown, interface};

use crate::threading::{XTaskQueueHandle, XTaskQueueRegistrationToken};

// XGameProtocolActivationCallback
pub type XGameProtocolActivationCallback =
    unsafe extern "system" fn(_context: *mut c_void, _protocol_uri: *const c_char) -> ();

// Class _GUID_95fd18d2_74dd_4d7c_aa1b_0b51827665d6
// IID _GUID_95fd18d2_74dd_4d7c_aa1b_0b51827665d6
#[interface("95fd18d2-74dd-4d7c-aa1b-0b51827665d6")]
pub unsafe trait IXGameProtocol: IUnknown {
    // XGameProtocolRegisterForActivation
    pub unsafe fn x_game_protocol_register_for_activation(
        self: &Self,
        _queue: XTaskQueueHandle,
        _context: *mut c_void,
        _callback: *mut XGameProtocolActivationCallback,
        _token: *mut XTaskQueueRegistrationToken,
    ) -> ();
    // XGameProtocolUnregisterForActivation
    pub unsafe fn x_game_protocol_unregister_for_activation(
        self: &Self,
        _token: XTaskQueueRegistrationToken,
        _wait: BOOL,
    ) -> BOOL;
}
