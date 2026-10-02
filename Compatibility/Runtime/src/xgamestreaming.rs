use std::ffi::{c_char, c_void};

use windows_core::{BOOL, HRESULT, IUnknown, interface};

use crate::{
    threading::{XTaskQueueHandle, XTaskQueueRegistrationToken},
    xpackage::XVersion,
};

pub type XGameStreamingClientId = u64;

// XGameStreamingClientPropertiesChangedCallback
pub type XGameStreamingClientPropertiesChangedCallback = unsafe extern "system" fn(
    _context: *mut c_void,
    _client: XGameStreamingClientId,
    _updated_properties_count: u32,
    _updated_properties: *mut XGameStreamingClientProperty,
) -> ();
// XGameStreamingConnectionStateChangedCallback
pub type XGameStreamingConnectionStateChangedCallback = unsafe extern "system" fn(
    _context: *mut c_void,
    _client: XGameStreamingClientId,
    _state: XGameStreamingConnectionState,
) -> ();

pub struct D3d12xboxFramePipelineToken;
pub struct IGameInputReading;
pub struct RECT;

#[repr(u32)]
pub enum XGameStreamingClientProperty {
    None = 0,
    StreamPhysicalDimensions = 1,
    TouchInputEnabled = 2,
    TouchBundleVersion = 4,
    IPAddress = 5,
    SessionId = 6,
    DisplayDetails = 7,
}
#[repr(u32)]
pub enum XGameStreamingConnectionState {
    Disconnected = 0,
    Connected = 1,
}
#[repr(u64)]
pub enum XGameStreamingGamepadPhysicality {
    None = 0x0000000000000000,
    DPadUpPhysical = 0x0000000000000001,
    DPadDownPhysical = 0x0000000000000002,
    DPadLeftPhysical = 0x0000000000000004,
    DPadRightPhysical = 0x0000000000000008,
    MenuPhysical = 0x0000000000000010,
    ViewPhysical = 0x0000000000000020,
    LeftThumbstickPhysical = 0x0000000000000040,
    RightThumbstickPhysical = 0x0000000000000080,
    LeftShoulderPhysical = 0x0000000000000100,
    RightShoulderPhysical = 0x0000000000000200,
    APhysical = 0x0000000000001000,
    BPhysical = 0x0000000000002000,
    XPhysical = 0x0000000000004000,
    YPhysical = 0x0000000000008000,
    LeftTriggerPhysical = 0x0000000000010000,
    RightTriggerPhysical = 0x0000000000020000,
    LeftThumbstickXPhysical = 0x0000000000040000,
    LeftThumbstickYPhysical = 0x0000000000080000,
    RightThumbstickXPhysical = 0x0000000000100000,
    RightThumbstickYPhysical = 0x0000000000200000,
    ButtonsPhysical = 0x000000000000F3FF,
    AnalogsPhysical = 0x00000000003F0000,
    AllPhysical = 0x00000000003FF3FF,
    DPadUpVirtual = 0x0000000100000000,
    DPadDownVirtual = 0x0000000200000000,
    DPadLeftVirtual = 0x0000000400000000,
    DPadRightVirtual = 0x0000000800000000,
    MenuVirtual = 0x0000001000000000,
    ViewVirtual = 0x0000002000000000,
    LeftThumbstickVirtual = 0x0000004000000000,
    RightThumbstickVirtual = 0x0000008000000000,
    LeftShoulderVirtual = 0x0000010000000000,
    RightShoulderVirtual = 0x0000020000000000,
    AVirtual = 0x0000100000000000,
    BVirtual = 0x0000200000000000,
    XVirtual = 0x0000400000000000,
    YVirtual = 0x0000800000000000,
    LeftTriggerVirtual = 0x0001000000000000,
    RightTriggerVirtual = 0x0002000000000000,

    LeftThumbstickXVirtual = 0x0004000000000000,
    LeftThumbstickYVirtual = 0x0008000000000000,
    RightThumbstickXVirtual = 0x0010000000000000,
    RightThumbstickYVirtual = 0x0020000000000000,
    ButtonsVirtual = 0x0000F3FF00000000,
    AnalogsVirtual = 0x003F000000000000,
    AllVirtual = 0x003FF3FF00000000,
}
#[repr(u32)]
pub enum XGameStreamingTouchControlsStateOperationKind {
    Replace = 0,
}
#[repr(u32)]
pub enum XGameStreamingTouchControlsStateValueKind {
    Boolean = 0,
    Integer = 1,
    Double = 2,
    String = 3,
}
#[repr(u32)]
pub enum XGameStreamingVideoFlags {
    None = 0x0,
    SupportsCustomAspectRatio = 0x1,
    SupportsPresentScaling = 0x2,
}

#[repr(C)]
pub struct XGameStreamingDisplayDetails {
    pub preferred_width: u32,
    pub preferred_height: u32,
    pub safe_area: RECT,
    pub max_pixels: u32,
    pub max_width: u32,
    pub max_height: u32,
    pub flags: XGameStreamingVideoFlags,
}
#[repr(C)]
pub struct XGameStreamingTouchControlsStateOperation {
    pub operation_kind: XGameStreamingTouchControlsStateOperationKind,
    pub path: *const c_char,
    pub value: XGameStreamingTouchControlsStateValue,
}

#[repr(C)]
pub struct XGameStreamingTouchControlsStateValue {
    value_kind: XGameStreamingTouchControlsStateValueKind,
    value: XGameStreamingTouchControlsStateValueUnion,
}

#[repr(C)]
pub union XGameStreamingTouchControlsStateValueUnion {
    boolean_value: bool,
    integer_value: i64,
    double_value: f64,
    string_value: *const c_char,
}

// Class _GUID_0a2192aa_b2d5_4d58_83be_383b6d80799e
// IID _GUID_0a2192aa_b2d5_4d58_83be_383b6d80799e
#[interface("0a2192aa-b2d5-4d58-83be-383b6d80799e")]
pub unsafe trait IXGameStreaming: IUnknown {
    // XGameStreamingInitialize
    pub unsafe fn x_game_streaming_initialize(self: &Self) -> HRESULT;
    // XGameStreamingUninitialize
    pub unsafe fn x_game_streaming_uninitialize(self: &Self) -> ();
    // XGameStreamingIsStreaming
    pub unsafe fn x_game_streaming_is_streaming(self: &Self) -> BOOL;
    // XGameStreamingRegisterClientPropertiesChanged
    pub unsafe fn x_game_streaming_register_client_properties_changed(
        self: &Self,
        _client: XGameStreamingClientId,
        _queue: XTaskQueueHandle,
        _context: *mut c_void,
        _callback: Option<XGameStreamingClientPropertiesChangedCallback>,
        _token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT;
    // XGameStreamingUnregisterClientPropertiesChanged
    pub unsafe fn x_game_streaming_unregister_client_properties_changed(
        self: &Self,
        _client: XGameStreamingClientId,
        _token: XTaskQueueRegistrationToken,
        _wait: BOOL,
    ) -> BOOL;
    // XGameStreamingGetStreamPhysicalDimensions
    pub unsafe fn x_game_streaming_get_stream_physical_dimensions(
        self: &Self,
        _client: XGameStreamingClientId,
        _horizontal_mm: *mut u32,
        _vertical_mm: *mut u32,
    ) -> HRESULT;
    // XGameStreamingGetClientCount
    pub unsafe fn x_game_streaming_get_client_count(self: &Self) -> u32;
    // XGameStreamingGetClients
    pub unsafe fn x_game_streaming_get_clients(
        self: &Self,
        _client_count: u32,
        _clients: *mut XGameStreamingClientId,
        _clients_used: *mut u32,
    ) -> HRESULT;
    // XGameStreamingGetConnectionState
    pub unsafe fn x_game_streaming_get_connection_state(
        self: &Self,
        _client: XGameStreamingClientId,
    ) -> XGameStreamingConnectionState;
    // XGameStreamingRegisterConnectionStateChanged
    pub unsafe fn x_game_streaming_register_connection_state_changed(
        self: &Self,
        _queue: XTaskQueueHandle,
        _context: *mut c_void,
        _callback: Option<XGameStreamingConnectionStateChangedCallback>,
        _token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT;
    // XGameStreamingUnregisterConnectionStateChanged
    pub unsafe fn x_game_streaming_unregister_connection_state_changed(
        self: &Self,
        _token: XTaskQueueRegistrationToken,
        _wait: BOOL,
    ) -> BOOL;
    // XGameStreamingGetStreamAddedLatency
    pub unsafe fn x_game_streaming_get_stream_added_latency(
        self: &Self,
        _client: XGameStreamingClientId,
        _average_input_latency_us: *mut u32,
        _average_output_latency_us: *mut u32,
        _standard_deviation_us: *mut u32,
    ) -> HRESULT;
    // XGameStreamingGetServerLocationNameSize
    pub unsafe fn x_game_streaming_get_server_location_name_size(self: &Self) -> usize;
    // XGameStreamingGetServerLocationName
    pub unsafe fn x_game_streaming_get_server_location_name(
        self: &Self,
        _server_location_name_size: usize,
        _server_location_name: *mut c_char,
    ) -> HRESULT;
    // XGameStreamingHideTouchControls
    pub unsafe fn x_game_streaming_hide_touch_controls(self: &Self) -> ();
    // XGameStreamingShowTouchControlLayout
    pub unsafe fn x_game_streaming_show_touch_control_layout(
        self: &Self,
        _layout: *const c_char,
    ) -> ();
    // XGameStreamingHideTouchControlsOnClient
    pub unsafe fn x_game_streaming_hide_touch_controls_on_client(
        self: &Self,
        _client: XGameStreamingClientId,
    ) -> ();
    // XGameStreamingShowTouchControlLayoutOnClient
    pub unsafe fn x_game_streaming_show_touch_control_layout_on_client(
        self: &Self,
        _client: XGameStreamingClientId,
        _layout: *const c_char,
    ) -> ();
    // XGameStreamingIsTouchInputEnabled
    pub unsafe fn x_game_streaming_is_touch_input_enabled(
        self: &Self,
        _client: XGameStreamingClientId,
        _touch_input_enabled: *mut BOOL,
    ) -> HRESULT;
    // XGameStreamingGetLastFrameDisplayed
    pub unsafe fn x_game_streaming_get_last_frame_displayed(
        self: &Self,
        _client: XGameStreamingClientId,
        _frame_pipeline_token: *mut D3d12xboxFramePipelineToken,
    ) -> HRESULT;
    // XGameStreamingGetAssociatedFrame
    pub unsafe fn x_game_streaming_get_associated_frame(
        self: &Self,
        _gamepad_reading: *mut IGameInputReading,
        _frame_pipeline_token: *mut D3d12xboxFramePipelineToken,
    ) -> HRESULT;
    // XGameStreamingGetGamepadPhysicality
    pub unsafe fn x_game_streaming_get_gamepad_physicality(
        self: &Self,
        _gamepad_reading: *mut IGameInputReading,
        _gamepad_physicality: *mut XGameStreamingGamepadPhysicality,
    ) -> HRESULT;
    // XGameStreamingUpdateTouchControlsState
    pub unsafe fn x_game_streaming_update_touch_controls_state(
        self: &Self,
        _operation_count: usize,
        _operations: *const XGameStreamingTouchControlsStateOperation,
    ) -> HRESULT;
    // XGameStreamingUpdateTouchControlsStateOnClient
    pub unsafe fn x_game_streaming_update_touch_controls_state_on_client(
        self: &Self,
        _client: XGameStreamingClientId,
        _operation_count: usize,
        _operations: *const XGameStreamingTouchControlsStateOperation,
    ) -> HRESULT;
    // XGameStreamingShowTouchControlsWithStateUpdate
    pub unsafe fn x_game_streaming_show_touch_controls_with_state_update(
        self: &Self,
        _layout: *const c_char,
        _operation_count: usize,
        _operations: *const XGameStreamingTouchControlsStateOperation,
    ) -> HRESULT;
    // XGameStreamingShowTouchControlsWithStateUpdateOnClient
    pub unsafe fn x_game_streaming_show_touch_controls_with_state_update_on_client(
        self: &Self,
        _client: XGameStreamingClientId,
        _layout: *const c_char,
        _operation_count: usize,
        _operations: *const XGameStreamingTouchControlsStateOperation,
    ) -> HRESULT;
    // XGameStreamingGetTouchBundleVersionNameSize
    pub unsafe fn x_game_streaming_get_touch_bundle_version_name_size(
        self: &Self,
        _client: XGameStreamingClientId,
    ) -> usize;
    // XGameStreamingGetTouchBundleVersion
    pub unsafe fn x_game_streaming_get_touch_bundle_version(
        self: &Self,
        _client: XGameStreamingClientId,
        _version: *mut XVersion,
        _version_name_size: usize,
        _version_name: *mut c_char,
    ) -> HRESULT;
    // XGameStreamingGetClientIPAddress
    pub unsafe fn x_game_streaming_get_client_i_p_address(
        self: &Self,
        _client: XGameStreamingClientId,
        _ip_address_size: usize,
        _ip_address: *mut c_char,
    ) -> HRESULT;
    // XGameStreamingGetSessionId
    pub unsafe fn x_game_streaming_get_session_id(
        self: &Self,
        _client: XGameStreamingClientId,
        _session_id_size: usize,
        _session_id: *mut c_char,
        _session_id_used: *mut usize,
    ) -> HRESULT;
    // XGameStreamingGetDisplayDetails
    pub unsafe fn x_game_streaming_get_display_details(
        self: &Self,
        _client: XGameStreamingClientId,
        _max_supported_pixels: u32,
        _widest_supported_aspect_ratio: f32,
        _tallest_supported_aspect_ratio: f32,
        _display_details: *mut XGameStreamingDisplayDetails,
    ) -> HRESULT;
    // XGameStreamingSetResolution
    pub unsafe fn x_game_streaming_set_resolution(
        self: &Self,
        _width: u32,
        _height: u32,
    ) -> HRESULT;
}
