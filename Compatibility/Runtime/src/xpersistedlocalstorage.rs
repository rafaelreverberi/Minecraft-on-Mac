use std::ffi::c_char;

use windows_core::{HRESULT, IUnknown, interface};

use crate::{xasync::XAsyncBlock, xpackage::XPackageMountHandle};

#[repr(C)]
pub struct XPersistentLocalStorageSpaceInfo {
    pub available_free_bytes: u64,
    pub total_free_bytes: u64,
    pub used_bytes: u64,
    pub total_bytes: u64,
}

#[interface("41a4e10c-5a7e-41d9-8c37-37bde62a07d6")]
pub unsafe trait IXPersistentLocalStorage: IUnknown {
    // XPersistentLocalStorageGetPathSize
    pub unsafe fn x_persistent_local_storage_get_path_size(
        self: &Self,
        _path_size: *mut usize,
    ) -> HRESULT;
    // XPersistentLocalStorageGetPath
    pub unsafe fn x_persistent_local_storage_get_path(
        self: &Self,
        _path_size: usize,
        _path: *mut c_char,
        _path_used: *mut usize,
    ) -> HRESULT;
    // XPersistentLocalStorageGetSpaceInfo
    pub unsafe fn x_persistent_local_storage_get_space_info(
        self: &Self,
        _info: *mut XPersistentLocalStorageSpaceInfo,
    ) -> HRESULT;
    // XPersistentLocalStoragePromptUserForSpaceAsync
    pub unsafe fn x_persistent_local_storage_prompt_user_for_space_async(
        self: &Self,
        _requested_bytes: u64,
        _async_block: *mut XAsyncBlock,
    ) -> HRESULT;
    // XPersistentLocalStoragePromptUserForSpaceResult
    pub unsafe fn x_persistent_local_storage_prompt_user_for_space_result(
        self: &Self,
        _async_block: *mut XAsyncBlock,
    ) -> HRESULT;
    // XPersistentLocalStorageMountForPackage
    pub unsafe fn x_persistent_local_storage_mount_for_package(
        self: &Self,
        _package_identifier: *const c_char,
        _mount_handle: *mut XPackageMountHandle,
    ) -> HRESULT;
}

#[interface("D29411DF-0794-4553-8B27-95FC02D0F75D")]
pub unsafe trait IXPersistentLocalStorage2: IXPersistentLocalStorage {}
