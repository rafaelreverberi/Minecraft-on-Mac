use std::ffi::{c_char, c_void};

use windows_core::{BOOL, HRESULT, IUnknown, interface};

use crate::threading::{XTaskQueueHandle, XTaskQueueRegistrationToken};

// XGameInviteEventCallback
pub type XGameInviteEventCallback =
    unsafe extern "system" fn(_context: *mut c_void, _invite_uri: *const c_char) -> ();

// Class _GUID_0651aae2_4012_4077_bf84_8b9097090e2c
// IID _GUID_0651aae2_4012_4077_bf84_8b9097090e2c
#[interface("0651aae2-4012-4077-bf84-8b9097090e2c")]
pub unsafe trait IXGameInvite: IUnknown {
    // XGameInviteRegisterForEvent
    pub unsafe fn x_game_invite_register_for_event(
        self: &Self,
        _queue: XTaskQueueHandle,
        _context: *mut c_void,
        _callback: Option<XGameInviteEventCallback>,
        _token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT;
    // XGameInviteUnregisterForEvent
    pub unsafe fn x_game_invite_unregister_for_event(
        self: &Self,
        _token: XTaskQueueRegistrationToken,
        _wait: BOOL,
    ) -> BOOL;
    // XGameInviteRegisterForPendingEvent
    pub unsafe fn __reserved_slot_5(&self);
    // XGameInviteUnregisterForPendingEvent
    pub unsafe fn __reserved_slot_6(&self);
    // XGameInviteAcceptPendingInvite
    pub unsafe fn __reserved_slot_7(&self);
}
