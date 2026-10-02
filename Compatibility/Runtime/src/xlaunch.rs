use std::ffi::c_char;

use windows_core::{HRESULT, IUnknown, interface};

use crate::user::XUserHandle;

// Class _GUID_973a344e_24bf_4d0f_8457_56c534892b29
// IID _GUID_973a344e_24bf_4d0f_8457_56c534892b29
#[interface("973a344e-24bf-4d0f-8457-56c534892b29")]
pub unsafe trait IXLaunch: IUnknown {
    // XGameGetXboxTitleId
    pub unsafe fn x_game_get_xbox_title_id(self: &Self, _title_id: *mut u32) -> HRESULT;
    // XLaunchNewGame
    pub unsafe fn x_launch_new_game(
        self: &Self,
        _exe_path: *const c_char,
        _args: *const c_char,
        _default_user: XUserHandle,
    ) -> ();
    // XLaunchRestartOnCrash
    pub unsafe fn x_launch_restart_on_crash(
        self: &Self,
        _args: *const c_char,
        _reserved: u32,
    ) -> HRESULT;
}

#[interface("2549F142-6419-4A06-97B5-931AAB7C2F34")]
pub unsafe trait IXLaunch2: IXLaunch {}

#[interface("50849859-0AD8-4F81-80E4-5BC78626F852")]
pub unsafe trait IXLaunch3: IXLaunch2 {}
