use std::ffi::{c_char, c_void};

use windows_core::{BOOL, HRESULT, IUnknown, interface};

type XSystemHandle = i64;

#[repr(C)]
pub enum XSystemHandleType {
    AppCaptureScreenshotStream = 0x00,
    DisplayTimeoutDeferral = 0x01,
    GameSaveContainer = 0x02,
    GameSaveProvider = 0x03,
    GameSaveUpdate = 0x04,
    PackageInstallationMonitor = 0x05,
    PackageMount = 0x06,
    SpeechSynthesizer = 0x07,
    SpeechSynthesizerStream = 0x08,
    StoreContext = 0x09,
    StoreLicense = 0x0a,
    StoreProductQuery = 0x0b,
    TaskQueue = 0x0c,
    User = 0x0d,
    UserSignOutDeferral = 0x0e,
    GameUiTextEntry = 0x0f,
}

#[repr(C)]
pub enum XSystemHandleCallbackReason {
    Created = 0x00,
    Destroyed = 0x01,
}

pub type XSystemHandleCallback =
    extern "system" fn(XSystemHandle, XSystemHandleType, XSystemHandleCallbackReason, *mut c_void);

// Class _GUID_e349bd1a_fc20_4e40_b99c_4178cc6b409f
// IID _GUID_e349bd1a_fc20_4e40_b99c_4178cc6b409f
#[interface("e349bd1a-fc20-4e40-b99c-4178cc6b409f")]
pub unsafe trait IXSystem: IUnknown {
    // XSystemGetConsoleId
    pub unsafe fn x_system_get_console_id(
        self: &Self,
        _console_id_size: usize,
        _console_id: *mut c_char,
        _console_id_used: *mut usize,
    ) -> HRESULT;
    // XSystemGetXboxLiveSandboxId
    pub unsafe fn x_system_get_xbox_live_sandbox_id(
        self: &Self,
        _sandbox_id_size: usize,
        _sandbox_id: *mut c_char,
        _sandbox_id_used: *mut usize,
    ) -> HRESULT;
    // XSystemGetAppSpecificDeviceId
    pub unsafe fn x_system_get_app_specific_device_id(
        self: &Self,
        _app_specific_device_id_size: usize,
        _app_specific_device_id: *mut c_char,
        _app_specific_device_id_used: *mut usize,
    ) -> HRESULT;
    // XSystemHandleTrack
    pub unsafe fn x_system_handle_track(
        self: &Self,
        _callback: XSystemHandleCallback,
        _context: *mut c_void,
    ) -> HRESULT;
    // XSystemIsHandleValid
    pub unsafe fn x_system_is_handle_valid(self: &Self, _handle: XSystemHandle) -> BOOL;
    // XSystemAllowFullDownloadBandwidth
    pub unsafe fn x_system_allow_full_download_bandwidth(self: &Self, _enable: BOOL) -> ();
}

#[interface("6fd71f09-7513-49f0-89bc-bfaf5df6f852")]
pub unsafe trait IXSystem2: IXSystem {}

#[interface("67ce4bfc-b1d1-4ac7-bc3a-cb9219a97a85")]
pub unsafe trait IXSystem3: IXSystem2 {}

#[interface("dadc2895-34b0-4ef5-a83e-45114d629b80")]
pub unsafe trait IXSystem4: IXSystem3 {}

#[interface("1861cf2e-e18b-4834-a9f5-b4a4e6efb4cf")]
pub unsafe trait IXSystem5: IXSystem4 {}
