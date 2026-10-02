use std::ffi::{c_char, c_void};

use libc::time_t;
use windows_core::{BOOL, HRESULT, IUnknown, interface};

use crate::{
    threading::{XTaskQueueHandle, XTaskQueueRegistrationToken},
    user::XUserHandle,
    xasync::XAsyncBlock,
};

const PRICE_MAX_SIZE: usize = 16;

#[repr(C)]
pub struct XStorePrice {
    pub base_price: f32,
    pub price: f32,
    pub recurrence_price: f32,
    pub currency_code: *const c_char,
    pub formatted_base_price: [c_char; PRICE_MAX_SIZE],
    pub formatted_price: [c_char; PRICE_MAX_SIZE],
    pub formatted_recurrence_price: [c_char; PRICE_MAX_SIZE],
    pub is_on_sale: bool,
    pub sale_end_date: time_t,
}

pub type XStoreContextHandle = u64;
pub type XStoreProductQueryHandle = u64;
pub type XStoreLicenseHandle = u64;

#[repr(u32)]
pub enum XStoreCanLicenseStatus {
    NotLicensableToUser = 0,
    Licensable = 1,
    LicenseActionNotApplicableToProduct = 2,
}
#[repr(u32)]
pub enum XStoreDurationUnit {
    Minute = 0,
    Hour = 1,
    Day = 2,
    Week = 3,
    Month = 4,
    Year = 5,
}
#[repr(u32)]
pub enum XStoreProductKind {
    None = 0x00,
    Consumable = 0x01,
    Durable = 0x02,
    Game = 0x04,
    Pass = 0x08,
    UnmanagedConsumable = 0x10,
}

#[repr(C)]
pub struct XStoreAvailability {
    pub availability_id: *const c_char,
    pub price: XStorePrice,
    pub end_date: libc::time_t,
}
#[repr(C)]
pub struct XStoreCollectionData {
    pub acquired_date: libc::time_t,
    pub start_date: libc::time_t,
    pub end_date: libc::time_t,
    pub is_trial: bool,
    pub trial_time_remaining_in_seconds: u32,
    pub quantity: u32,
    pub campaign_id: *const c_char,
    pub developer_offer_id: *const c_char,
}
#[repr(C)]
pub struct XStoreConsumableResult {
    pub quantity: u32,
}
#[repr(C)]
pub struct XStoreImage {
    pub uri: *const c_char,
    pub height: u32,
    pub width: u32,
    pub caption: *const c_char,
    pub image_purpose_tag: *const c_char,
}
#[repr(C)]
pub struct XStoreProduct {
    pub store_id: *const c_char,
    pub title: *const c_char,
    pub description: *const c_char,
    pub language: *const c_char,
    pub in_app_offer_token: *const c_char,
    pub link_uri: *mut c_char,
    pub product_kind: XStoreProductKind,
    pub price: XStorePrice,
    pub has_digital_download: bool,
    pub is_in_user_collection: bool,
    pub keywords_count: u32,
    pub keywords: *const *mut c_char,
    pub skus_count: u32,
    pub skus: *mut XStoreSku,
    pub images_count: u32,
    pub images: *mut XStoreImage,
    pub videos_count: u32,
    pub videos: *mut XStoreVideo,
}
#[repr(C)]
pub struct XStoreRateAndReviewResult {
    pub was_updated: bool,
}
#[repr(C)]
pub struct XStoreSku {
    pub sku_id: *const c_char,
    pub title: *const c_char,
    pub description: *const c_char,
    pub language: *const c_char,
    pub price: XStorePrice,
    pub is_trial: bool,
    pub is_in_user_collection: bool,
    pub collection_data: XStoreCollectionData,
    pub is_subscription: bool,
    pub subscription_info: XStoreSubscriptionInfo,
    pub bundled_skus_count: u32,
    pub bundled_skus: *const *mut c_char,
    pub images_count: u32,
    pub images: *mut XStoreImage,
    pub videos_count: u32,
    pub videos: *mut XStoreVideo,
    pub availabilities_count: u32,
    pub availabilities: *mut XStoreAvailability,
}
#[repr(C)]
pub struct XStoreSubscriptionInfo {
    pub has_trial_period: bool,
    pub trial_period_unit: XStoreDurationUnit,
    pub trial_period: u32,
    pub billing_period_unit: XStoreDurationUnit,
    pub billing_period: u32,
}
#[repr(C)]
pub struct XStoreVideo {
    pub uri: *const c_char,
    pub height: u32,
    pub width: u32,
    pub caption: *const c_char,
    pub video_purpose_tag: *const c_char,
    pub preview_image: XStoreImage,
}

// XStorePackageLicenseLostCallback
pub type XStorePackageLicenseLostCallback = unsafe extern "system" fn(_context: *mut c_void) -> ();
// XStoreGameLicenseChangedCallback
pub type XStoreGameLicenseChangedCallback = unsafe extern "system" fn(_context: *mut c_void) -> ();
// XStoreProductQueryCallback
pub type XStoreProductQueryCallback =
    unsafe extern "system" fn(_product: *const XStoreProduct, _context: *mut c_void) -> BOOL;

#[repr(C)]
pub struct XStorePackageUpdate;

// from docs??
const STORE_SKU_ID_SIZE: usize = 18;
const TRIAL_UNIQUE_ID_MAX_SIZE: usize = 64;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XStoreGameLicense {
    pub sku_store_id: [c_char; STORE_SKU_ID_SIZE],
    pub is_active: bool,
    pub is_trial_owned_by_this_user: bool,
    pub is_disc_license: bool,
    pub is_trial: bool,
    pub trial_time_remaining_in_seconds: u32,
    pub trial_unique_id: [c_char; TRIAL_UNIQUE_ID_MAX_SIZE],
    pub expiration_date: i64,
}

impl Default for XStoreGameLicense {
    fn default() -> Self {
        Self {
            sku_store_id: [0; STORE_SKU_ID_SIZE],
            is_active: true,
            is_trial_owned_by_this_user: false,
            is_disc_license: false,
            is_trial: false,
            trial_time_remaining_in_seconds: 0,
            trial_unique_id: [0; TRIAL_UNIQUE_ID_MAX_SIZE],
            expiration_date: 0,
        }
    }
}

#[repr(C)]
pub struct XStoreAddonLicense;

#[repr(C)]
pub struct XStoreCanAcquireLicenseResult;

// Class _GUID_0dd112ac_7c24_448c_b92b_3960fb5bd30c
// IID _GUID_0dd112ac_7c24_448c_b92b_3960fb5bd30c
#[interface("0dd112ac-7c24-448c-b92b-3960fb5bd30c")]
pub unsafe trait IXStore: IUnknown {
    // XStoreCreateContext
    pub unsafe fn x_store_create_context(
        self: &Self,
        _user: XUserHandle,
        _store_context_handle: *mut XStoreContextHandle,
    ) -> HRESULT;
    // XStoreCloseContextHandle
    pub unsafe fn x_store_close_context_handle(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
    ) -> ();
    // XStoreQueryAssociatedProductsAsync
    pub unsafe fn x_store_query_associated_products_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _product_kinds: XStoreProductKind,
        _max_items_to_retrieve_per_page: u32,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreQueryAssociatedProductsResult
    pub unsafe fn x_store_query_associated_products_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _product_query_handle: *mut XStoreProductQueryHandle,
    ) -> HRESULT;
    // XStoreQueryProductsAsync
    pub unsafe fn x_store_query_products_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _product_kinds: XStoreProductKind,
        _store_ids: *const *mut c_char,
        _store_ids_count: usize,
        _action_filters: *const *mut c_char,
        _action_filters_count: usize,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreQueryProductsResult
    pub unsafe fn x_store_query_products_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _product_query_handle: *mut XStoreProductQueryHandle,
    ) -> HRESULT;
    // XStoreQueryEntitledProductsAsync
    pub unsafe fn x_store_query_entitled_products_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _product_kinds: XStoreProductKind,
        _max_items_to_retrieve_per_page: u32,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreQueryEntitledProductsResult
    pub unsafe fn x_store_query_entitled_products_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _product_query_handle: *mut XStoreProductQueryHandle,
    ) -> HRESULT;
    // XStoreQueryProductForCurrentGameAsync
    pub unsafe fn x_store_query_product_for_current_game_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreQueryProductForCurrentGameResult
    pub unsafe fn x_store_query_product_for_current_game_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _product_query_handle: *mut XStoreProductQueryHandle,
    ) -> HRESULT;
    // XStoreQueryProductForPackageAsync
    pub unsafe fn x_store_query_product_for_package_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _product_kinds: XStoreProductKind,
        _package_identifier: *const c_char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreQueryProductForPackageResult
    pub unsafe fn x_store_query_product_for_package_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _product_query_handle: *mut XStoreProductQueryHandle,
    ) -> HRESULT;
    // XStoreEnumerateProductsQuery
    pub unsafe fn x_store_enumerate_products_query(
        self: &Self,
        _product_query_handle: XStoreProductQueryHandle,
        _context: *mut c_void,
        _callback: Option<XStoreProductQueryCallback>,
    ) -> HRESULT;
    // XStoreProductsQueryHasMorePages
    pub unsafe fn x_store_products_query_has_more_pages(
        self: &Self,
        _product_query_handle: XStoreProductQueryHandle,
    ) -> BOOL;
    // XStoreProductsQueryNextPageAsync
    pub unsafe fn x_store_products_query_next_page_async(
        self: &Self,
        _product_query_handle: XStoreProductQueryHandle,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreProductsQueryNextPageResult
    pub unsafe fn x_store_products_query_next_page_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _product_query_handle: *mut XStoreProductQueryHandle,
    ) -> HRESULT;
    // XStoreCloseProductsQueryHandle
    pub unsafe fn x_store_close_products_query_handle(
        self: &Self,
        _product_query_handle: XStoreProductQueryHandle,
    ) -> ();
    // XStoreAcquireLicenseForPackageAsync
    pub unsafe fn x_store_acquire_license_for_package_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _package_identifier: *const c_char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreAcquireLicenseForPackageResult
    pub unsafe fn x_store_acquire_license_for_package_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _store_license_handle: *mut XStoreLicenseHandle,
    ) -> HRESULT;
    // XStoreIsLicenseValid
    pub unsafe fn x_store_is_license_valid(
        self: &Self,
        _store_license_handle: XStoreLicenseHandle,
    ) -> BOOL;
    // XStoreCloseLicenseHandle
    pub unsafe fn x_store_close_license_handle(
        self: &Self,
        _store_license_handle: XStoreLicenseHandle,
    ) -> ();
    // XStoreCanAcquireLicenseForStoreIdAsync
    pub unsafe fn x_store_can_acquire_license_for_store_id_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _store_product_id: *const c_char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreCanAcquireLicenseForStoreIdResult
    pub unsafe fn x_store_can_acquire_license_for_store_id_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _store_can_acquire_license: *mut XStoreCanAcquireLicenseResult,
    ) -> HRESULT;
    // XStoreCanAcquireLicenseForPackageAsync
    pub unsafe fn x_store_can_acquire_license_for_package_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _package_identifier: *const c_char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreCanAcquireLicenseForPackageResult
    pub unsafe fn x_store_can_acquire_license_for_package_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _store_can_acquire_license: *mut XStoreCanAcquireLicenseResult,
    ) -> HRESULT;
    // XStoreQueryGameLicenseAsync
    pub unsafe fn x_store_query_game_license_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreQueryGameLicenseResult
    pub unsafe fn x_store_query_game_license_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _license: *mut XStoreGameLicense,
    ) -> HRESULT;
    // XStoreQueryAddOnLicensesAsync
    pub unsafe fn x_store_query_add_on_licenses_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreQueryAddOnLicensesResultCount
    pub unsafe fn x_store_query_add_on_licenses_result_count(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _count: *mut u32,
    ) -> HRESULT;
    // XStoreQueryAddOnLicensesResult
    pub unsafe fn x_store_query_add_on_licenses_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _count: u32,
        _add_on_licenses: *mut XStoreAddonLicense,
    ) -> HRESULT;
    // XStoreQueryConsumableBalanceRemainingAsync
    pub unsafe fn x_store_query_consumable_balance_remaining_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _store_product_id: *const c_char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreQueryConsumableBalanceRemainingResult
    pub unsafe fn x_store_query_consumable_balance_remaining_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _consumable_result: *mut XStoreConsumableResult,
    ) -> HRESULT;
    pub unsafe fn __reserved_slot_35(&self);
    // XStoreReportConsumableFulfillmentResult
    pub unsafe fn x_store_report_consumable_fulfillment_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _consumable_result: *mut XStoreConsumableResult,
    ) -> HRESULT;
    // XStoreGetUserCollectionsIdAsync
    pub unsafe fn x_store_get_user_collections_id_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _service_ticket: *const c_char,
        _publisher_user_id: *const c_char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreGetUserCollectionsIdResultSize
    pub unsafe fn x_store_get_user_collections_id_result_size(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _size: *mut usize,
    ) -> HRESULT;
    // XStoreGetUserCollectionsIdResult
    pub unsafe fn x_store_get_user_collections_id_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _size: usize,
        _result: *mut c_char,
    ) -> HRESULT;
    // XStoreGetUserPurchaseIdAsync
    pub unsafe fn x_store_get_user_purchase_id_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _service_ticket: *const c_char,
        _publisher_user_id: *const c_char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreGetUserPurchaseIdResultSize
    pub unsafe fn x_store_get_user_purchase_id_result_size(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _size: *mut usize,
    ) -> HRESULT;
    // XStoreGetUserPurchaseIdResult
    pub unsafe fn x_store_get_user_purchase_id_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _size: usize,
        _result: *mut c_char,
    ) -> HRESULT;
    // XStoreQueryLicenseTokenAsync
    pub unsafe fn x_store_query_license_token_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _product_ids: *const *mut c_char,
        _product_ids_count: usize,
        _custom_developer_string: *const c_char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreQueryLicenseTokenResultSize
    pub unsafe fn x_store_query_license_token_result_size(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _size: *mut usize,
    ) -> HRESULT;
    // XStoreQueryLicenseTokenResult
    pub unsafe fn x_store_query_license_token_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _size: usize,
        _result: *mut c_char,
    ) -> HRESULT;
    pub unsafe fn __reserved_slot_46(&self);
    pub unsafe fn __reserved_slot_47(&self);
    pub unsafe fn __reserved_slot_48(&self);
    // XStoreShowPurchaseUIAsync
    pub unsafe fn x_store_show_purchase_u_i_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _store_id: *const c_char,
        _name: *const c_char,
        _extended_json_data: *const c_char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreShowPurchaseUIResult
    pub unsafe fn x_store_show_purchase_u_i_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreShowRateAndReviewUIAsync
    pub unsafe fn x_store_show_rate_and_review_u_i_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreShowRateAndReviewUIResult
    pub unsafe fn x_store_show_rate_and_review_u_i_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _result: *mut XStoreRateAndReviewResult,
    ) -> HRESULT;
    // XStoreShowRedeemTokenUIAsync
    pub unsafe fn x_store_show_redeem_token_u_i_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _token: *const c_char,
        _allowed_store_ids: *const *mut c_char,
        _allowed_store_ids_count: usize,
        _disallow_csv_redemption: BOOL,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreShowRedeemTokenUIResult
    pub unsafe fn x_store_show_redeem_token_u_i_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreQueryGameAndDlcPackageUpdatesAsync
    pub unsafe fn x_store_query_game_and_dlc_package_updates_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreQueryGameAndDlcPackageUpdatesResultCount
    pub unsafe fn x_store_query_game_and_dlc_package_updates_result_count(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _count: *mut u32,
    ) -> HRESULT;
    // XStoreQueryGameAndDlcPackageUpdatesResult
    pub unsafe fn x_store_query_game_and_dlc_package_updates_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _count: u32,
        _package_updates: *mut XStorePackageUpdate,
    ) -> HRESULT;
    // XStoreDownloadPackageUpdatesAsync
    pub unsafe fn x_store_download_package_updates_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _package_identifiers: *const *mut c_char,
        _package_identifiers_count: usize,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreDownloadPackageUpdatesResult
    pub unsafe fn x_store_download_package_updates_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreDownloadAndInstallPackageUpdatesAsync
    pub unsafe fn x_store_download_and_install_package_updates_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _package_identifiers: *const *mut c_char,
        _package_identifiers_count: usize,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreDownloadAndInstallPackageUpdatesResult
    pub unsafe fn x_store_download_and_install_package_updates_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreDownloadAndInstallPackagesAsync
    pub unsafe fn x_store_download_and_install_packages_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _store_ids: *const *mut c_char,
        _store_ids_count: usize,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreDownloadAndInstallPackagesResultCount
    pub unsafe fn x_store_download_and_install_packages_result_count(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _count: *mut u32,
    ) -> HRESULT;
    // XStoreDownloadAndInstallPackagesResult
    pub unsafe fn x_store_download_and_install_packages_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _count: u32,
        _package_identifiers: *mut *mut c_char,
    ) -> HRESULT;
    // XStoreQueryPackageIdentifier
    pub unsafe fn x_store_query_package_identifier(
        self: &Self,
        _store_id: *const c_char,
        _size: usize,
        _package_identifier: *mut c_char,
    ) -> HRESULT;
    // XStoreRegisterGameLicenseChanged
    pub unsafe fn x_store_register_game_license_changed(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _queue: XTaskQueueHandle,
        _context: *mut c_void,
        _callback: Option<XStoreGameLicenseChangedCallback>,
        _token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT;
    // XStoreUnregisterGameLicenseChanged
    pub unsafe fn x_store_unregister_game_license_changed(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _token: XTaskQueueRegistrationToken,
        _wait: BOOL,
    ) -> BOOL;
    // XStoreRegisterPackageLicenseLost
    pub unsafe fn x_store_register_package_license_lost(
        self: &Self,
        _license_handle: XStoreLicenseHandle,
        _queue: XTaskQueueHandle,
        _context: *mut c_void,
        _callback: Option<XStorePackageLicenseLostCallback>,
        _token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT;
    // XStoreUnregisterPackageLicenseLost
    pub unsafe fn x_store_unregister_package_license_lost(
        self: &Self,
        _license_handle: XStoreLicenseHandle,
        _token: XTaskQueueRegistrationToken,
        _wait: BOOL,
    ) -> BOOL;
    pub unsafe fn __reserved_slot_70(&self);
    // XStoreAcquireLicenseForDurablesAsync
    pub unsafe fn x_store_acquire_license_for_durables_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _store_id: *const c_char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreAcquireLicenseForDurablesResult
    pub unsafe fn x_store_acquire_license_for_durables_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _store_license_handle: *mut XStoreLicenseHandle,
    ) -> HRESULT;
    // XStoreShowAssociatedProductsUIAsync
    pub unsafe fn x_store_show_associated_products_u_i_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _store_id: *const c_char,
        _product_kinds: XStoreProductKind,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreShowAssociatedProductsUIResult
    pub unsafe fn x_store_show_associated_products_u_i_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreShowProductPageUIAsync
    pub unsafe fn x_store_show_product_page_u_i_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _store_id: *const c_char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreShowProductPageUIResult
    pub unsafe fn x_store_show_product_page_u_i_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreQueryAssociatedProductsForStoreIdAsync
    pub unsafe fn x_store_query_associated_products_for_store_id_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _store_id: *const c_char,
        _product_kinds: XStoreProductKind,
        _max_items_to_retrieve_per_page: u32,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreQueryAssociatedProductsForStoreIdResult
    pub unsafe fn x_store_query_associated_products_for_store_id_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _product_query_handle: *mut XStoreProductQueryHandle,
    ) -> HRESULT;
    // XStoreQueryPackageUpdatesAsync
    pub unsafe fn x_store_query_package_updates_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _package_identifiers: *const *mut c_char,
        _package_identifiers_count: usize,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreQueryPackageUpdatesResultCount
    pub unsafe fn x_store_query_package_updates_result_count(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _count: *mut u32,
    ) -> HRESULT;
    // XStoreQueryPackageUpdatesResult
    pub unsafe fn x_store_query_package_updates_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _count: u32,
        _package_updates: *mut XStorePackageUpdate,
    ) -> HRESULT;
    // XStoreShowGiftingUIAsync
    pub unsafe fn x_store_show_gifting_u_i_async(
        self: &Self,
        _store_context_handle: XStoreContextHandle,
        _store_id: *const c_char,
        _name: *const c_char,
        _extended_json_data: *const c_char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XStoreShowGiftingUIResult
    pub unsafe fn x_store_show_gifting_u_i_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
}

#[interface("60B09F4E-1B85-45B1-826C-169118E230E1")]
pub unsafe trait IXStore2: IXStore2_1 {}

#[interface("de3dbdd4-0b37-4bdb-a10e-acf3a354d06a")]
pub unsafe trait IXStore2_1: IXStore {}
