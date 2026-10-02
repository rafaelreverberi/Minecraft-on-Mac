use std::borrow::Cow;
use std::cell::Cell;
use std::env::{home_dir, temp_dir};
use std::ffi::{CStr, c_char, c_void};
use std::mem::size_of;
use std::path::Path;
use std::ptr::{null, null_mut};
use std::sync::{Mutex, OnceLock};
use windows::libloaderapi::GetModuleFileNameW;
use windows::minwindef::MAX_PATH;
use windows_core::{BOOL, GUID, HRESULT, IUnknown, Interface, PCWSTR, implement, interface};
use xodus::secrets;
use xodus::tokens::TokenManager;

const CLSID_XSTORE: GUID = GUID::from_u128(0x0dd112ac_7c24_448c_b92b_3960fb5bd30c);
const CLSID_XNETWORKING: GUID = GUID::from_u128(0x37e56907_2f10_41e8_b72f_36edb185331a);
const CLSID_XPERSISTENT_LOCAL_STORAGE: GUID =
    GUID::from_u128(0xf4faf4d4_2d04_4fce_b3e0_474a713a3e84);

const CLSID_XUSER: GUID = GUID::from_u128(0x01acd177_91f9_4763_a38e_ccbb55ce32e0);
const CLSID_XUserDEVICE: GUID = GUID::from_u128(0x7d824997_10dc_45ab_86b7_2737767c0bf1);
const CLSID_XPACKAGE: GUID = GUID::from_u128(0xaf406016_e850_4aa8_a88d_2f3dcb9dac7e);
const CLSID_XGAMESAVE: GUID = GUID::from_u128(0x704c3f58_e629_4cc2_b197_30511b996fe2);

use crate::stub::XStub;
use crate::threading::{IXAsync, XAsyncBlock, XTaskQueueHandle, XTaskQueueRegistrationToken};
use crate::user::{IXUser, XUser, XUserHandle, do_license_token};
use crate::xasync::get_result;
use crate::xlaunch::IXLaunch;
use crate::xnetworking::{
    IXNetworking, IXNetworking_Impl, XNetworkingConfigurationSetting,
    XNetworkingConnectivityCostHint, XNetworkingConnectivityHint, XNetworkingConnectivityLevelHint,
    XNetworkingSecurityInformation, XNetworkingStatisticsBuffer, XNetworkingStatisticsType,
};
use crate::xpackage::{IXPackage, XPackageMountHandle};
use crate::xpersistedlocalstorage::{
    IXPersistentLocalStorage_Impl, IXPersistentLocalStorage2, IXPersistentLocalStorage2_Impl,
    XPersistentLocalStorageSpaceInfo,
};
use crate::xstore::{
    self, IXStore, IXStore_Impl, IXStore2, IXStore2_1_Impl, IXStore2_Impl, XStoreAddonLicense,
    XStoreCanAcquireLicenseResult, XStoreConsumableResult, XStoreContextHandle, XStoreGameLicense,
    XStoreGameLicenseChangedCallback, XStoreLicenseHandle, XStorePackageLicenseLostCallback,
    XStorePackageUpdate, XStorePrice, XStoreProduct, XStoreProductKind, XStoreProductQueryCallback,
    XStoreProductQueryHandle, XStoreRateAndReviewResult,
};
use crate::xsystem::IXSystem;
use crate::{E_FAIL, E_NOTIMPL, results::*, threading, xasync};

#[interface("8836fe87-edb9-4fe3-8dad-05f0d2cd5b40")]
pub unsafe trait IXFeature: IUnknown {
    unsafe fn xgame_runtime_is_feature_available(&self, feature: u32) -> bool;
}

#[implement(IXFeature)]
pub struct XFeature;

impl IXFeature_Impl for XFeature_Impl {
    unsafe fn xgame_runtime_is_feature_available(&self, feature: u32) -> bool {
        return true || feature != 10;
    }
}

#[implement(IXPersistentLocalStorage2)]
pub struct XPersistentLocalStorage {
    tmp_path: String,
}
impl IXPersistentLocalStorage2_Impl for XPersistentLocalStorage_Impl {}

impl IXPersistentLocalStorage_Impl for XPersistentLocalStorage_Impl {
    unsafe fn x_persistent_local_storage_get_path_size(&self, path_size: *mut usize) -> HRESULT {
        println!(
            "x_persistent_local_storage_get_path_size: {}",
            self.tmp_path
        );
        unsafe {
            *path_size = self.tmp_path.len() + 1;
        }
        return S_OK;
    }

    unsafe fn x_persistent_local_storage_get_path(
        &self,
        path_size: usize,
        path: *mut c_char,
        path_used: *mut usize,
    ) -> HRESULT {
        println!("x_persistent_local_storage_get_path: {}", self.tmp_path);
        let bytes = self.tmp_path.as_bytes();
        let len = bytes.len().min(path_size.saturating_sub(1));
        for (index, byte) in bytes.iter().copied().take(len).enumerate() {
            unsafe {
                *path.add(index) = byte as c_char;
            }
        }
        if path_size != 0 {
            unsafe {
                *path.add(len) = 0;
            }
        }
        unsafe {
            *path_used = len + 1;
        }
        return S_OK;
    }

    unsafe fn x_persistent_local_storage_get_space_info(
        &self,
        info: *mut XPersistentLocalStorageSpaceInfo,
    ) -> HRESULT {
        println!(
            "x_persistent_local_storage_get_space_info: {}",
            self.tmp_path
        );
        unsafe {
            *info = XPersistentLocalStorageSpaceInfo {
                available_free_bytes: 1024 * 1024 * 1024,
                total_free_bytes: 1024 * 1024 * 1024,
                used_bytes: 512 * 1024 * 1024,
                total_bytes: 2 * 1024 * 1024 * 1024,
            };
        }
        return S_OK;
    }

    unsafe fn x_persistent_local_storage_prompt_user_for_space_async(
        &self,
        _requested_bytes: u64,
        _async_block: *mut XAsyncBlock,
    ) -> HRESULT {
        todo!()
    }

    unsafe fn x_persistent_local_storage_prompt_user_for_space_result(
        &self,
        _async_block: *mut XAsyncBlock,
    ) -> HRESULT {
        todo!()
    }

    unsafe fn x_persistent_local_storage_mount_for_package(
        &self,
        _package_identifier: *const c_char,
        _mount_handle: *mut XPackageMountHandle,
    ) -> HRESULT {
        todo!()
    }
}

#[interface("5c48dedf-0b67-4492-a4b5-6829b8e796e1")]
pub unsafe trait IXStoreAlias1: xstore::IXStore {}

#[interface("b09d803c-2414-4a05-82c6-66dfdc9e9a44")]
pub unsafe trait IXStoreAlias2: xstore::IXStore {}

#[interface("2d42fea5-e71d-4b76-97cd-c50afbb3ae5d")]
pub unsafe trait IXStoreAlias3: xstore::IXStore {}

// XNetworkingConnectivityHintChangedCallback
pub type XNetworkingConnectivityHintChangedCallback = unsafe extern "system" fn(
    context: *mut c_void,
    connectivity_hint: *const XNetworkingConnectivityHint,
) -> ();

// XNetworkingPreferredLocalUdpMultiplayerPortChangedCallback
pub type XNetworkingPreferredLocalUdpMultiplayerPortChangedCallback =
    unsafe extern "system" fn(
        context: *mut c_void,
        preferred_local_udp_multiplayer_port: u16,
    ) -> ();

#[interface("bf2346b2-39af-4658-b5ea-44713c7e83b3")]
pub unsafe trait IXNetworking2: IXNetworking {}

#[implement(xstore::IXStore, IXStoreAlias1, IXStoreAlias2, IXStoreAlias3, IXStore2)]
pub struct XStoreObject;

fn debug_cstr<'t>(c: *const c_char) -> Cow<'t, str> {
    if c.is_null() {
        return Cow::Borrowed("");
    }
    unsafe { CStr::from_ptr(c).to_string_lossy() }
}

impl IXStore_Impl for XStoreObject_Impl {
    unsafe fn x_store_create_context(
        &self,
        _user: XUserHandle,
        store_context_handle: *mut XStoreContextHandle,
    ) -> HRESULT {
        println!("x_store_create_context");
        unsafe {
            *store_context_handle = 1;
        };
        HRESULT(0)
    }

    unsafe fn x_store_query_game_license_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_query_game_license_async");
        unsafe {
            xasync::run_sync(async_.cast(), move || {
                // println!("storeContextHandle: {storeContextHandle}");
                return Ok(XStoreGameLicense::default());
            })
        }
    }

    unsafe fn x_store_query_game_license_result(
        &self,
        async_: *mut XAsyncBlock,
        license: *mut XStoreGameLicense,
    ) -> HRESULT {
        println!("XStoreQueryGameLicenseResult");
        if async_.is_null() || license.is_null() {
            return E_POINTER;
        }

        let mut payload = XStoreGameLicense::default();
        match unsafe { get_result(async_, null_mut(), &mut payload) } {
            Ok(_) => {
                unsafe {
                    *license = payload;
                }
                S_OK
            }
            Err(hr) => return hr,
        }
    }

    unsafe fn x_store_close_context_handle(
        &self,
        _store_context_handle: XStoreContextHandle,
    ) -> () {
    }

    unsafe fn x_store_query_associated_products_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        _product_kinds: XStoreProductKind,
        _max_items_to_retrieve_per_page: u32,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_query_associated_products_async");
        E_NOTIMPL
    }

    unsafe fn x_store_query_associated_products_result(
        &self,
        _async_: *mut XAsyncBlock,
        _product_query_handle: *mut XStoreProductQueryHandle,
    ) -> HRESULT {
        println!("x_store_query_associated_products_result");
        E_NOTIMPL
    }

    unsafe fn x_store_query_products_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        _product_kinds: XStoreProductKind,
        _store_ids: *const *mut c_char,
        _store_ids_count: usize,
        _action_filters: *const *mut c_char,
        _action_filters_count: usize,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_query_products_async");
        E_NOTIMPL
    }

    unsafe fn x_store_query_products_result(
        &self,
        _async_: *mut XAsyncBlock,
        _product_query_handle: *mut XStoreProductQueryHandle,
    ) -> HRESULT {
        println!("x_store_query_products_result");
        E_NOTIMPL
    }

    unsafe fn x_store_query_entitled_products_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        _product_kinds: XStoreProductKind,
        _max_items_to_retrieve_per_page: u32,
        async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_query_entitled_products_async");
        unsafe { xasync::run(async_, async { Ok(0 as XStoreProductQueryHandle) }) }
    }

    unsafe fn x_store_query_entitled_products_result(
        &self,
        async_: *mut XAsyncBlock,
        product_query_handle: *mut XStoreProductQueryHandle,
    ) -> HRESULT {
        println!("x_store_query_entitled_products_result");
        unsafe {
            xasync::get_result(async_, null_mut(), product_query_handle)
                .map_or_else(|a| a, |_| S_OK)
        }
    }

    unsafe fn x_store_query_product_for_current_game_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        E_NOTIMPL
    }

    unsafe fn x_store_query_product_for_current_game_result(
        &self,
        _async_: *mut XAsyncBlock,
        _product_query_handle: *mut XStoreProductQueryHandle,
    ) -> HRESULT {
        E_NOTIMPL
    }

    unsafe fn x_store_query_product_for_package_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        _product_kinds: XStoreProductKind,
        _package_identifier: *const c_char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        E_NOTIMPL
    }

    unsafe fn x_store_query_product_for_package_result(
        &self,
        _async_: *mut XAsyncBlock,
        _product_query_handle: *mut XStoreProductQueryHandle,
    ) -> HRESULT {
        E_NOTIMPL
    }

    unsafe fn x_store_enumerate_products_query(
        &self,
        _product_query_handle: XStoreProductQueryHandle,
        context: *mut c_void,
        callback: Option<XStoreProductQueryCallback>,
    ) -> HRESULT {
        println!("x_store_enumerate_products_query");
        // let product = XStoreProduct {
        //     store_id: c"9NN6VS9SPW2R".as_ptr(),
        //     title: c"Halo 4".as_ptr(),
        //     description: c"Erleben Sie die triumphale Wiederkehr des Master Chief, um ein uraltes Böses zu bekämpfen, das auf Rache und Vernichtung sinnt. Als Gestrandeter auf einer mysteriösen Welt sieht er sich neuen Feinden und einer tödlichen Technologie gegenüber, die die Welt für immer verändern werden.".as_ptr(),
        //     language: c"de-DE".as_ptr(),
        //     in_app_offer_token: null(),
        //     link_uri: null_mut(),
        //     product_kind: XStoreProductKind::Durable,
        //     price: XStorePrice {
        //         price: 32.0,
        //         base_price: 32.0,
        //         currency_code: c"EUR".as_ptr(),
        //         formatted_base_price: [0i8; 16],
        //         recurrence_price: 0.0,
        //         formatted_price: [0i8; 16],
        //         formatted_recurrence_price: [0i8; 16],
        //         is_on_sale: false,
        //         sale_end_date: 0,
        //     },
        //     has_digital_download: true,
        //     is_in_user_collection: true,
        //     keywords_count: 0,
        //     keywords: null(),
        //     skus_count: 0,
        //     skus: null_mut(),
        //     images_count: 0,
        //     images: null_mut(),
        //     videos_count: 0,
        //     videos: null_mut(),
        // };
        // callback.unwrap()(&product, context);
        let product: XStoreProduct = XStoreProduct {
            store_id: c"9N9RNPBLR7X3".as_ptr(),
            title: c"HaloReach".as_ptr(),
            description: c"Erleben Sie die triumphale Wiederkehr des Master Chief, um ein uraltes Böses zu bekämpfen, das auf Rache und Vernichtung sinnt. Als Gestrandeter auf einer mysteriösen Welt sieht er sich neuen Feinden und einer tödlichen Technologie gegenüber, die die Welt für immer verändern werden.".as_ptr(),
            language: c"de-DE".as_ptr(),
            in_app_offer_token: null(),
            link_uri: null_mut(),
            product_kind: XStoreProductKind::Durable,
            price: XStorePrice {
                price: 32.0,
                base_price: 32.0,
                currency_code: c"EUR".as_ptr(),
                formatted_base_price: [0i8; 16],
                recurrence_price: 0.0,
                formatted_price: [0i8; 16],
                formatted_recurrence_price: [0i8; 16],
                is_on_sale: false,
                sale_end_date: 0,
            },
            has_digital_download: true,
            is_in_user_collection: true,
            keywords_count: 0,
            keywords: null(),
            skus_count: 0,
            skus: null_mut(),
            images_count: 0,
            images: null_mut(),
            videos_count: 0,
            videos: null_mut(),
        };
        callback.unwrap()(&product, context);

        S_OK
    }

    unsafe fn x_store_products_query_has_more_pages(
        &self,
        _product_query_handle: XStoreProductQueryHandle,
    ) -> BOOL {
        println!("x_store_products_query_has_more_pages");
        false.into()
    }

    unsafe fn x_store_products_query_next_page_async(
        &self,
        _product_query_handle: XStoreProductQueryHandle,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_products_query_next_page_async");
        E_NOTIMPL
    }

    unsafe fn x_store_products_query_next_page_result(
        &self,
        _async_: *mut XAsyncBlock,
        _product_query_handle: *mut XStoreProductQueryHandle,
    ) -> HRESULT {
        E_NOTIMPL
    }

    unsafe fn x_store_close_products_query_handle(
        &self,
        _product_query_handle: XStoreProductQueryHandle,
    ) -> () {
        println!("x_store_close_products_query_handle");
    }

    unsafe fn x_store_acquire_license_for_package_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        _package_identifier: *const c_char,
        async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_acquire_license_for_package_async");
        unsafe { xasync::run(async_, async { Ok(1 as XStoreLicenseHandle) }) }
    }

    unsafe fn x_store_acquire_license_for_package_result(
        &self,
        async_: *mut XAsyncBlock,
        store_license_handle: *mut XStoreLicenseHandle,
    ) -> HRESULT {
        println!("x_store_acquire_license_for_package_result");
        unsafe { xasync::get_result(async_, null_mut(), store_license_handle) }
            .map_or_else(|h| h, |_| S_OK)
    }

    unsafe fn x_store_is_license_valid(&self, _store_license_handle: XStoreLicenseHandle) -> BOOL {
        println!("x_store_is_license_valid");
        true.into()
    }

    unsafe fn x_store_close_license_handle(
        &self,
        _store_license_handle: XStoreLicenseHandle,
    ) -> () {
        println!("x_store_close_license_handle");
    }

    unsafe fn x_store_can_acquire_license_for_store_id_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        _store_product_id: *const c_char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_can_acquire_license_for_store_id_async");
        E_NOTIMPL
    }

    unsafe fn x_store_can_acquire_license_for_store_id_result(
        &self,
        _async_: *mut XAsyncBlock,
        _store_can_acquire_license: *mut XStoreCanAcquireLicenseResult,
    ) -> HRESULT {
        println!("x_store_can_acquire_license_for_store_id_result");
        E_NOTIMPL
    }

    unsafe fn x_store_can_acquire_license_for_package_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        _package_identifier: *const c_char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_can_acquire_license_for_package_async");
        E_NOTIMPL
    }

    unsafe fn x_store_can_acquire_license_for_package_result(
        &self,
        _async_: *mut XAsyncBlock,
        _store_can_acquire_license: *mut XStoreCanAcquireLicenseResult,
    ) -> HRESULT {
        println!("x_store_can_acquire_license_for_package_result");
        E_NOTIMPL
    }

    unsafe fn x_store_query_add_on_licenses_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_query_add_on_licenses_async");
        E_NOTIMPL
    }

    unsafe fn x_store_query_add_on_licenses_result_count(
        &self,
        _async_: *mut XAsyncBlock,
        _count: *mut u32,
    ) -> HRESULT {
        println!("x_store_query_add_on_licenses_result_count");
        E_NOTIMPL
    }

    unsafe fn x_store_query_add_on_licenses_result(
        &self,
        _async_: *mut XAsyncBlock,
        _count: u32,
        _add_on_licenses: *mut XStoreAddonLicense,
    ) -> HRESULT {
        println!("x_store_query_add_on_licenses_result");
        E_NOTIMPL
    }

    unsafe fn x_store_query_consumable_balance_remaining_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        _store_product_id: *const c_char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_query_consumable_balance_remaining_async");
        E_NOTIMPL
    }

    unsafe fn x_store_query_consumable_balance_remaining_result(
        &self,
        _async_: *mut XAsyncBlock,
        _consumable_result: *mut XStoreConsumableResult,
    ) -> HRESULT {
        println!("x_store_query_consumable_balance_remaining_result");
        E_NOTIMPL
    }

    unsafe fn __reserved_slot_35(&self) {
        todo!()
    }

    unsafe fn x_store_report_consumable_fulfillment_result(
        &self,
        _async_: *mut XAsyncBlock,
        _consumable_result: *mut XStoreConsumableResult,
    ) -> HRESULT {
        println!("x_store_report_consumable_fulfillment_result");
        E_NOTIMPL
    }

    unsafe fn x_store_get_user_collections_id_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        _service_ticket: *const c_char,
        _publisher_user_id: *const c_char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_get_user_collections_id_async");
        E_NOTIMPL
    }

    unsafe fn x_store_get_user_collections_id_result_size(
        &self,
        _async_: *mut XAsyncBlock,
        _size: *mut usize,
    ) -> HRESULT {
        println!("x_store_get_user_collections_id_result_size");
        E_NOTIMPL
    }

    unsafe fn x_store_get_user_collections_id_result(
        &self,
        _async_: *mut XAsyncBlock,
        _size: usize,
        _result: *mut c_char,
    ) -> HRESULT {
        println!("x_store_get_user_collections_id_result");
        E_NOTIMPL
    }

    unsafe fn x_store_get_user_purchase_id_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        _service_ticket: *const c_char,
        _publisher_user_id: *const c_char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_get_user_purchase_id_async");
        E_NOTIMPL
    }

    unsafe fn x_store_get_user_purchase_id_result_size(
        &self,
        _async_: *mut XAsyncBlock,
        _size: *mut usize,
    ) -> HRESULT {
        println!("x_store_get_user_purchase_id_result_size");
        E_NOTIMPL
    }

    unsafe fn x_store_get_user_purchase_id_result(
        &self,
        _async_: *mut XAsyncBlock,
        _size: usize,
        _result: *mut c_char,
    ) -> HRESULT {
        println!("x_store_get_user_purchase_id_result");
        E_NOTIMPL
    }

    unsafe fn x_store_query_license_token_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        product_ids: *const *mut c_char,
        product_ids_count: usize,
        custom_developer_string: *const c_char,
        async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_query_license_token_async");
        // E_NOTIMPL
        let mut products = Vec::with_capacity(product_ids_count);
        unsafe {
            for i in 0..product_ids_count {
                products.push(
                    CStr::from_ptr(*product_ids.add(i))
                        .to_str()
                        .unwrap()
                        .to_owned(),
                );
            }
            let dev_str = CStr::from_ptr(custom_developer_string)
                .to_str()
                .unwrap()
                .to_owned();
            xasync::run_dyn(async_, async move {
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                let token = runtime
                    .spawn(async {
                        let client = reqwest::Client::builder()
                            .use_rustls_tls()
                            .http1_only()
                            .connection_verbose(false)
                            .pool_max_idle_per_host(0)
                            .connect_timeout(std::time::Duration::from_secs(5))
                            .timeout(std::time::Duration::from_secs(10))
                            .build()
                            .unwrap(); //
                        std::env::set_var("HOME", std::env::var_os("USERPROFILE").unwrap());
                        println!("{}", std::env::var_os("HOME").unwrap().to_string_lossy());
                        secrets::init_secrets().expect("Unable to initialize credentials");
                        let tokens: TokenManager = TokenManager::with_keychain_and_memory();

                        let token = do_license_token(&client, &tokens, products, dev_str)
                            .await
                            .unwrap();
                        crate::diag("license token generated");

                        token
                    })
                    .await
                    .unwrap();

                let req_size = token.len() + 1;
                Ok::<_, HRESULT>((
                    move |b: *mut c_void, s: usize| {
                        std::ptr::copy_nonoverlapping(token.as_ptr(), b as *mut u8, token.len());
                        unsafe { *((b as *mut u8).add(token.len())) = 0 };
                        return s;
                    },
                    req_size,
                ))
            })
        }
    }

    unsafe fn x_store_query_license_token_result_size(
        &self,
        async_: *mut XAsyncBlock,
        s: *mut usize,
    ) -> HRESULT {
        println!("x_store_query_license_token_result_size");
        if s.is_null() {
            return E_POINTER;
        }
        match unsafe { xasync::get_result_size(async_) } {
            Err(hr) => hr,
            Ok(size) => unsafe {
                *s = size;
                println!("x_store_query_license_token_result_size {}", size);
                S_OK
            },
        }
    }

    unsafe fn x_store_query_license_token_result(
        &self,
        async_: *mut XAsyncBlock,
        size: usize,
        result: *mut c_char,
    ) -> HRESULT {
        println!("x_store_query_license_token_result");
        // E_NOTIMPL
        match unsafe {
            xasync::get_result_dyn(async_, null_mut(), size, result as *mut c_void, null_mut())
        } {
            Err(hr) => return hr,
            _ => {
                println!(
                    "x_store_query_license_token_result: {}",
                    CStr::from_ptr(result).to_string_lossy()
                );
                S_OK
            }
        }
    }

    unsafe fn __reserved_slot_46(&self) {
        todo!()
    }

    unsafe fn __reserved_slot_47(&self) {
        todo!()
    }

    unsafe fn __reserved_slot_48(&self) {
        todo!()
    }

    unsafe fn x_store_show_purchase_u_i_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        store_id: *const c_char,
        name: *const c_char,
        extended_json_data: *const c_char,
        async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!(
            "x_store_show_purchase_u_i_async {} {}, {}",
            debug_cstr(store_id),
            debug_cstr(name),
            debug_cstr(extended_json_data)
        );
        unsafe { xasync::run(async_, async { Ok(()) }) }
    }

    unsafe fn x_store_show_purchase_u_i_result(&self, async_: *mut XAsyncBlock) -> HRESULT {
        println!("x_store_show_purchase_u_i_result");
        unsafe { xasync::get_status(async_, false).map_or_else(|h| h, |_| S_OK) }
    }

    unsafe fn x_store_show_rate_and_review_u_i_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_show_rate_and_review_u_i_async");
        E_NOTIMPL
    }

    unsafe fn x_store_show_rate_and_review_u_i_result(
        &self,
        _async_: *mut XAsyncBlock,
        _result: *mut XStoreRateAndReviewResult,
    ) -> HRESULT {
        println!("x_store_show_rate_and_review_u_i_result");
        E_NOTIMPL
    }

    unsafe fn x_store_show_redeem_token_u_i_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        _token: *const c_char,
        _allowed_store_ids: *const *mut c_char,
        _allowed_store_ids_count: usize,
        _disallow_csv_redemption: BOOL,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_show_redeem_token_u_i_async");
        E_NOTIMPL
    }

    unsafe fn x_store_show_redeem_token_u_i_result(&self, _async_: *mut XAsyncBlock) -> HRESULT {
        println!("x_store_show_redeem_token_u_i_result");
        E_NOTIMPL
    }

    unsafe fn x_store_query_game_and_dlc_package_updates_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_query_game_and_dlc_package_updates_async");

        unsafe { xasync::run(async_, async { Ok(()) }) }
    }

    unsafe fn x_store_query_game_and_dlc_package_updates_result_count(
        &self,
        _async_: *mut XAsyncBlock,
        count: *mut u32,
    ) -> HRESULT {
        println!("x_store_query_game_and_dlc_package_updates_result_count");
        *count = 0;
        S_OK
    }

    unsafe fn x_store_query_game_and_dlc_package_updates_result(
        &self,
        async_: *mut XAsyncBlock,
        _count: u32,
        _package_updates: *mut XStorePackageUpdate,
    ) -> HRESULT {
        println!("x_store_query_game_and_dlc_package_updates_result");
        unsafe { xasync::get_status(async_, false).map_or_else(|h| h, |_| S_OK) }
    }

    unsafe fn x_store_download_package_updates_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        _package_identifiers: *const *mut c_char,
        _package_identifiers_count: usize,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_download_package_updates_async");
        E_NOTIMPL
    }

    unsafe fn x_store_download_package_updates_result(&self, _async_: *mut XAsyncBlock) -> HRESULT {
        println!("x_store_download_package_updates_result");
        E_NOTIMPL
    }

    unsafe fn x_store_download_and_install_package_updates_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        _package_identifiers: *const *mut c_char,
        _package_identifiers_count: usize,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_download_and_install_package_updates_async");
        E_NOTIMPL
    }

    unsafe fn x_store_download_and_install_package_updates_result(
        &self,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_download_and_install_package_updates_result");
        E_NOTIMPL
    }

    unsafe fn x_store_download_and_install_packages_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        store_ids: *const *mut c_char,
        store_ids_count: usize,
        async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_download_and_install_packages_async");
        unsafe {
            xasync::run_dyn(async_, async {
                let req_size = 0;
                Ok::<_, HRESULT>((
                    move |b: *mut c_void, s: usize| {
                        return s;
                    },
                    req_size,
                ))
            })
        }
    }

    unsafe fn x_store_download_and_install_packages_result_count(
        &self,
        async_: *mut XAsyncBlock,
        count: *mut u32,
    ) -> HRESULT {
        println!("x_store_download_and_install_packages_result_count");
        unsafe {
            xasync::get_result_size(async_).map_or_else(
                |h| h,
                |s| {
                    *count = s as u32;
                    S_OK
                },
            )
        }
    }

    unsafe fn x_store_download_and_install_packages_result(
        &self,
        _async_: *mut XAsyncBlock,
        count: u32,
        package_identifiers: *mut *mut c_char,
    ) -> HRESULT {
        println!("x_store_download_and_install_packages_result");
        S_OK
    }

    unsafe fn x_store_query_package_identifier(
        &self,
        _store_id: *const c_char,
        _size: usize,
        _package_identifier: *mut c_char,
    ) -> HRESULT {
        println!("x_store_query_package_identifier");
        E_NOTIMPL
    }

    unsafe fn x_store_register_game_license_changed(
        &self,
        _store_context_handle: XStoreContextHandle,
        _queue: XTaskQueueHandle,
        _context: *mut c_void,
        _callback: Option<XStoreGameLicenseChangedCallback>,
        _token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT {
        println!("x_store_register_game_license_changed");
        S_OK
    }

    unsafe fn x_store_unregister_game_license_changed(
        &self,
        _store_context_handle: XStoreContextHandle,
        _token: XTaskQueueRegistrationToken,
        _wait: BOOL,
    ) -> BOOL {
        println!("x_store_unregister_game_license_changed");
        true.into()
    }

    unsafe fn x_store_register_package_license_lost(
        &self,
        _license_handle: XStoreLicenseHandle,
        _queue: XTaskQueueHandle,
        _context: *mut c_void,
        _callback: Option<XStorePackageLicenseLostCallback>,
        _token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT {
        println!("x_store_register_package_license_lost");
        S_OK.into()
    }

    unsafe fn x_store_unregister_package_license_lost(
        &self,
        _license_handle: XStoreLicenseHandle,
        _token: XTaskQueueRegistrationToken,
        _wait: BOOL,
    ) -> BOOL {
        println!("x_store_unregister_package_license_lost");
        true.into()
    }

    unsafe fn __reserved_slot_70(&self) {
        todo!()
    }

    unsafe fn x_store_acquire_license_for_durables_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        _store_id: *const c_char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_acquire_license_for_durables_async");
        E_NOTIMPL
    }

    unsafe fn x_store_acquire_license_for_durables_result(
        &self,
        _async_: *mut XAsyncBlock,
        _store_license_handle: *mut XStoreLicenseHandle,
    ) -> HRESULT {
        println!("x_store_acquire_license_for_durables_result");
        E_NOTIMPL
    }

    unsafe fn x_store_show_associated_products_u_i_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        _store_id: *const c_char,
        _product_kinds: XStoreProductKind,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_show_associated_products_u_i_async");
        E_NOTIMPL
    }

    unsafe fn x_store_show_associated_products_u_i_result(
        &self,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_show_associated_products_u_i_result");
        E_NOTIMPL
    }

    unsafe fn x_store_show_product_page_u_i_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        _store_id: *const c_char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_show_product_page_u_i_async");
        E_NOTIMPL
    }

    unsafe fn x_store_show_product_page_u_i_result(&self, _async_: *mut XAsyncBlock) -> HRESULT {
        println!("x_store_show_product_page_u_i_result");
        E_NOTIMPL
    }

    unsafe fn x_store_query_associated_products_for_store_id_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        _store_id: *const c_char,
        _product_kinds: XStoreProductKind,
        _max_items_to_retrieve_per_page: u32,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_query_associated_products_for_store_id_async");
        E_NOTIMPL
    }

    unsafe fn x_store_query_associated_products_for_store_id_result(
        &self,
        _async_: *mut XAsyncBlock,
        _product_query_handle: *mut XStoreProductQueryHandle,
    ) -> HRESULT {
        println!("x_store_query_associated_products_for_store_id_result");
        E_NOTIMPL
    }

    unsafe fn x_store_query_package_updates_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        _package_identifiers: *const *mut c_char,
        _package_identifiers_count: usize,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_query_package_updates_async");
        E_NOTIMPL
    }

    unsafe fn x_store_query_package_updates_result_count(
        &self,
        _async_: *mut XAsyncBlock,
        _count: *mut u32,
    ) -> HRESULT {
        println!("x_store_query_package_updates_result_count");
        E_NOTIMPL
    }

    unsafe fn x_store_query_package_updates_result(
        &self,
        _async_: *mut XAsyncBlock,
        _count: u32,
        _package_updates: *mut XStorePackageUpdate,
    ) -> HRESULT {
        println!("x_store_query_package_updates_result");
        E_NOTIMPL
    }

    unsafe fn x_store_show_gifting_u_i_async(
        &self,
        _store_context_handle: XStoreContextHandle,
        _store_id: *const c_char,
        _name: *const c_char,
        _extended_json_data: *const c_char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        println!("x_store_show_gifting_u_i_asyncx_store_show_gifting_u_i_async");
        E_NOTIMPL
    }

    unsafe fn x_store_show_gifting_u_i_result(&self, _async_: *mut XAsyncBlock) -> HRESULT {
        println!("x_store_show_gifting_u_i_result");
        E_NOTIMPL
    }
}

impl IXStore2_Impl for XStoreObject_Impl {}
impl IXStore2_1_Impl for XStoreObject_Impl {}
impl IXStoreAlias1_Impl for XStoreObject_Impl {}
impl IXStoreAlias2_Impl for XStoreObject_Impl {}
impl IXStoreAlias3_Impl for XStoreObject_Impl {}

#[implement(IXNetworking, IXNetworking2)]
pub struct XNetworkingObject;

impl IXNetworking_Impl for XNetworkingObject_Impl {
    unsafe fn x_networking_get_connectivity_hint(
        &self,
        connectivity_hint: *mut XNetworkingConnectivityHint,
    ) -> HRESULT {
        if connectivity_hint.is_null() {
            return E_POINTER;
        }
        unsafe {
            *connectivity_hint = XNetworkingConnectivityHint {
                connectivity_level: XNetworkingConnectivityLevelHint::InternetAccess,
                connectivity_cost: XNetworkingConnectivityCostHint::Unrestricted,
                iana_interface_type: 6,
                network_initialized: true.into(),
                approaching_data_limit: false.into(),
                over_data_limit: false.into(),
                roaming: false.into(),
            };
        }
        S_OK
    }

    unsafe fn x_networking_query_security_information_for_url_async(
        &self,
        url: *const c_char,
        async_block: *mut XAsyncBlock,
    ) -> HRESULT {
        let url = unsafe { CStr::from_ptr(url) };
        println!(
            "XNetworkingQuerySecurityInformationForUrlAsync {}",
            url.to_string_lossy()
        );
        unsafe {
            let storage = url.to_str().unwrap_or_default();
            xasync::run_sync(async_block, move || {
                println!(
                    "XNetworkingQuerySecurityInformationForUrlAsync: storage: {}",
                    storage
                );
                Ok(XNetworkingSecurityInformation {
                    enabled_http_security_protocol_flags: 0x00000080
                        | 0x00000200
                        | 0x00000800
                        | 0x00002000,
                    thumbprint_count: 0,
                    thumbprints: null_mut(),
                })
            })
        }
    }

    unsafe fn x_networking_query_security_information_for_url_async_result(
        &self,
        async_block: *mut XAsyncBlock,
        security_information_buffer_byte_count: usize,
        security_information_buffer_byte_count_used: *mut usize,
        security_information_buffer: *mut u8,
        security_information: *mut *mut XNetworkingSecurityInformation,
    ) -> HRESULT {
        if security_information_buffer_byte_count < size_of::<XNetworkingSecurityInformation>() {
            return E_FAIL;
        }
        if !security_information_buffer_byte_count_used.is_null() {
            unsafe { *security_information_buffer_byte_count_used = 0 };
        }
        match unsafe {
            get_result(
                async_block,
                null_mut(),
                security_information_buffer.cast::<XNetworkingSecurityInformation>(),
            )
        } {
            Ok(_) => {
                if !security_information_buffer_byte_count_used.is_null() {
                    unsafe {
                        *security_information_buffer_byte_count_used =
                            size_of::<XNetworkingSecurityInformation>()
                    };
                }
                unsafe { *security_information = security_information_buffer.cast() };
                S_OK
            }
            Err(hr) => hr,
        }
    }

    unsafe fn x_networking_query_security_information_for_url_async_result_size(
        &self,
        async_block: *mut XAsyncBlock,
        security_information_buffer_byte_count: *mut usize,
    ) -> HRESULT {
        let r = unsafe { xasync::get_result_size(async_block) };
        match r {
            Ok(size) => unsafe {
                *security_information_buffer_byte_count = size;
                S_OK
            },
            Err(hr) => hr,
        }
    }

    unsafe fn x_networking_query_security_information_for_url_utf16_async(
        &self,
        url: *const u16,
        async_block: *mut XAsyncBlock,
    ) -> HRESULT {
        let url = PCWSTR::from_raw(url);
        println!(
            "XNetworkingQuerySecurityInformationForUrlUtf16Async {} thread: {:?}",
            unsafe { url.to_string() }.unwrap(),
            std::thread::current().id(),
        );
        unsafe {
            let storage = url.to_string().unwrap();
            xasync::run_sync(async_block, move || {
                println!(
                    "XNetworkingQuerySecurityInformationForUrlUtf16Async: storage: {} thread: {:?}",
                    storage,
                    std::thread::current().id()
                );
                Ok(XNetworkingSecurityInformation {
                    enabled_http_security_protocol_flags: 0x00000080
                        | 0x00000200
                        | 0x00000800
                        | 0x00002000,
                    thumbprint_count: 0,
                    thumbprints: null_mut(),
                })
            })
        }
    }

    unsafe fn x_networking_query_security_information_for_url_utf16_async_result(
        &self,
        async_block: *mut XAsyncBlock,
        security_information_buffer_byte_count: usize,
        security_information_buffer_byte_count_used: *mut usize,
        security_information_buffer: *mut u8,
        security_information: *mut *mut XNetworkingSecurityInformation,
    ) -> HRESULT {
        println!(
            "XNetworkingQuerySecurityInformationForUrlUtf16AsyncResult thread: {:?}",
            std::thread::current().id()
        );
        if security_information_buffer_byte_count < size_of::<XNetworkingSecurityInformation>() {
            return E_FAIL;
        }
        if !security_information_buffer_byte_count_used.is_null() {
            unsafe { *security_information_buffer_byte_count_used = 0 };
        }
        match unsafe {
            get_result(
                async_block,
                null_mut(),
                security_information_buffer.cast::<XNetworkingSecurityInformation>(),
            )
        } {
            Ok(_) => {
                if !security_information_buffer_byte_count_used.is_null() {
                    unsafe {
                        *security_information_buffer_byte_count_used =
                            size_of::<XNetworkingSecurityInformation>()
                    };
                }
                unsafe { *security_information = security_information_buffer.cast() };
                println!(
                    "XNetworkingQuerySecurityInformationForUrlUtf16AsyncResult: OK thread: {:?}",
                    std::thread::current().id()
                );
                S_OK
            }
            Err(hr) => hr,
        }
    }

    unsafe fn x_networking_query_security_information_for_url_utf16_async_result_size(
        &self,
        async_block: *mut XAsyncBlock,
        security_information_buffer_byte_count: *mut usize,
    ) -> HRESULT {
        println!(
            "XNetworkingQuerySecurityInformationForUrlUtf16AsyncResultSize thread: {:?}",
            std::thread::current().id()
        );
        let r = unsafe { xasync::get_result_size(async_block) };
        match r {
            Ok(size) => unsafe {
                *security_information_buffer_byte_count = size;
                S_OK
            },
            Err(hr) => hr,
        }
    }

    unsafe fn x_networking_register_connectivity_hint_changed(
        &self,
        _queue: XTaskQueueHandle,
        context: *mut c_void,
        callback: Option<XNetworkingConnectivityHintChangedCallback>,
        _token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT {
        if let Some(callback) = callback {
            // println!("XNetworkingRegisterConnectivityHintChanged");
            unsafe {
                callback(
                    context,
                    &XNetworkingConnectivityHint {
                        connectivity_level: XNetworkingConnectivityLevelHint::InternetAccess,
                        connectivity_cost: XNetworkingConnectivityCostHint::Unrestricted,
                        iana_interface_type: 6,
                        network_initialized: true.into(),
                        approaching_data_limit: false.into(),
                        over_data_limit: false.into(),
                        roaming: false.into(),
                    },
                )
            };
        }
        S_OK
    }

    unsafe fn x_networking_verify_server_certificate(
        &self,
        _request_handle: *mut c_void,
        _security_information: *const XNetworkingSecurityInformation,
    ) -> HRESULT {
        S_OK
    }

    unsafe fn x_networking_query_preferred_local_udp_multiplayer_port(
        &self,
        preferred_local_udp_multiplayer_port: *mut u16,
    ) -> HRESULT {
        *preferred_local_udp_multiplayer_port = 1600u16;
        S_OK
    }

    unsafe fn x_networking_query_preferred_local_udp_multiplayer_port_async(
        &self,
        async_block: *mut XAsyncBlock,
    ) -> HRESULT {
        unsafe {
            xasync::run_sync(async_block, || {
                return Ok(1600u16);
            })
        }
    }

    unsafe fn x_networking_query_preferred_local_udp_multiplayer_port_async_result(
        &self,
        async_block: *mut XAsyncBlock,
        preferred_local_udp_multiplayer_port: *mut u16,
    ) -> HRESULT {
        unsafe {
            xasync::get_result(
                async_block,
                null_mut(),
                preferred_local_udp_multiplayer_port,
            )
        }
        .map(|_| S_OK)
        .unwrap_or_else(|e| e)
    }

    unsafe fn x_networking_register_preferred_local_udp_multiplayer_port_changed(
        &self,
        _queue: XTaskQueueHandle,
        _context: *mut c_void,
        _callback: Option<XNetworkingPreferredLocalUdpMultiplayerPortChangedCallback>,
        _token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT {
        todo!()
    }

    unsafe fn x_networking_unregister_preferred_local_udp_multiplayer_port_changed(
        &self,
        _token: XTaskQueueRegistrationToken,
        _wait: BOOL,
    ) -> BOOL {
        todo!()
    }

    unsafe fn x_networking_unregister_connectivity_hint_changed(
        &self,
        _token: XTaskQueueRegistrationToken,
        _wait: BOOL,
    ) -> BOOL {
        true.into()
    }

    unsafe fn x_networking_query_configuration_setting(
        &self,
        _configuration_setting: XNetworkingConfigurationSetting,
        _value: *mut u64,
    ) -> HRESULT {
        todo!()
    }

    unsafe fn x_networking_set_configuration_setting(
        &self,
        _configuration_parameter: XNetworkingConfigurationSetting,
        _value: u64,
    ) -> HRESULT {
        todo!()
    }

    unsafe fn x_networking_query_statistics(
        &self,
        _statistics_type: XNetworkingStatisticsType,
        _statistics_buffer: *mut XNetworkingStatisticsBuffer,
    ) -> HRESULT {
        todo!()
    }
}

impl IXNetworking2_Impl for XNetworkingObject_Impl {}

struct GlobalInterface<T>(T);

unsafe impl<T> Send for GlobalInterface<T> {}
unsafe impl<T> Sync for GlobalInterface<T> {}

static XFEATURE_SINGLETON: OnceLock<GlobalInterface<IXFeature>> = OnceLock::new();
static XSTORE_SINGLETON: OnceLock<GlobalInterface<IXStore>> = OnceLock::new();
static XNETWORKING_SINGLETON: OnceLock<GlobalInterface<IXNetworking>> = OnceLock::new();
static XPERSISTENT_LOCAL_STORAGE_SINGLETON: OnceLock<GlobalInterface<IXPersistentLocalStorage2>> =
    OnceLock::new();
static XUSER_SINGLETON: OnceLock<GlobalInterface<IXUser>> = OnceLock::new();
static XASYNC_SINGLETON: OnceLock<GlobalInterface<IXAsync>> = OnceLock::new();
static XSTUB_SINGLETON: OnceLock<GlobalInterface<IXPackage>> = OnceLock::new();

fn xfeature_singleton() -> &'static IXFeature {
    &XFEATURE_SINGLETON
        .get_or_init(|| GlobalInterface(XFeature.into()))
        .0
}

fn xstore_singleton() -> &'static IXStore {
    &XSTORE_SINGLETON
        .get_or_init(|| GlobalInterface(XStoreObject.into()))
        .0
}

fn xnetworking_singleton() -> &'static IXNetworking {
    &XNETWORKING_SINGLETON
        .get_or_init(|| GlobalInterface(XNetworkingObject.into()))
        .0
}

fn xpersistent_local_storage_singleton() -> &'static IXPersistentLocalStorage2 {
    &XPERSISTENT_LOCAL_STORAGE_SINGLETON
        .get_or_init(|| {
            let mut path = [0u16; MAX_PATH as usize];
            let len = unsafe { GetModuleFileNameW(None, &mut path) };
            let path = String::from_utf16_lossy(&path[..len as usize]);
            let path = Path::new(&path).parent();

            GlobalInterface(
                XPersistentLocalStorage {
                    tmp_path: path.unwrap().to_str().unwrap().to_owned()/*home_dir().unwrap().join("path").to_string_lossy().to_string()*/,
                }
                .into(),
            )
        })
        .0
}

fn xuser_singleton() -> &'static IXUser {
    &XUSER_SINGLETON
        .get_or_init(|| {
            GlobalInterface(
                XUser {
                    runtime: tokio::runtime::Builder::new_multi_thread()
                        .enable_all()
                        .build()
                        .unwrap(),
                    handle: Cell::new(None),
                }
                .into(),
            )
        })
        .0
}

fn xasync_singleton() -> &'static IXAsync {
    &XASYNC_SINGLETON
        .get_or_init(|| {
            let async_: threading::IXAsync = threading::XAsync {
                process_queue: Mutex::new(null_mut()),
                runtime: tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap(),
            }
            .into();

            let mut queue: *mut c_void = std::ptr::null_mut();
            let _ = unsafe {
                async_.x_task_queue_create(
                    threading::XTaskQueueDispatchMode::ThreadPool,
                    threading::XTaskQueueDispatchMode::ThreadPool,
                    &mut queue,
                )
            };
            let _ = unsafe { async_.x_task_queue_set_current_process_task_queue(queue) };
            GlobalInterface(async_)
        })
        .0
}

fn xstub_singleton() -> &'static IXPackage {
    &XSTUB_SINGLETON
        .get_or_init(|| GlobalInterface(XStub {}.into()))
        .0
}

fn query<T: Interface + Clone>(
    object: &T,
    interface_id: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    if interface_id.is_null() || out.is_null() {
        return E_POINTER;
    }
    let object = object.clone();
    let interface_id = unsafe { *interface_id };
    if unsafe { object.query(&interface_id, out) }.is_ok() {
        // println!("query: ack {:#32x}", interface_id.to_u128());
        S_OK
    } else {
        println!("query: nack {:?}", interface_id);
        unsafe {
            *out = std::ptr::null_mut();
        }
        E_NOINTERFACE
    }
}

pub fn query_api_impl(
    runtime_class_id: *const GUID,
    interface_id: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    if runtime_class_id.is_null() || interface_id.is_null() || out.is_null() {
        return E_POINTER;
    }

    let class_id = unsafe { *runtime_class_id };
    // println!("query_api_impl: {:?}", class_id);
    let res = match class_id {
        IXFeature::IID => {
            // println!("query_api_impl: {:#32x} {:#32x}", class_id.to_u128(), unsafe { *interface_id }.to_u128());
            query(xfeature_singleton(), interface_id, out)
        }
        CLSID_XSTORE => {
            // println!("query_api_impl: {:#32x} {:#32x}", class_id.to_u128(), unsafe { *interface_id }.to_u128());
            query(xstore_singleton(), interface_id, out)
        }
        CLSID_XNETWORKING => {
            // println!(
            //     "query_api_impl: {:#32x} {:#32x}",
            //     class_id.to_u128(),
            //     unsafe { *interface_id }.to_u128()
            // );
            query(xnetworking_singleton(), interface_id, out)
        }
        CLSID_XPERSISTENT_LOCAL_STORAGE => {
            query(xpersistent_local_storage_singleton(), interface_id, out)
        }
        #[cfg(feature = "xuser")]
        CLSID_XUSER => query(xuser_singleton(), interface_id, out),
        // #[cfg(feature = "xuser")]
        // CLSID_XUSERDEVICE => query(xuser_singleton(), interface_id, out),
        #[cfg(feature = "xasync")]
        xasync::CLSID_XASYNC => query(xasync_singleton(), interface_id, out),
        CLSID_XGAMESAVE => query(xstub_singleton(), interface_id, out),
        CLSID_XPACKAGE => query(xstub_singleton(), interface_id, out),
        IXSystem::IID => query(xstub_singleton(), interface_id, out),
        IXLaunch::IID => query(xstub_singleton(), interface_id, out),
        _ => {
            let resp = crate::delegated_query_api_impl(runtime_class_id, interface_id, out);
            if resp.is_err() {
                return match class_id {
                    CLSID_XUSER => query(xuser_singleton(), interface_id, out),
                    xasync::CLSID_XASYNC => query(xasync_singleton(), interface_id, out),
                    _ => resp,
                };
            }
            resp
        }
    };
    res
}

#[cfg(test)]
mod tests {
    use std::ffi::c_void;
    use std::ptr::null_mut;

    use crate::com::{IXStore, XStoreGameLicense, get_result, query_api_impl};
    use crate::xasync::{XAsyncBlock, get_status, run};
    use crate::{
        E_FAIL, InitializeApiImplEx2, UninitializeApiImpl, set_delegated_dll_path_for_test,
    };
    use windows_core::{GUID, HRESULT, Interface};

    #[test]
    fn test() {
        let mut out: *mut c_void = std::ptr::null_mut();
        let hr = query_api_impl(
            &crate::com::CLSID_XSTORE,
            &crate::com::IXStore::IID,
            &mut out,
        );

        assert_eq!(hr, HRESULT(0));

        let store: IXStore = unsafe { IXStore::from_raw(out) };

        unsafe {
            let mut store_ctx: u64 = 0;
            let hr = store.x_store_create_context(null_mut(), &mut store_ctx);
            assert_eq!(hr, HRESULT(0));
            let hr = store.x_store_query_game_license_async(store_ctx, std::ptr::null_mut());
            assert_eq!(hr, HRESULT(0));
        };
    }

    #[test]
    #[ignore = "requires xgameruntime.gdk.dll delegate support in the Wine environment"]
    fn query_game_license_async_blocks_via_xasync() {
        let init_hr = InitializeApiImplEx2(2604, 100000, 10, std::ptr::null_mut());
        assert_eq!(init_hr, HRESULT(0));

        let mut out = std::ptr::null_mut();
        let hr = query_api_impl(
            &crate::com::CLSID_XSTORE,
            &crate::com::IXStore::IID,
            &mut out,
        );
        assert_eq!(hr, HRESULT(0));

        let store: IXStore = unsafe { IXStore::from_raw(out) };
        let mut store_ctx: u64 = 0;
        let hr = unsafe { store.x_store_create_context(null_mut(), &mut store_ctx) };
        assert_eq!(hr, HRESULT(0));

        let mut async_block = XAsyncBlock {
            queue: std::ptr::null_mut(),
            context: std::ptr::null_mut(),
            callback: None,
            internal: [0; std::mem::size_of::<*mut c_void>() * 4],
        };
        let hr = unsafe { store.x_store_query_game_license_async(store_ctx, &mut async_block) };
        assert_eq!(hr, HRESULT(0));

        unsafe { get_status(&mut async_block, true) }.unwrap();

        let mut license = XStoreGameLicense::default();
        let result_hr =
            unsafe { store.x_store_query_game_license_result(&mut async_block, &mut license) };
        assert_eq!(result_hr, HRESULT(0));
        // assert_eq!(read_c_string(&license.skuStoreId), "TRIAL-SKU-001");
        assert!(license.is_active);
        assert!(!license.is_trial_owned_by_this_user);
        assert!(!license.is_trial);
        assert!(!license.is_disc_license);
        assert_eq!(license.trial_time_remaining_in_seconds, 0);
        // assert_eq!(read_c_string(&license.trialUniqueId), "trial-license");

        let mut async_block = XAsyncBlock {
            queue: std::ptr::null_mut(),
            context: std::ptr::null_mut(),
            callback: None,
            internal: [0; std::mem::size_of::<*mut c_void>() * 4],
        };

        let tokio = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to create Tokio runtime");

        let handle = tokio.handle().clone();
        #[derive(Debug)]
        struct Payload {
            v: i32,
            v2: i64,
            v3: GUID,
        }

        let hr = unsafe {
            run(&mut async_block, async move {
                println!("starting");

                let task = handle.spawn(async {
                    let client = reqwest::Client::new();

                    let response = client
                        .get("http://google.com")
                        .send()
                        .await
                        .map_err(|_| E_FAIL)?;

                    println!("finished {}", response.status());

                    Ok::<Payload, HRESULT>(Payload {
                        v: 0,
                        v2: 323,
                        v3: GUID::zeroed(),
                    })
                });

                task.await.map_err(|_| E_FAIL)?
            })
        };
        assert_eq!(hr, HRESULT(0));

        unsafe { get_status(&mut async_block, true) }.unwrap();

        let mut payload: Payload = Payload {
            v: 0,
            v2: 0,
            v3: GUID::zeroed(),
        };
        unsafe { get_result(&mut async_block, std::ptr::null(), &mut payload) }.unwrap();

        println!("res {:?}", payload);

        assert_eq!(payload.v, 0);
        assert_eq!(payload.v2, 323);
        assert_eq!(payload.v3, GUID::zeroed());

        let uninit_hr = UninitializeApiImpl();
        assert_eq!(uninit_hr, HRESULT(0));
        set_delegated_dll_path_for_test(None);
    }
}
