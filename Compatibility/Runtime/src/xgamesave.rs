use std::ffi::{c_char, c_void};

use windows_core::{BOOL, HRESULT, IUnknown, interface};

use crate::{user::XUserHandle, xasync::XAsyncBlock};

pub type XGameSaveProviderHandle = u64;
pub type XGameSaveContainerHandle = u64;
pub type XGameSaveUpdateHandle = u64;
// XGameSaveBlobInfoCallback
pub type XGameSaveBlobInfoCallback =
    unsafe extern "system" fn(_info: *const XGameSaveBlobInfo, _context: *mut c_void) -> BOOL;
// XGameSaveContainerInfoCallback
pub type XGameSaveContainerInfoCallback =
    unsafe extern "system" fn(_info: *const XGameSaveContainerInfo, _context: *mut c_void) -> BOOL;

#[repr(C)]
pub struct XGameSaveBlob {
    pub info: XGameSaveBlobInfo,
    pub data: *mut u8,
}
#[repr(C)]
pub struct XGameSaveBlobInfo {
    pub name: *const c_char,
    pub size: u32,
}
#[repr(C)]
pub struct XGameSaveContainerInfo {
    pub name: *const c_char,
    pub display_name: *const c_char,
    pub blob_count: u32,
    pub total_size: u64,
    pub last_modified_time: libc::time_t,
    pub needs_sync: bool,
}

// Class _GUID_704c3f58_e629_4cc2_b197_30511b996fe2

#[interface("1bfff3af-f14a-40a3-8e35-9ada906593f9")]
pub unsafe trait IXGameSave: IUnknown {
    // XGameSaveInitializeProvider
    pub unsafe fn x_game_save_initialize_provider(
        self: &Self,
        _requesting_user: XUserHandle,
        _configuration_id: *const c_char,
        _sync_on_demand: BOOL,
        _provider: *mut XGameSaveProviderHandle,
    ) -> HRESULT;
    // XGameSaveInitializeProviderAsync
    pub unsafe fn x_game_save_initialize_provider_async(
        self: &Self,
        _requesting_user: XUserHandle,
        _configuration_id: *const c_char,
        _sync_on_demand: BOOL,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XGameSaveInitializeProviderResult
    pub unsafe fn x_game_save_initialize_provider_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _provider: *mut XGameSaveProviderHandle,
    ) -> HRESULT;
    // XGameSaveCloseProvider
    pub unsafe fn x_game_save_close_provider(self: &Self, _provider: XGameSaveProviderHandle)
    -> ();
    // XGameSaveGetRemainingQuota
    pub unsafe fn x_game_save_get_remaining_quota(
        self: &Self,
        _provider: XGameSaveProviderHandle,
        _remaining_quota: *mut i64,
    ) -> HRESULT;
    // XGameSaveGetRemainingQuotaAsync
    pub unsafe fn x_game_save_get_remaining_quota_async(
        self: &Self,
        _provider: XGameSaveProviderHandle,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XGameSaveGetRemainingQuotaResult
    pub unsafe fn x_game_save_get_remaining_quota_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _remaining_quota: *mut i64,
    ) -> HRESULT;
    // XGameSaveDeleteContainer
    pub unsafe fn x_game_save_delete_container(
        self: &Self,
        _provider: XGameSaveProviderHandle,
        _container_name: *const c_char,
    ) -> HRESULT;
    // XGameSaveDeleteContainerAsync
    pub unsafe fn x_game_save_delete_container_async(
        self: &Self,
        _provider: XGameSaveProviderHandle,
        _container_name: *const c_char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XGameSaveDeleteContainerResult
    pub unsafe fn x_game_save_delete_container_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XGameSaveGetContainerInfo
    pub unsafe fn x_game_save_get_container_info(
        self: &Self,
        _provider: XGameSaveProviderHandle,
        _container_name: *const c_char,
        _context: *mut c_void,
        _callback: Option<XGameSaveContainerInfoCallback>,
    ) -> HRESULT;
    // XGameSaveEnumerateContainerInfo
    pub unsafe fn x_game_save_enumerate_container_info(
        self: &Self,
        _provider: XGameSaveProviderHandle,
        _context: *mut c_void,
        _callback: Option<XGameSaveContainerInfoCallback>,
    ) -> HRESULT;
    // XGameSaveEnumerateContainerInfoByName
    pub unsafe fn x_game_save_enumerate_container_info_by_name(
        self: &Self,
        _provider: XGameSaveProviderHandle,
        _container_name_prefix: *const c_char,
        _context: *mut c_void,
        _callback: Option<XGameSaveContainerInfoCallback>,
    ) -> HRESULT;
    // XGameSaveCreateContainer
    pub unsafe fn x_game_save_create_container(
        self: &Self,
        _provider: XGameSaveProviderHandle,
        _container_name: *const c_char,
        _container_context: *mut XGameSaveContainerHandle,
    ) -> HRESULT;
    // XGameSaveCloseContainer
    pub unsafe fn x_game_save_close_container(
        self: &Self,
        _context: XGameSaveContainerHandle,
    ) -> ();
    // XGameSaveEnumerateBlobInfo
    pub unsafe fn x_game_save_enumerate_blob_info(
        self: &Self,
        _container: XGameSaveContainerHandle,
        _context: *mut c_void,
        _callback: Option<XGameSaveBlobInfoCallback>,
    ) -> HRESULT;
    // XGameSaveEnumerateBlobInfoByName
    pub unsafe fn x_game_save_enumerate_blob_info_by_name(
        self: &Self,
        _container: XGameSaveContainerHandle,
        _blob_name_prefix: *const c_char,
        _context: *mut c_void,
        _callback: Option<XGameSaveBlobInfoCallback>,
    ) -> HRESULT;
    // XGameSaveReadBlobData
    pub unsafe fn x_game_save_read_blob_data(
        self: &Self,
        _container: XGameSaveContainerHandle,
        _blob_names: *const *mut c_char,
        _count_of_blobs: *mut u32,
        _blobs_size: usize,
        _blob_data: *mut XGameSaveBlob,
    ) -> HRESULT;
    // XGameSaveReadBlobDataAsync
    pub unsafe fn x_game_save_read_blob_data_async(
        self: &Self,
        _container: XGameSaveContainerHandle,
        _blob_names: *const *mut c_char,
        _count_of_blobs: u32,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XGameSaveReadBlobDataResult
    pub unsafe fn x_game_save_read_blob_data_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _blobs_size: usize,
        _blob_data: *mut XGameSaveBlob,
        _count_of_blobs: *mut u32,
    ) -> HRESULT;
    // XGameSaveCreateUpdate
    pub unsafe fn x_game_save_create_update(
        self: &Self,
        _container: XGameSaveContainerHandle,
        _container_display_name: *const c_char,
        _update_context: *mut XGameSaveUpdateHandle,
    ) -> HRESULT;
    // XGameSaveCloseUpdate
    pub unsafe fn x_game_save_close_update(self: &Self, _context: XGameSaveUpdateHandle) -> ();
    // XGameSaveSubmitBlobWrite
    pub unsafe fn x_game_save_submit_blob_write(
        self: &Self,
        _update_context: XGameSaveUpdateHandle,
        _blob_name: *const c_char,
        _data: *const u8,
        _byte_count: usize,
    ) -> HRESULT;
    // XGameSaveSubmitBlobDelete
    pub unsafe fn x_game_save_submit_blob_delete(
        self: &Self,
        _update_context: XGameSaveUpdateHandle,
        _blob_name: *const c_char,
    ) -> HRESULT;
    // XGameSaveSubmitUpdate
    pub unsafe fn x_game_save_submit_update(
        self: &Self,
        _update_context: XGameSaveUpdateHandle,
    ) -> HRESULT;
    // XGameSaveSubmitUpdateAsync
    pub unsafe fn x_game_save_submit_update_async(
        self: &Self,
        _update_context: XGameSaveUpdateHandle,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XGameSaveSubmitUpdateResult
    pub unsafe fn x_game_save_submit_update_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XGameSaveFilesGetFolderWithUiAsync
    pub unsafe fn x_game_save_files_get_folder_with_ui_async(
        self: &Self,
        _requesting_user: XUserHandle,
        _configuration_id: *const char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XGameSaveFilesGetFolderWithUiResult
    pub unsafe fn x_game_save_files_get_folder_with_ui_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _folder_size: usize,
        _folder_result: *mut char,
    ) -> HRESULT;
    // XGameSaveFilesGetRemainingQuota
    pub unsafe fn x_game_save_files_get_remaining_quota(
        self: &Self,
        _user_context: XUserHandle,
        _configuration_id: *const char,
        _remaining_quota: *mut i64,
    ) -> HRESULT;
    // PFXGameSaveInitializeConfig
    pub unsafe fn __reserved_slot_33(&self);
    pub unsafe fn __reserved_slot_34(&self);
    // PFXGameSaveFilesGetFolderWithUiAsync
    pub unsafe fn __reserved_slot_35(&self);
    // PFXGameSaveFilesGetFolderWithUiResult
    pub unsafe fn __reserved_slot_36(&self);
    // PFXGameSaveFilesGetRemainingQuota
    pub unsafe fn __reserved_slot_37(&self);
    // PFXGameSaveSetUiCallbacks
    pub unsafe fn __reserved_slot_38(&self);
    // PFXGameSaveProgressUiGetProgress
    pub unsafe fn __reserved_slot_39(&self);
    // PFXGameSaveSetProgressUiResponse
    pub unsafe fn __reserved_slot_40(&self);
    // PFXGameSaveSetSyncFailedUiResponse
    pub unsafe fn __reserved_slot_41(&self);
    // PFXGameSaveSetActiveDeviceContentionUiResponse
    pub unsafe fn __reserved_slot_42(&self);
    // PFXGameSaveSetConflictUiResponse
    pub unsafe fn __reserved_slot_43(&self);
    // PFXGameSaveSetOutOfStorageUiResponse
    pub unsafe fn __reserved_slot_44(&self);
    // PFXGameSaveFilesUploadWithUiAsync
    pub unsafe fn __reserved_slot_45(&self);
    // PFXGameSaveFilesUploadWithUiResult
    pub unsafe fn __reserved_slot_46(&self);
    // PFXGameSaveFilesSetSaveDescriptionAsync
    pub unsafe fn __reserved_slot_47(&self);
    // PFXGameSaveFilesSetSaveDescriptionResult
    pub unsafe fn __reserved_slot_48(&self);
    // PFXGameSaveSetActiveDeviceChangedCallback
    pub unsafe fn __reserved_slot_49(&self);
}

#[interface("ab4ae4fb-6508-4950-a032-45fd4bf8c43b")]
pub unsafe trait IXGameSave2: IXGameSave {}

#[interface("704C3F58-E629-4CC2-B197-30511B996EE2")]
pub unsafe trait IXGameSave3: IXGameSave {}
