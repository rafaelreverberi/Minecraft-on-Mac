use std::{ffi::{CStr, CString, c_char, c_void}, io::{self, Read, Write}, os::windows::fs::MetadataExt, path::Path, ptr::{null, null_mut}, str::{FromStr}};

use windows::{libloaderapi::GetModuleFileNameW, minwindef::MAX_PATH};
use windows_core::{BOOL, HRESULT, implement};
use crate::{E_FAIL, E_NOTIMPL, S_OK, threading::{XTaskQueueHandle, XTaskQueueRegistrationToken}, user::{XUserHandle, load_game_config}, xasync::{self, XAsyncBlock}, xgamesave::{IXGameSave, IXGameSave_Impl, IXGameSave2, IXGameSave2_Impl, IXGameSave3, IXGameSave3_Impl, XGameSaveBlob, XGameSaveBlobInfo, XGameSaveBlobInfoCallback, XGameSaveContainerHandle, XGameSaveContainerInfo, XGameSaveContainerInfoCallback, XGameSaveProviderHandle, XGameSaveUpdateHandle}, xlaunch::{IXLaunch_Impl, IXLaunch2_Impl, IXLaunch3_Impl}, xpackage::{IXPackage_Impl, XPackageChunkAvailability, XPackageChunkSelector, XPackageInstallationMonitorHandle, XPackageInstallationProgress, XPackageInstallationProgressCallback, *}, xsystem::{IXSystem_Impl, IXSystem2_Impl, IXSystem3_Impl, IXSystem4_Impl, IXSystem5_Impl, XSystemHandleCallback}};
use crate::xsystem::IXSystem;
use crate::xsystem::IXSystem5;
use crate::xlaunch::IXLaunch;
use crate::xlaunch::IXLaunch2;
use crate::xlaunch::IXLaunch3;

#[implement(IXGameSave, IXGameSave2, IXGameSave3, IXSystem, IXSystem5, IXPackage, IXPackage2, IXLaunch, IXLaunch2,IXLaunch3)]
pub struct XStub;

impl IXGameSave3_Impl for XStub_Impl {}

impl IXLaunch_Impl for XStub_Impl {
    unsafe fn x_game_get_xbox_title_id(&self, title_id: *mut u32) -> HRESULT {
        println!("x_game_get_xbox_title_id");
        load_game_config().map_or_else(|| E_FAIL, |cfg| {
            unsafe { *title_id = cfg.get_title_id() as u32 };
            println!("x_game_get_xbox_title_id {}", cfg.get_title_id());
            S_OK
        })
    }

    unsafe fn x_launch_new_game(&self,_exe_path: *const c_char,_args: *const c_char,_default_user: XUserHandle) -> () {
        todo!()
    }

    unsafe fn x_launch_restart_on_crash(&self,_args: *const c_char,_reserved: u32) -> HRESULT {
        todo!()
    }
}
impl IXLaunch2_Impl for XStub_Impl {}
impl IXLaunch3_Impl for XStub_Impl {}


impl IXSystem_Impl for XStub_Impl {
    unsafe fn x_system_get_console_id(&self,_console_id_size: usize,_console_id: *mut c_char,_console_id_used: *mut usize) -> HRESULT {
        todo!()
    }

    unsafe fn x_system_get_xbox_live_sandbox_id(&self, sandbox_id_size: usize, sandbox_id: *mut c_char, sandbox_id_used: *mut usize) -> HRESULT {
        println!("x_system_get_xbox_live_sandbox_id");
        let out = unsafe { &mut *std::slice::from_raw_parts_mut(sandbox_id, sandbox_id_size) };
        out.iter_mut().zip(c"RETAIL".to_bytes_with_nul().iter()).for_each(|(d,i)| *d = *i as i8);
        S_OK
    }

    unsafe fn x_system_get_app_specific_device_id(&self,_app_specific_device_id_size: usize,_app_specific_device_id: *mut c_char,_app_specific_device_id_used: *mut usize) -> HRESULT {
        S_OK
    }

    unsafe fn x_system_handle_track(&self,_callback: XSystemHandleCallback,_context: *mut c_void) -> HRESULT {
        todo!()
    }

    unsafe fn x_system_is_handle_valid(&self,_handle: i64) -> BOOL {
        todo!()
    }

    unsafe fn x_system_allow_full_download_bandwidth(&self,_enable: BOOL) -> () {
        todo!()
    }
}

impl IXSystem2_Impl for XStub_Impl {}
impl IXSystem3_Impl for XStub_Impl {}
impl IXSystem4_Impl for XStub_Impl {}
impl IXSystem5_Impl for XStub_Impl {}

struct XGameSaveUpdate {
    container: *mut XGameSaveContainer,
    container_display_name: String,
}

struct XGameSaveContainer {
    container_name: String,
    provider: *mut XGameSaveProvider,
}

struct XGameSaveProvider {
    root: String
}

impl XGameSaveProvider {
    fn new(user: XUserHandle) -> Result<Self, HRESULT> {
        let xuid = unsafe { crate::user::authenticated_xuid(user) }.ok_or(E_FAIL)?;
        let local = std::env::var_os("LOCALAPPDATA").map(std::path::PathBuf::from)
            .or_else(|| std::env::var_os("USERPROFILE").map(|p|std::path::PathBuf::from(p).join("AppData").join("Local")))
            .ok_or(E_FAIL)?;
        // Stable per-title, per-real-account storage in the owned bottle, outside version folders.
        Ok(Self { root: local.join("Minecraft on Mac").join("Dungeons2").join(xuid.to_string()).join("savedata").to_string_lossy().into_owned() })
    }
}

fn visit_dirs(dir: &Path, cb: &dyn Fn(&Path, &XGameSaveContainerInfo)) -> io::Result<()> {
    if dir.is_dir() {
        let mut container_info = XGameSaveContainerInfo {
            blob_count: 0,
            display_name: null(),
            last_modified_time: 0,
            name: null(),
            needs_sync: false,
            total_size: 0,
        };

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, cb)?;
            } else {
                let md = entry.metadata().unwrap();
                container_info.blob_count += 1;
                container_info.total_size += md.len();
                // md.modified().unwrap()
            }
        }
        if container_info.blob_count > 0 {
            cb(dir, &container_info);
        }
    }
    Ok(())
}

impl IXGameSave_Impl for XStub_Impl {
    unsafe fn x_game_save_initialize_provider(&self,_requesting_user: XUserHandle,_configuration_id: *const c_char,_sync_on_demand: BOOL, provider: *mut XGameSaveProviderHandle) -> HRESULT {
        // todo!()
        let value = match XGameSaveProvider::new(_requesting_user) { Ok(v) => v, Err(hr) => return hr };
        * provider = Box::into_raw(Box::new(value)) as XGameSaveProviderHandle;
        S_OK
    }

    unsafe fn x_game_save_initialize_provider_async(&self, _requesting_user: XUserHandle,configuration_id: *const c_char, sync_on_demand: BOOL, async_: *mut XAsyncBlock) -> HRESULT {
        let value = match XGameSaveProvider::new(_requesting_user) { Ok(v) => v, Err(hr) => return hr };
        unsafe { xasync::run(async_, async move {
            Ok(Box::into_raw(Box::new(value)))
        }) }
    }

    unsafe fn x_game_save_initialize_provider_result(&self, async_: *mut XAsyncBlock, provider: *mut XGameSaveProviderHandle) -> HRESULT {
        unsafe { xasync::get_result(async_, null_mut(), provider).map_or_else(|e|e, |_| S_OK) }
    }

    unsafe fn x_game_save_close_provider(&self,_provider: XGameSaveProviderHandle) -> () {
    }

    unsafe fn x_game_save_get_remaining_quota(&self,_provider: XGameSaveProviderHandle, remaining_quota: *mut i64) -> HRESULT {
        *remaining_quota = 16 * 1024 * 1024 as i64;
        S_OK
    }

    unsafe fn x_game_save_get_remaining_quota_async(&self,_provider: XGameSaveProviderHandle, async_: *mut XAsyncBlock) -> HRESULT {
        unsafe { xasync::run(async_, async {
            Ok(16 * 1024 * 1024 as i64)
        }) }
    }

    unsafe fn x_game_save_get_remaining_quota_result(&self,async_: *mut XAsyncBlock,remaining_quota: *mut i64) -> HRESULT {
        unsafe { xasync::get_result(async_, null_mut(), remaining_quota).map_or_else(|e|e, |_| S_OK) }
    }

    unsafe fn x_game_save_delete_container(&self,_provider: XGameSaveProviderHandle, container_name: *const c_char) -> HRESULT {
        println!("x_game_save_delete_container {}", CStr::from_ptr(container_name).to_string_lossy());
        S_OK
    }

    unsafe fn x_game_save_delete_container_async(&self,_provider: XGameSaveProviderHandle, container_name: *const c_char, async_: *mut XAsyncBlock) -> HRESULT {
        println!("x_game_save_delete_container {}", CStr::from_ptr(container_name).to_string_lossy());
        unsafe { xasync::run(async_, async {
            Ok(())
        } ) }
    }

    unsafe fn x_game_save_delete_container_result(&self,async_: *mut XAsyncBlock) -> HRESULT {
        unsafe { xasync::get_status(async_, false).map_or_else(|h|h, |_|S_OK) }
    }

    unsafe fn x_game_save_get_container_info(&self,_provider: XGameSaveProviderHandle, container_name: *const c_char, context: *mut c_void, callback: Option<XGameSaveContainerInfoCallback>) -> HRESULT {
        println!("x_game_save_get_container_info {}", CStr::from_ptr(container_name).to_string_lossy());
        let info = XGameSaveContainerInfo {
            name: container_name.cast(),
            blob_count: 1,
            display_name: container_name.cast(),
            total_size: 0,
            last_modified_time: 0,
            needs_sync: false,
        };
        callback.unwrap()(&info, context);
        S_OK
    }

    unsafe fn x_game_save_enumerate_container_info(&self, provider: XGameSaveProviderHandle, context: *mut c_void, callback: Option<XGameSaveContainerInfoCallback>) -> HRESULT {
        println!("x_game_save_enumerate_container_info");
        let provider_ = unsafe {
            &*(provider as *mut XGameSaveProvider)
        };
        let root = Path::new(&provider_.root);
        println!("root {}", provider_.root);
        visit_dirs(root, &|e: &Path, data| {
            let container_name = e.strip_prefix(root).unwrap();
            let cn = CString::from_str(container_name.to_str().unwrap()).unwrap();
            println!("entry {} {}", provider_.root, container_name.to_str().unwrap());
            callback.map(|f| f(&XGameSaveContainerInfo{
                blob_count: data.blob_count,
                total_size: data.total_size,
                last_modified_time: data.last_modified_time,
                display_name: cn.as_ptr(),
                name: cn.as_ptr(),
                needs_sync: data.needs_sync,
            }, context));
        }).unwrap();
        println!("ok");
        S_OK
    }

    unsafe fn x_game_save_enumerate_container_info_by_name(&self, provider: XGameSaveProviderHandle, container_name_prefix: *const c_char, context: *mut c_void, callback: Option<XGameSaveContainerInfoCallback>) -> HRESULT {
        println!("x_game_save_enumerate_container_info_by_name");
        let provider_ = unsafe {
            &*(provider as *mut XGameSaveProvider)
        };
        let container_name_prefix = CStr::from_ptr(container_name_prefix).to_str().unwrap();
        let root = Path::new(&provider_.root);
        visit_dirs(root, &|e: &Path, data| {
            let container_name = e.strip_prefix(root).unwrap();
            let cn = container_name.to_str().unwrap();
            if cn.starts_with(container_name_prefix) {
                let cn = CString::from_str(cn).unwrap();
                callback.map(|f| f(&XGameSaveContainerInfo{
                    blob_count: data.blob_count,
                    total_size: data.total_size,
                    last_modified_time: data.last_modified_time,
                    display_name: cn.as_ptr(),
                    name: cn.as_ptr(),
                    needs_sync: data.needs_sync,
                }, context));
            }
        }).unwrap();
        S_OK
    }

    unsafe fn x_game_save_create_container(&self, provider: XGameSaveProviderHandle, container_name: *const c_char, container_context: *mut XGameSaveContainerHandle) -> HRESULT {
        let provider_ = unsafe {
            &*(provider as *mut XGameSaveProvider)
        };
        let context = Box::new(XGameSaveContainer{ provider: provider as *mut XGameSaveProvider, container_name: unsafe { CStr::from_ptr(container_name).to_string_lossy().to_string() } } );
        println!("x_game_save_create_container {} {}", provider_.root, context.container_name);
        unsafe { *container_context = Box::into_raw(context) as XGameSaveContainerHandle };
        S_OK
    }

    unsafe fn x_game_save_close_container(&self, context: XGameSaveContainerHandle) -> () {
        let context = unsafe { Box::<XGameSaveContainer>::from_raw(context as *mut XGameSaveContainer) };
        println!("x_game_save_close_container {}", context.container_name)
    }

    unsafe fn x_game_save_enumerate_blob_info(&self, container: XGameSaveContainerHandle, context: *mut c_void, callback: Option<XGameSaveBlobInfoCallback>) -> HRESULT {
        let container = unsafe { &*(container as *mut XGameSaveContainer) };
        let provider = unsafe { &*container.provider };
        println!("x_game_save_enumerate_blob_info {}", container.container_name);
        let file = Path::new(&provider.root).join(&container.container_name).to_string_lossy().to_string();
        println!("{}", file);
        std::fs::create_dir_all(&file).unwrap();
        let folder = std::fs::read_dir(file).unwrap();
        let mut anyFiles = false;
        for f in folder {
            anyFiles = true;
            let f = f.unwrap();
            let c_string = CString::new(f.file_name().to_string_lossy().to_string()).unwrap();
            let info = XGameSaveBlobInfo {
                name: c_string.as_ptr(),
                size: f.metadata().unwrap().file_size() as u32,
            };
            println!("Entry {} {}", f.file_name().to_string_lossy(), info.size);
            callback.unwrap()(&info, context);
        }
        if anyFiles { S_OK } else { E_FAIL }
    }

    unsafe fn x_game_save_enumerate_blob_info_by_name(&self, container: XGameSaveContainerHandle, blob_name_prefix: *const c_char, context: *mut c_void, callback: Option<XGameSaveBlobInfoCallback>) -> HRESULT {
        let container = unsafe { &*(container as *mut XGameSaveContainer) };
        let provider = unsafe { &*container.provider };
        let file = Path::new(&provider.root).join(&container.container_name);
        let name = unsafe {
            CStr::from_ptr(blob_name_prefix).to_string_lossy()
        };
        println!("x_game_save_enumerate_blob_info_by_name {} {}", container.container_name, name);

        if let Ok(f) = std::fs::File::open(file.join(name.to_string())) {
            let info = XGameSaveBlobInfo {
                name: blob_name_prefix,
                size: f.metadata().unwrap().len() as u32,
            };
            callback.unwrap()(&info, context);
            S_OK
        } else {
            E_FAIL
        }
    }

    unsafe fn x_game_save_read_blob_data(&self, container: XGameSaveContainerHandle, blob_names: *const *mut c_char, count_of_blobs: *mut u32, blobs_size: usize, blob_data: *mut XGameSaveBlob) -> HRESULT {
        let container = unsafe { &*(container as *mut XGameSaveContainer) };
        let provider = unsafe { &*container.provider };
        let file = Path::new(&provider.root).join(&container.container_name);
        println!("x_game_save_read_blob_data {}", container.container_name);
        let mut data : *mut u8 = blob_data.add(unsafe { *count_of_blobs } as usize).cast();
        for i in 0..unsafe { *count_of_blobs } {
            println!("x_game_save_read_blob_data {} {}", container.container_name, unsafe { CStr::from_ptr(*blob_names.add(i as usize)) }.to_string_lossy());
            let info = &mut *blob_data.add(i as usize);
            let name = CStr::from_ptr(*blob_names.add(i as usize));
            let bname = name.to_bytes_with_nul();
            // advance data payload
            info.info.name = data as *mut i8;
            // copy c string
            std::slice::from_raw_parts_mut(data, bname.len()).iter_mut().zip(bname).for_each(|(a, b)| *a = *b);
            data = data.add(bname.len());

            let mut f = std::fs::File::open(file.join(name.to_string_lossy().to_string())).unwrap();
            info.info.size = f.metadata().unwrap().len() as u32;

            f.read_exact(std::slice::from_raw_parts_mut(data, info.info.size as usize)).unwrap();
            info.data = data;

            data = data.add(info.info.size as usize);

            // info.info.name

            // size->size += strlen(info->name) + 1; // length + null
            // size->size += info->size + sizeof(XGameSaveBlob);
        }
        S_OK
    }

    unsafe fn x_game_save_read_blob_data_async(&self,container: XGameSaveContainerHandle, blob_names: *const *mut c_char, count_of_blobs: u32, async_: *mut XAsyncBlock) -> HRESULT {
        let container = unsafe { &*(container as *mut XGameSaveContainer) };
        println!("x_game_save_read_blob_data_async {}", container.container_name);

        let mut blob_names_a = Vec::with_capacity(count_of_blobs as usize);
        for i in 0..unsafe { count_of_blobs } {
            println!("x_game_save_read_blob_data_async {} {}", container.container_name, unsafe { CStr::from_ptr(*blob_names.add(i as usize)) }.to_string_lossy());
            blob_names_a.push(unsafe { CStr::from_ptr(*blob_names.add(i as usize)) }.to_string_lossy().to_string());
        }
        let provider = unsafe { &*container.provider };
        let file = Path::new(&provider.root).join(&container.container_name).clone();
        unsafe { xasync::run_dyn(async_, async move {
            let mut required_size = 0;
            for i in 0..count_of_blobs {
                required_size += std::mem::size_of::<XGameSaveBlob>();
                let name = &blob_names_a[i as usize];
                required_size += name.len() + 1;
                let Ok(f) = std::fs::File::open(file.join(name)) else {
                    return Err(E_FAIL);
                };
                required_size += f.metadata().unwrap().len() as usize;
            }

            Ok((move |buffer, size| {
                println!("x_game_save_read_blob_data_async size {size}");
                let blob_data = buffer as *mut XGameSaveBlob;
                let mut data : *mut u8 = blob_data.add(count_of_blobs as usize).cast();
                for i in 0..count_of_blobs {
                    let info = &mut *blob_data.add(i as usize);
                    let name = &blob_names_a[i as usize];
                    // advance data payload
                    info.info.name = data as *mut i8;
                    // copy c string‚
                    std::slice::from_raw_parts_mut(data, name.len()).iter_mut().zip(name.as_bytes()).for_each(|(a, b)| *a = *b);
                    data = data.add(name.len());
                    // null byte
                    *data = 0;
                    data = data.add(1);
                    let mut f = std::fs::File::open(file.join(name)).unwrap();
                    info.info.size = f.metadata().unwrap().len() as u32;

                    f.read_exact(std::slice::from_raw_parts_mut(data, info.info.size as usize)).unwrap();
                    info.data = data;

                    data = data.add(info.info.size as usize);
                }
                println!("x_game_save_read_blob_data_async done size {size}");
                required_size as usize
            }, required_size))
        }) }
    }

    unsafe fn x_game_save_read_blob_data_result(&self,async_: *mut XAsyncBlock, blobs_size: usize, blob_data: *mut XGameSaveBlob, count_of_blobs: *mut u32) -> HRESULT {
        let mut used_size = 0;
        unsafe { xasync::get_result_dyn(async_, null_mut(), blobs_size, blob_data as *mut c_void, &mut used_size) }.unwrap();
        unsafe { *count_of_blobs = 1 as u32 };
        S_OK
    }

    unsafe fn x_game_save_create_update(&self, container: XGameSaveContainerHandle, container_display_name: *const c_char, update_context: *mut XGameSaveUpdateHandle) -> HRESULT {
        let raw = container as *mut XGameSaveContainer;
        let container = unsafe { &*(raw) };
        let update = Box::new(XGameSaveUpdate{ container: raw, container_display_name: CStr::from_ptr(container_display_name).to_string_lossy().to_string() } );
        println!("x_game_save_create_update {} {}", container.container_name, update.container_display_name);
        unsafe { *update_context = Box::into_raw(update) as XGameSaveUpdateHandle };
        S_OK
    }

    unsafe fn x_game_save_close_update(&self, context: XGameSaveUpdateHandle) -> () {
        unsafe { Box::<XGameSaveUpdate>::from_raw(context as *mut XGameSaveUpdate) };
    }

    unsafe fn x_game_save_submit_blob_write(&self, update_context: XGameSaveUpdateHandle, blob_name: *const c_char, data: *const u8, byte_count: usize) -> HRESULT {
        let update_context = unsafe { &*(update_context as *mut XGameSaveUpdate) };
        println!("x_game_save_submit_blob_write {} {} byte_count {byte_count}", update_context.container_display_name, CStr::from_ptr(blob_name).to_string_lossy());
        let container = unsafe { &*update_context.container };
        let provider = unsafe { &*container.provider };
        let file = Path::new(&provider.root).join(&container.container_name).join(CStr::from_ptr(blob_name).to_string_lossy().to_string());
        println!("{}", file.to_string_lossy());
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        let mut f = std::fs::File::create(file).unwrap();
        f.write_all(std::slice::from_raw_parts(data, byte_count)).unwrap();
        f.sync_all().unwrap();
        S_OK
    }

    unsafe fn x_game_save_submit_blob_delete(&self, update_context: XGameSaveUpdateHandle, blob_name: *const c_char) -> HRESULT {
        let update_context = unsafe { &*(update_context as *mut XGameSaveUpdate) };
        println!("x_game_save_submit_blob_delete {} {}", update_context.container_display_name, CStr::from_ptr(blob_name).to_string_lossy());
        S_OK
    }

    unsafe fn x_game_save_submit_update(&self, update_context: XGameSaveUpdateHandle) -> HRESULT {
        let update_context = unsafe { &*(update_context as *mut XGameSaveUpdate) };
        println!("x_game_save_submit_update_async {}", update_context.container_display_name);
        S_OK
    }

    unsafe fn x_game_save_submit_update_async(&self, update_context: XGameSaveUpdateHandle, async_: *mut XAsyncBlock) -> HRESULT {
        unsafe { xasync::run(async_, async move {
            let update_context = unsafe { &*(update_context as *mut XGameSaveUpdate) };
            println!("x_game_save_submit_update_async {}", update_context.container_display_name);
            Ok(())
        }) }
    }

    unsafe fn x_game_save_submit_update_result(&self,_async_: *mut XAsyncBlock) -> HRESULT {
        println!("x_game_save_submit_update_result");
        unsafe { xasync::get_status(_async_, false).map_or_else(|e|e, |_| S_OK) }
    }

    unsafe fn x_game_save_files_get_folder_with_ui_async(&self,_requesting_user: XUserHandle,_configuration_id: *const char,_async_: *mut XAsyncBlock) -> HRESULT {
        todo!()
    }

    unsafe fn x_game_save_files_get_folder_with_ui_result(&self,_async_: *mut XAsyncBlock,_folder_size: usize,_folder_result: *mut char) -> HRESULT {
        todo!()
    }

    unsafe fn x_game_save_files_get_remaining_quota(&self,_user_context: XUserHandle,_configuration_id: *const char,_remaining_quota: *mut i64) -> HRESULT {
        todo!()
    }

    unsafe fn __reserved_slot_33(&self,) {
        todo!()
    }

    unsafe fn __reserved_slot_34(&self,) {
        todo!()
    }

    unsafe fn __reserved_slot_35(&self,) {
        todo!()
    }

    unsafe fn __reserved_slot_36(&self,) {
        todo!()
    }

    unsafe fn __reserved_slot_37(&self,) {
        todo!()
    }

    unsafe fn __reserved_slot_38(&self,) {
        todo!()
    }

    unsafe fn __reserved_slot_39(&self,) {
        todo!()
    }

    unsafe fn __reserved_slot_40(&self,) {
        todo!()
    }

    unsafe fn __reserved_slot_41(&self,) {
        todo!()
    }

    unsafe fn __reserved_slot_42(&self,) {
        todo!()
    }

    unsafe fn __reserved_slot_43(&self,) {
        todo!()
    }

    unsafe fn __reserved_slot_44(&self,) {
        todo!()
    }

    unsafe fn __reserved_slot_45(&self,) {
        todo!()
    }

    unsafe fn __reserved_slot_46(&self,) {
        todo!()
    }

    unsafe fn __reserved_slot_47(&self,) {
        todo!()
    }

    unsafe fn __reserved_slot_48(&self,) {
        todo!()
    }

    unsafe fn __reserved_slot_49(&self,) {
        todo!()
    }
}

impl IXGameSave2_Impl for XStub_Impl {

}

impl IXPackage_Impl for XStub_Impl {
    unsafe fn x_package_get_current_process_package_identifier(&self, buffer_size: usize, buffer: *mut c_char) -> HRESULT {
        println!("x_package_get_current_process_package_identifier");
        // todo!()
        std::slice::from_raw_parts_mut(buffer as *mut u8, 6).copy_from_slice(c"local".to_bytes_with_nul());
        S_OK
    }

    unsafe fn x_package_is_packaged_process(&self,) -> BOOL {
        println!("x_package_is_packaged_process");
        // false for halo campaign evolved
        false.into()
        // true for the halo mcc
        // true.into()
    }

    unsafe fn x_package_create_installation_monitor(&self, package_identifier: *const c_char, selector_count: u32, selectors: *mut XPackageChunkSelector,_minimum_update_interval_ms: u32,_queue: XTaskQueueHandle, installation_monitor: *mut XPackageInstallationMonitorHandle) -> HRESULT {
        // todo!()
        println!("x_package_create_installation_monitor: {}", CStr::from_ptr(package_identifier).to_string_lossy());
        *installation_monitor = 1;
        S_OK
    }

    unsafe fn x_package_close_installation_monitor_handle(&self,_installation_monitor: XPackageInstallationMonitorHandle) -> () {
        println!("x_package_close_installation_monitor_handle");
        // todo!()
    }

    unsafe fn x_package_get_installation_progress(&self,_installation_monitor: XPackageInstallationMonitorHandle, progress: *mut XPackageInstallationProgress) -> () {
        println!("x_package_get_installation_progress");
        *progress = XPackageInstallationProgress { total_bytes: 1000, installed_bytes: 1000, launch_bytes: 1000, launchable: true, completed: true };
    }

    unsafe fn x_package_update_installation_monitor(&self,_installation_monitor: XPackageInstallationMonitorHandle) -> BOOL {
        println!("x_package_update_installation_monitor");
        // todo!()
        true.into()
    }

    unsafe fn x_package_register_installation_progress_changed(&self, installation_monitor: XPackageInstallationMonitorHandle, context: *mut c_void, callback: Option<XPackageInstallationProgressCallback> ,_token: *mut XTaskQueueRegistrationToken) -> HRESULT {
        println!("x_package_register_installation_progress_changed");
        callback.unwrap()(context, installation_monitor);
        S_OK
    }

    unsafe fn x_package_unregister_installation_progress_changed(&self,_installation_monitor: XPackageInstallationMonitorHandle,_token: XTaskQueueRegistrationToken,_wait: BOOL) -> BOOL {
        println!("x_package_unregister_installation_progress_changed");
        true.into()
    }

    unsafe fn x_package_get_user_locale(&self, locale_size: usize, locale: *mut c_char) -> HRESULT {
        assert!(locale_size >= 3);
        std::slice::from_raw_parts_mut(locale as *mut u8, 3).copy_from_slice(c"DE".to_bytes_with_nul());
        S_OK
    }

    unsafe fn x_package_find_chunk_availability(&self,_package_identifier: *const c_char, selector_count: u32, selectors: *mut XPackageChunkSelector, availability: *mut XPackageChunkAvailability) -> HRESULT {
        println!("x_package_find_chunk_availability {}", if selector_count == 1 && (&*selectors).type_ == XPackageChunkSelectorType::Chunk { (&*selectors).data.chunk_id.to_string() } else { CStr::from_ptr((&*selectors).data.tag).to_string_lossy().to_string() });
        *availability = XPackageChunkAvailability::Ready;
        S_OK
    }

    unsafe fn x_package_enumerate_chunk_availability(&self,_package_identifier: *const c_char,_type_: XPackageChunkSelectorType,_context: *mut c_void,_callback: Option<XPackageChunkAvailabilityCallback>) -> HRESULT {
        println!("x_package_enumerate_chunk_availability");
        S_OK
    }

    unsafe fn x_package_change_chunk_install_order(&self,_package_identifier: *const c_char,_selector_count: u32,_selectors: *mut XPackageChunkSelector) -> HRESULT {
        todo!()
    }

    unsafe fn x_package_install_chunks(&self,_package_identifier: *const c_char,_selector_count: u32,_selectors: *mut XPackageChunkSelector,_minimum_update_interval_ms: u32,_suppress_user_confirmation: BOOL,_queue: XTaskQueueHandle,_installation_monitor: *mut XPackageInstallationMonitorHandle) -> HRESULT {
        todo!()
    }

    unsafe fn x_package_install_chunks_async(&self,_package_identifier: *const c_char,_selector_count: u32,_selectors: *mut XPackageChunkSelector,_minimum_update_interval_ms: u32,_suppress_user_confirmation: BOOL,_async_block: *mut XAsyncBlock) -> HRESULT {
        todo!()
    }

    unsafe fn x_package_install_chunks_result(&self,_async_block: *mut XAsyncBlock,_installation_monitor: *mut XPackageInstallationMonitorHandle) -> HRESULT {
        todo!()
    }

    unsafe fn x_package_estimate_download_size(&self,_package_identifier: *const c_char,_selector_count: u32,_selectors: *mut XPackageChunkSelector,_download_size: *mut u64,_should_present_user_confirmation: *mut BOOL) -> HRESULT {
        todo!()
    }

    unsafe fn x_package_uninstall_chunks(&self,_package_identifier: *const c_char,_selector_count: u32,_selectors: *mut XPackageChunkSelector) -> HRESULT {
        todo!()
    }

    unsafe fn __reserved_slot_20(&self,) {
        todo!()
    }

    unsafe fn __reserved_slot_21(&self,) {
        todo!()
    }

    unsafe fn x_package_unregister_package_installed(&self,_token: XTaskQueueRegistrationToken,_wait: BOOL) -> BOOL {
        true.into()
    }

    unsafe fn __reserved_slot_23(&self,) {
        todo!()
    }

    unsafe fn x_package_get_mount_path_size(&self,_mount: XPackageMountHandle,_path_size: *mut usize) -> HRESULT {
        todo!()
    }

    unsafe fn x_package_get_mount_path(&self,_mount: XPackageMountHandle,_path_size: usize,_path: *mut c_char) -> HRESULT {
        todo!()
    }

    unsafe fn x_package_close_mount_handle(&self,_mount: XPackageMountHandle) -> () {
        todo!()
    }

    unsafe fn __reserved_slot_27(&self,) {
        todo!()
    }

    // //XPackageEnumeratePackages
    // unsafe fn __reserved_slot_28(&self,) {
    //     println!("XPackageEnumeratePackages");
    //     todo!()
    // }
    unsafe fn x_package_enumerate_packages2(self: &Self, _kind: XPackageKind, _scope: XPackageEnumerationScope, context: *mut c_void, callback: Option<XPackageEnumerationCallback>) -> HRESULT {
        println!("x_package_enumerate_packages2");
        // let details = XPackageDetails {
        //     package_identifier: c"Halo4".as_ptr(),
        //     version: 100,
        //     kind: XPackageKind::Content,
        //     display_name: c"Halo4".as_ptr(),
        //     description: c"Halo4".as_ptr(),
        //     publisher: c"MS".as_ptr(),
        //     store_id: c"9NN6VS9SPW2R".as_ptr(),
        //     installing: false,
        //     index: 0,
        //     count: 2,
        //     age_restricted: false,
        //     title_i_d: c"9NN6VS9SPW2R".as_ptr(),
        // };
        // unsafe { callback.unwrap()(context, &details) };
        let details = XPackageDetails {
            package_identifier: c"HaloReach".as_ptr(),
            version: 100,
            kind: XPackageKind::Content,
            display_name: c"HaloReach".as_ptr(),
            description: c"HaloReach".as_ptr(),
            publisher: c"MS".as_ptr(),
            store_id: c"9N9RNPBLR7X3".as_ptr(),
            installing: false,
            index: 1,
            count: 2,
            age_restricted: false,
            title_i_d: c"9N9RNPBLR7X3".as_ptr(),
        };

        unsafe { callback.unwrap()(context, &details) };
        S_OK
    }

    // XPackageRegisterPackageInstalled
    unsafe fn __reserved_slot_29(&self,) {
        println!("XPackageRegisterPackageInstalled");
    }

    unsafe fn x_package_get_write_stats(&self,_write_stats: *mut XPackageWriteStats) -> HRESULT {
        todo!()
    }

    unsafe fn __reserved_slot_31(&self,) {
        todo!()
    }

    unsafe fn x_package_uninstall_u_w_p_instance(&self,_package_name: *const c_char) -> HRESULT {
        todo!()
    }

    unsafe fn x_package_enumerate_features(&self,_package_identifier: *const c_char,_context: *mut c_void,_callback: Option<XPackageFeatureEnumerationCallback>) -> HRESULT {
        println!("x_package_enumerate_features");
        S_OK
    }

    unsafe fn x_package_uninstall_package(&self,_package_identifier: *const c_char) -> BOOL {
        todo!()
    }

    unsafe fn __reserved_slot_35(&self,) {
        todo!()
    }

    unsafe fn __reserved_slot_36(&self,) {
        todo!()
    }

    unsafe fn x_package_mount_with_ui_async(&self,_package_identifier: *const char,_async_: *mut XAsyncBlock) -> HRESULT {
        todo!()
    }

    unsafe fn x_package_mount_with_ui_result(&self,_async_: *mut XAsyncBlock,_mount: *mut XPackageMountHandle) -> HRESULT {
        todo!()
    }

    unsafe fn x_package_enumerate_packages(&self,_kind: XPackageKind,_scope: XPackageEnumerationScope,_context: *mut c_void,_callback: Option<XPackageEnumerationCallback>) -> HRESULT {
        // todo!()
        println!("x_package_enumerate_packages");
        S_OK
    }

    unsafe fn x_package_register_package_installed(&self,_queue: XTaskQueueHandle,_context: *mut c_void,_callback: Option<XPackageInstalledCallback> ,_token: *mut XTaskQueueRegistrationToken) -> HRESULT {
        // todo!()
        S_OK
    }
}

impl IXPackage2_Impl for XStub_Impl { }
