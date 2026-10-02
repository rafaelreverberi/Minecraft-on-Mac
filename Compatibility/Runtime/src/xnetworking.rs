use std::ffi::{c_char, c_void};

use windows_core::{BOOL, HRESULT, IUnknown, interface};

use crate::{
    threading::{XTaskQueueHandle, XTaskQueueRegistrationToken},
    xasync::XAsyncBlock,
};

#[repr(u32)]
pub enum XNetworkingConfigurationSetting {
    MaxTitleTcpQueuedReceiveBufferSize = 0,
    MaxSystemTcpQueuedReceiveBufferSize = 1,
    MaxToolsTcpQueuedReceiveBufferSize = 2,
}
#[repr(u32)]
pub enum XNetworkingConnectivityCostHint {
    Unknown = 0,
    Unrestricted = 1,
    Fixed = 2,
    Variable = 3,
}
#[repr(u32)]
pub enum XNetworkingConnectivityLevelHint {
    Unknown = 0,
    None = 1,
    LocalAccess = 2,
    InternetAccess = 3,
    ConstrainedInternetAccess = 4,
}
#[repr(u32)]
pub enum XNetworkingStatisticsType {
    TitleTcpQueuedReceivedBufferUsage = 0,
    SystemTcpQueuedReceivedBufferUsage = 1,
    ToolsTcpQueuedReceivedBufferUsage = 2,
}
#[repr(u32)]
pub enum XNetworkingThumbprintType {
    Leaf = 0,
    Issuer = 1,
    Root = 2,
}

#[repr(C)]
pub struct XNetworkingConnectivityHint {
    pub connectivity_level: XNetworkingConnectivityLevelHint,
    pub connectivity_cost: XNetworkingConnectivityCostHint,
    pub iana_interface_type: u32,
    pub network_initialized: bool,
    pub approaching_data_limit: bool,
    pub over_data_limit: bool,
    pub roaming: bool,
}
#[repr(C)]
pub struct XNetworkingSecurityInformation {
    pub enabled_http_security_protocol_flags: u32,
    pub thumbprint_count: usize,
    pub thumbprints: *mut XNetworkingThumbprint,
}
#[repr(C)]
pub struct XNetworkingTcpQueuedReceivedBufferUsageStatistics {
    pub num_bytes_currently_queued: u64,
    pub peak_num_bytes_ever_queued: u64,
    pub total_num_bytes_queued: u64,
    pub num_bytes_dropped_for_exceeding_configured_max: u64,
    pub num_bytes_dropped_due_to_any_failure: u64,
}
#[repr(C)]
pub struct XNetworkingThumbprint {
    pub thumbprint_type: XNetworkingThumbprintType,
    pub thumbprint_buffer_byte_count: usize,
    pub thumbprint_buffer: *mut u8,
}

pub type XNetworkingStatisticsBuffer = XNetworkingTcpQueuedReceivedBufferUsageStatistics;

// XNetworkingPreferredLocalUdpMultiplayerPortChangedCallback
pub type XNetworkingPreferredLocalUdpMultiplayerPortChangedCallback =
    unsafe extern "system" fn(
        _context: *mut c_void,
        _preferred_local_udp_multiplayer_port: u16,
    ) -> ();
// XNetworkingConnectivityHintChangedCallback
pub type XNetworkingConnectivityHintChangedCallback = unsafe extern "system" fn(
    _context: *mut c_void,
    _connectivity_hint: *const XNetworkingConnectivityHint,
) -> ();

// Class _GUID_37e56907_2f10_41e8_b72f_36edb185331a
// IID _GUID_37e56907_2f10_41e8_b72f_36edb185331a
#[interface("37e56907-2f10-41e8-b72f-36edb185331a")]
pub unsafe trait IXNetworking: IUnknown {
    // XNetworkingQueryPreferredLocalUdpMultiplayerPort
    pub unsafe fn x_networking_query_preferred_local_udp_multiplayer_port(
        self: &Self,
        _preferred_local_udp_multiplayer_port: *mut u16,
    ) -> HRESULT;
    // XNetworkingQueryPreferredLocalUdpMultiplayerPortAsync
    pub unsafe fn x_networking_query_preferred_local_udp_multiplayer_port_async(
        self: &Self,
        _async_block: *mut XAsyncBlock,
    ) -> HRESULT;
    // XNetworkingQueryPreferredLocalUdpMultiplayerPortAsyncResult
    pub unsafe fn x_networking_query_preferred_local_udp_multiplayer_port_async_result(
        self: &Self,
        _async_block: *mut XAsyncBlock,
        _preferred_local_udp_multiplayer_port: *mut u16,
    ) -> HRESULT;
    // XNetworkingRegisterPreferredLocalUdpMultiplayerPortChanged
    pub unsafe fn x_networking_register_preferred_local_udp_multiplayer_port_changed(
        self: &Self,
        _queue: XTaskQueueHandle,
        _context: *mut c_void,
        _callback: Option<XNetworkingPreferredLocalUdpMultiplayerPortChangedCallback>,
        _token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT;
    // XNetworkingUnregisterPreferredLocalUdpMultiplayerPortChanged
    pub unsafe fn x_networking_unregister_preferred_local_udp_multiplayer_port_changed(
        self: &Self,
        _token: XTaskQueueRegistrationToken,
        _wait: BOOL,
    ) -> BOOL;
    // XNetworkingQuerySecurityInformationForUrlAsync
    pub unsafe fn x_networking_query_security_information_for_url_async(
        self: &Self,
        _url: *const c_char,
        _async_block: *mut XAsyncBlock,
    ) -> HRESULT;
    // XNetworkingQuerySecurityInformationForUrlAsyncResultSize
    pub unsafe fn x_networking_query_security_information_for_url_async_result_size(
        self: &Self,
        _async_block: *mut XAsyncBlock,
        _security_information_buffer_byte_count: *mut usize,
    ) -> HRESULT;
    // XNetworkingQuerySecurityInformationForUrlAsyncResult
    pub unsafe fn x_networking_query_security_information_for_url_async_result(
        self: &Self,
        _async_block: *mut XAsyncBlock,
        _security_information_buffer_byte_count: usize,
        _security_information_buffer_byte_count_used: *mut usize,
        _security_information_buffer: *mut u8,
        _security_information: *mut *mut XNetworkingSecurityInformation,
    ) -> HRESULT;
    // XNetworkingQuerySecurityInformationForUrlUtf16Async
    pub unsafe fn x_networking_query_security_information_for_url_utf16_async(
        self: &Self,
        _url: *const u16,
        _async_block: *mut XAsyncBlock,
    ) -> HRESULT;
    // XNetworkingQuerySecurityInformationForUrlUtf16AsyncResultSize
    pub unsafe fn x_networking_query_security_information_for_url_utf16_async_result_size(
        self: &Self,
        _async_block: *mut XAsyncBlock,
        _security_information_buffer_byte_count: *mut usize,
    ) -> HRESULT;
    // XNetworkingQuerySecurityInformationForUrlUtf16AsyncResult
    pub unsafe fn x_networking_query_security_information_for_url_utf16_async_result(
        self: &Self,
        _async_block: *mut XAsyncBlock,
        _security_information_buffer_byte_count: usize,
        _security_information_buffer_byte_count_used: *mut usize,
        _security_information_buffer: *mut u8,
        _security_information: *mut *mut XNetworkingSecurityInformation,
    ) -> HRESULT;
    // XNetworkingVerifyServerCertificate
    pub unsafe fn x_networking_verify_server_certificate(
        self: &Self,
        _request_handle: *mut c_void,
        _security_information: *const XNetworkingSecurityInformation,
    ) -> HRESULT;
    // XNetworkingGetConnectivityHint
    pub unsafe fn x_networking_get_connectivity_hint(
        self: &Self,
        _connectivity_hint: *mut XNetworkingConnectivityHint,
    ) -> HRESULT;
    // XNetworkingRegisterConnectivityHintChanged
    pub unsafe fn x_networking_register_connectivity_hint_changed(
        self: &Self,
        _queue: XTaskQueueHandle,
        _context: *mut c_void,
        _callback: Option<XNetworkingConnectivityHintChangedCallback>,
        _token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT;
    // XNetworkingUnregisterConnectivityHintChanged
    pub unsafe fn x_networking_unregister_connectivity_hint_changed(
        self: &Self,
        _token: XTaskQueueRegistrationToken,
        _wait: BOOL,
    ) -> BOOL;
    // XNetworkingQueryConfigurationSetting
    pub unsafe fn x_networking_query_configuration_setting(
        self: &Self,
        _configuration_setting: XNetworkingConfigurationSetting,
        _value: *mut u64,
    ) -> HRESULT;
    // XNetworkingSetConfigurationSetting
    pub unsafe fn x_networking_set_configuration_setting(
        self: &Self,
        _configuration_parameter: XNetworkingConfigurationSetting,
        _value: u64,
    ) -> HRESULT;
    // XNetworkingQueryStatistics
    pub unsafe fn x_networking_query_statistics(
        self: &Self,
        _statistics_type: XNetworkingStatisticsType,
        _statistics_buffer: *mut XNetworkingStatisticsBuffer,
    ) -> HRESULT;
}
