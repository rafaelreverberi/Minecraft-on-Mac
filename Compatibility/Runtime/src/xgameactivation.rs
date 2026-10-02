use std::ffi::{c_char, c_void};

use windows_core::{BOOL, HRESULT, IUnknown, interface};

use crate::threading::{XTaskQueueHandle, XTaskQueueRegistrationToken};

// XGameActivationCallback
pub type XGameActivationCallback = unsafe extern "system" fn(
    _context: *mut c_void,
    _activation_info: *const XGameActivationInfo,
) -> ();

#[repr(u32)]
pub enum XGameActivationType {
    Protocol = 0,
    File = 1,
    PendingGameInvite = 2,
    AcceptedGameInvite = 3,
}

pub struct XGameActivationInfo {
    pub type_: XGameActivationType,
    pub value: *const c_char,
}

// Class _GUID_7f0fe8b8_e075_49ab_9aa7_a1e065489a9e
// IID _GUID_7f0fe8b8_e075_49ab_9aa7_a1e065489a9e
#[interface("7f0fe8b8-e075-49ab-9aa7-a1e065489a9e")]
pub unsafe trait IXGameActivation: IUnknown {
    // XGameActivationRegisterForEvent
    pub unsafe fn x_game_activation_register_for_event(
        self: &Self,
        _queue: XTaskQueueHandle,
        _context: *mut c_void,
        _callback: Option<XGameActivationCallback>,
        _token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT;
    // XGameActivationUnregisterForEvent
    pub unsafe fn x_game_activation_unregister_for_event(
        self: &Self,
        _token: XTaskQueueRegistrationToken,
        _wait: BOOL,
    ) -> BOOL;
    // XGameActivationAcceptPendingInvite
    pub unsafe fn x_game_activation_accept_pending_invite(
        self: &Self,
        _invite_uri: *const c_char,
    ) -> HRESULT;
}
