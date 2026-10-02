use std::ffi::{c_char, c_void};

use windows_core::{BOOL, HRESULT, IUnknown, interface};

use crate::{
    threading::{XTaskQueueHandle, XTaskQueueRegistrationToken},
    xasync::XAsyncBlock,
};

#[repr(u32)]
pub enum XPackageChunkAvailability {
    Ready,
    Pending,
    Installable,
    Unavailable,
}
#[repr(u32)]
#[derive(PartialEq)]
pub enum XPackageChunkSelectorType {
    Language,
    Tag,
    Chunk,
    Feature,
}
#[repr(u32)]
pub enum XPackageEnumerationScope {
    ThisOnly,
    ThisAndRelated,
    ThisPublisher,
}
#[repr(u32)]
pub enum XPackageKind {
    Game,
    Content,
}

pub type XVersion = u64;

pub type XPackageInstallationMonitorHandle = usize;
pub type XPackageMountHandle = usize;

#[repr(C)]
pub struct XPackageDetails {
    pub package_identifier: *const c_char,
    pub version: XVersion,
    pub kind: XPackageKind,
    pub display_name: *const c_char,
    pub description: *const c_char,
    pub publisher: *const c_char,
    pub store_id: *const c_char,
    pub installing: bool,
    pub index: u32,
    pub count: u32,
    pub age_restricted: bool,
    pub title_i_d: *const c_char,
}
#[repr(C)]
pub struct XPackageFeature {
    pub id: *const c_char,
    pub display_name: *const c_char,
    pub tags: *const c_char,
    pub hidden: bool,
    pub store_id_count: u32,
    pub store_ids: *const *mut c_char,
}
#[repr(C)]
pub struct XPackageInstallationProgress {
    pub total_bytes: u64,
    pub installed_bytes: u64,
    pub launch_bytes: u64,
    pub launchable: bool,
    pub completed: bool,
}
#[repr(C)]
pub struct XPackageWriteStats {
    pub interval: u64,
    pub budget: u64,
    pub elapsed: u64,
    pub bytes_written: u64,
}

// XPackageInstallationProgressCallback
pub type XPackageInstallationProgressCallback = unsafe extern "system" fn(
    _context: *mut c_void,
    _monitor: XPackageInstallationMonitorHandle,
) -> ();

// XPackageEnumerationCallback
pub type XPackageEnumerationCallback =
    unsafe extern "system" fn(_context: *mut c_void, _details: *const XPackageDetails) -> BOOL;

// XPackageFeatureEnumerationCallback
pub type XPackageFeatureEnumerationCallback =
    unsafe extern "system" fn(_context: *mut c_void, _feature: *const XPackageFeature) -> BOOL;

// XPackageInstalledCallback
pub type XPackageInstalledCallback =
    unsafe extern "system" fn(_context: *mut c_void, _details: *const XPackageDetails) -> ();

// XPackageChunkAvailabilityCallback
pub type XPackageChunkAvailabilityCallback = unsafe extern "system" fn(
    _context: *mut c_void,
    _selector: *const XPackageChunkSelector,
    _availability: XPackageChunkAvailability,
) -> BOOL;

#[repr(C)]
pub union XPackageChunkSelectorData {
    pub language: *const c_char,
    pub tag: *const c_char,
    pub feature: *const c_char,
    pub chunk_id: u32,
}

#[repr(C)]
pub struct XPackageChunkSelector {
    pub type_: XPackageChunkSelectorType,
    pub data: XPackageChunkSelectorData,
}

// Class _GUID_af406016_e850_4aa8_a88d_2f3dcb9dac7e
// IID _GUID_af406016_e850_4aa8_a88d_2f3dcb9dac7e
#[interface("e2a4734b-2f4a-456d-aa8f-d065e04fb209")]
pub unsafe trait IXPackage: IUnknown {
    // XPackageGetCurrentProcessPackageIdentifier
    pub unsafe fn x_package_get_current_process_package_identifier(
        self: &Self,
        _buffer_size: usize,
        _buffer: *mut c_char,
    ) -> HRESULT;
    // XPackageIsPackagedProcess
    pub unsafe fn x_package_is_packaged_process(self: &Self) -> BOOL;
    // XPackageCreateInstallationMonitor
    pub unsafe fn x_package_create_installation_monitor(
        self: &Self,
        _package_identifier: *const c_char,
        _selector_count: u32,
        _selectors: *mut XPackageChunkSelector,
        _minimum_update_interval_ms: u32,
        _queue: XTaskQueueHandle,
        _installation_monitor: *mut XPackageInstallationMonitorHandle,
    ) -> HRESULT;
    // XPackageCloseInstallationMonitorHandle
    pub unsafe fn x_package_close_installation_monitor_handle(
        self: &Self,
        _installation_monitor: XPackageInstallationMonitorHandle,
    ) -> ();
    // XPackageGetInstallationProgress
    pub unsafe fn x_package_get_installation_progress(
        self: &Self,
        _installation_monitor: XPackageInstallationMonitorHandle,
        _progress: *mut XPackageInstallationProgress,
    ) -> ();
    // XPackageUpdateInstallationMonitor
    pub unsafe fn x_package_update_installation_monitor(
        self: &Self,
        _installation_monitor: XPackageInstallationMonitorHandle,
    ) -> BOOL;
    // XPackageRegisterInstallationProgressChanged
    pub unsafe fn x_package_register_installation_progress_changed(
        self: &Self,
        _installation_monitor: XPackageInstallationMonitorHandle,
        _context: *mut c_void,
        _callback: Option<XPackageInstallationProgressCallback>,
        _token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT;
    // XPackageUnregisterInstallationProgressChanged
    pub unsafe fn x_package_unregister_installation_progress_changed(
        self: &Self,
        _installation_monitor: XPackageInstallationMonitorHandle,
        _token: XTaskQueueRegistrationToken,
        _wait: BOOL,
    ) -> BOOL;
    // XPackageGetUserLocale
    pub unsafe fn x_package_get_user_locale(
        self: &Self,
        _locale_size: usize,
        _locale: *mut c_char,
    ) -> HRESULT;
    // XPackageFindChunkAvailability
    pub unsafe fn x_package_find_chunk_availability(
        self: &Self,
        _package_identifier: *const c_char,
        _selector_count: u32,
        _selectors: *mut XPackageChunkSelector,
        _availability: *mut XPackageChunkAvailability,
    ) -> HRESULT;
    // XPackageEnumerateChunkAvailability
    pub unsafe fn x_package_enumerate_chunk_availability(
        self: &Self,
        _package_identifier: *const c_char,
        _type_: XPackageChunkSelectorType,
        _context: *mut c_void,
        _callback: Option<XPackageChunkAvailabilityCallback>,
    ) -> HRESULT;
    // XPackageChangeChunkInstallOrder
    pub unsafe fn x_package_change_chunk_install_order(
        self: &Self,
        _package_identifier: *const c_char,
        _selector_count: u32,
        _selectors: *mut XPackageChunkSelector,
    ) -> HRESULT;
    // XPackageInstallChunks
    pub unsafe fn x_package_install_chunks(
        self: &Self,
        _package_identifier: *const c_char,
        _selector_count: u32,
        _selectors: *mut XPackageChunkSelector,
        _minimum_update_interval_ms: u32,
        _suppress_user_confirmation: BOOL,
        _queue: XTaskQueueHandle,
        _installation_monitor: *mut XPackageInstallationMonitorHandle,
    ) -> HRESULT;
    // XPackageInstallChunksAsync
    pub unsafe fn x_package_install_chunks_async(
        self: &Self,
        _package_identifier: *const c_char,
        _selector_count: u32,
        _selectors: *mut XPackageChunkSelector,
        _minimum_update_interval_ms: u32,
        _suppress_user_confirmation: BOOL,
        _async_block: *mut XAsyncBlock,
    ) -> HRESULT;
    // XPackageInstallChunksResult
    pub unsafe fn x_package_install_chunks_result(
        self: &Self,
        _async_block: *mut XAsyncBlock,
        _installation_monitor: *mut XPackageInstallationMonitorHandle,
    ) -> HRESULT;
    // XPackageEstimateDownloadSize
    pub unsafe fn x_package_estimate_download_size(
        self: &Self,
        _package_identifier: *const c_char,
        _selector_count: u32,
        _selectors: *mut XPackageChunkSelector,
        _download_size: *mut u64,
        _should_present_user_confirmation: *mut BOOL,
    ) -> HRESULT;
    // XPackageUninstallChunks
    pub unsafe fn x_package_uninstall_chunks(
        self: &Self,
        _package_identifier: *const c_char,
        _selector_count: u32,
        _selectors: *mut XPackageChunkSelector,
    ) -> HRESULT;
    pub unsafe fn __reserved_slot_20(&self);
    pub unsafe fn __reserved_slot_21(&self);
    // XPackageUnregisterPackageInstalled
    pub unsafe fn x_package_unregister_package_installed(
        self: &Self,
        _token: XTaskQueueRegistrationToken,
        _wait: BOOL,
    ) -> BOOL;
    pub unsafe fn __reserved_slot_23(&self);
    // XPackageGetMountPathSize
    pub unsafe fn x_package_get_mount_path_size(
        self: &Self,
        _mount: XPackageMountHandle,
        _path_size: *mut usize,
    ) -> HRESULT;
    // XPackageGetMountPath
    pub unsafe fn x_package_get_mount_path(
        self: &Self,
        _mount: XPackageMountHandle,
        _path_size: usize,
        _path: *mut c_char,
    ) -> HRESULT;
    // XPackageCloseMountHandle
    pub unsafe fn x_package_close_mount_handle(self: &Self, _mount: XPackageMountHandle) -> ();
    pub unsafe fn __reserved_slot_27(&self);
    pub unsafe fn x_package_enumerate_packages2(
        self: &Self,
        _kind: XPackageKind,
        _scope: XPackageEnumerationScope,
        _context: *mut c_void,
        _callback: Option<XPackageEnumerationCallback>,
    ) -> HRESULT;
    pub unsafe fn __reserved_slot_29(&self);
    // XPackageGetWriteStats
    pub unsafe fn x_package_get_write_stats(
        self: &Self,
        _write_stats: *mut XPackageWriteStats,
    ) -> HRESULT;
    pub unsafe fn __reserved_slot_31(&self);
    // XPackageUninstallUWPInstance
    pub unsafe fn x_package_uninstall_u_w_p_instance(
        self: &Self,
        _package_name: *const c_char,
    ) -> HRESULT;
    // XPackageEnumerateFeatures
    pub unsafe fn x_package_enumerate_features(
        self: &Self,
        _package_identifier: *const c_char,
        _context: *mut c_void,
        _callback: Option<XPackageFeatureEnumerationCallback>,
    ) -> HRESULT;
    // XPackageUninstallPackage
    pub unsafe fn x_package_uninstall_package(
        self: &Self,
        _package_identifier: *const c_char,
    ) -> BOOL;
    pub unsafe fn __reserved_slot_35(&self);
    pub unsafe fn __reserved_slot_36(&self);
    // XPackageMountWithUiAsync
    pub unsafe fn x_package_mount_with_ui_async(
        self: &Self,
        _package_identifier: *const char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XPackageMountWithUiResult
    pub unsafe fn x_package_mount_with_ui_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _mount: *mut XPackageMountHandle,
    ) -> HRESULT;
    // XPackageEnumeratePackages
    pub unsafe fn x_package_enumerate_packages(
        self: &Self,
        _kind: XPackageKind,
        _scope: XPackageEnumerationScope,
        _context: *mut c_void,
        _callback: Option<XPackageEnumerationCallback>,
    ) -> HRESULT;
    // XPackageRegisterPackageInstalled
    pub unsafe fn x_package_register_package_installed(
        self: &Self,
        _queue: XTaskQueueHandle,
        _context: *mut c_void,
        _callback: Option<XPackageInstalledCallback>,
        _token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT;
}

#[interface("3720DE07-E8E4-44A3-AD32-B359E8ADBE55")]
pub unsafe trait IXPackage2: IXPackage {}
