use crate::results::E_POINTER;
use crate::threading::XAsyncBlock;
#[cfg(feature = "xuser")]
use crate::xbox_utils;
use crate::{
    E_FAIL,
    results::S_OK,
    threading::{XTaskQueueHandle, XTaskQueueRegistrationToken},
    xasync,
};
#[cfg(feature = "xuser")]
use crate::{authenticator::XalAuthenticator, xbox_utils::ProfileUser};
#[cfg(feature = "xuser")]
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::io::Read;
use std::path::Path;
#[cfg(feature = "xuser")]
use std::ptr::slice_from_raw_parts_mut;
use std::ptr::{self, null_mut};
#[cfg(feature = "xuser")]
use std::sync::Arc;
use std::{
    ffi::{CStr, c_char},
    os::raw::c_void,
};
#[cfg(feature = "xuser")]
use tokio_util::bytes::Bytes;
use windows::libloaderapi::GetModuleFileNameW;
use windows::minwindef::MAX_PATH;
use windows_core::{BOOL, HRESULT, IUnknown, Interface, implement, interface};
#[cfg(feature = "xuser")]
use xal_new::{self as xal, SignaturePolicyCache};
#[cfg(feature = "xuser")]
use xal_new::{DeviceType, XalAppParameters, XalClientParameters};
use xodus::models::licensing::LicenseUserIdentity;
#[cfg(feature = "xuser")]
use xodus::models::live::ExchangeUserTokenOutcome;
#[cfg(feature = "xuser")]
use xodus::models::secrets::Token;
#[cfg(feature = "xuser")]
use xodus::models::soap::BodyContent;
#[cfg(feature = "xuser")]
use xodus::{secrets, tokens::TokenManager};

#[repr(u32)]
pub enum XUserAddOptions {
    None = 0x00,
    AddDefaultUserSilently = 0x01,
    AllowGuests = 0x02,
    AddDefaultUserAllowingUI = 0x04,
}
#[repr(u32)]
pub enum XUserAgeGroup {
    Unknown = 0,
    Child = 1,
    Teen = 2,
    Adult = 3,
}
#[repr(u32)]
pub enum XUserChangeEvent {
    SignedInAgain = 0,
    SigningOut = 1,
    SignedOut = 2,
    Gamertag = 3,
    GamerPicture = 4,
    Privileges = 5,
}
#[repr(u32)]
pub enum XUserDefaultAudioEndpointKind {
    CommunicationRender = 0,
    CommunicationCapture = 1,
}
#[repr(u32)]
pub enum XUserGamerPictureSize {
    Small = 0,
    Medium = 1,
    Large = 2,
    ExtraLarge = 3,
}
#[repr(u32)]
pub enum XUserGamertagComponent {
    Classic = 0,
    Modern = 1,
    ModernSuffix = 2,
    UniqueModern = 3,
}
#[repr(u32)]
pub enum XUserGetMsaTokenSilentlyOptions {
    None = 0x00,
}
#[repr(u32)]
pub enum XUserGetTokenAndSignatureOptions {
    None = 0x00,
    ForceRefresh = 0x01,
    AllUsers = 0x02,
}
#[repr(u32)]
pub enum XUserPrivilege {
    CrossPlay = 185,
    Clubs = 188,
    Sessions = 189,
    Broadcast = 190,
    ManageProfilePrivacy = 196,
    GameDvr = 198,
    MultiplayerParties = 203,
    CloudManageSession = 207,
    CloudJoinSession = 208,
    CloudSavedGames = 209,
    SocialNetworkSharing = 220,
    UserGeneratedContent = 247,
    Communications = 252,
    Multiplayer = 254,
    AddFriends = 255,
}
#[repr(u32)]
pub enum XUserPrivilegeDenyReason {
    None = 0,
    PurchaseRequired = 1,
    Restricted = 2,
    Banned = 3,
    Unknown = 0xFFFFFFFF,
}
#[repr(u32)]
pub enum XUserPrivilegeOptions {
    None = 0x00,
    AllUsers = 0x01,
}
#[repr(u32)]
pub enum XUserState {
    SignedIn = 0,
    SigningOut = 1,
    SignedOut = 2,
}
#[repr(u32)]
pub enum XUserPlatformOperationResult {
    Success = 0,
    Failure = 1,
    Canceled = 2,
}

#[repr(u32)]
pub enum XUserPlatformSpopOperationResult {
    SignInHere = 0,
    SwitchAccount = 1,
    Failure = 2,
    Canceled = 3,
}

pub struct AppLocalDeviceId {
    pub value: [u8; 16],
}

#[repr(C)]
pub struct XUserDeviceAssociationChange {
    pub device_id: AppLocalDeviceId,
    pub old_user: XUserLocalId,
    pub new_user: XUserLocalId,
}
#[repr(C)]
pub struct XUserGetTokenAndSignatureData {
    pub token_size: usize,
    pub signature_size: usize,
    pub token: *const c_char,
    pub signature: *const c_char,
}
#[repr(C)]
pub struct XUserGetTokenAndSignatureHttpHeader {
    pub name: *const c_char,
    pub value: *const c_char,
}
#[repr(C)]
pub struct XUserGetTokenAndSignatureUtf16Data {
    pub token_count: usize,
    pub signature_count: usize,
    pub token: *const u16,
    pub signature: *const u16,
}
#[repr(C)]
pub struct XUserGetTokenAndSignatureUtf16HttpHeader {
    pub name: *const u16,
    pub value: *const u16,
}
#[repr(C)]
pub struct XUserLocalId {
    pub value: u64,
}

pub type XUserPlatformRemoteConnectShowPromptEventHandler = unsafe extern "system" fn(
    context: *const c_void,
    user_identifier: u32,
    operation: u32,
    url: *const c_char,
    code: *const c_char,
    qr_code_size: usize,
    qr_code: *const c_char,
);
pub type XUserPlatformRemoteConnectClosePromptEventHandler = unsafe extern "system" fn();

#[repr(C)]
pub struct XUserPlatformRemoteConnectEventHandler {
    pub show: Option<XUserPlatformRemoteConnectShowPromptEventHandler>,
    pub close: Option<XUserPlatformRemoteConnectClosePromptEventHandler>,
    pub context: *mut c_void,
}

pub type XUserHandle = *mut c_void;
pub type XUserSignOutDeferralHandle = *mut c_void;
pub type XUserChangeEventCallback = *mut c_void;
pub type XUserDeviceAssociationChangedCallback = *mut c_void;
pub type XUserPlatformSpopPromptEventHandlers = *mut c_void;

pub type XUserPlatformOperation = u64;

#[interface("01acd177-91f9-4763-a38e-ccbb55ce32e0")]
pub unsafe trait IXUser: IUnknown {
    pub unsafe fn x_user_duplicate_handle(
        self: &Self,
        handle: XUserHandle,
        duplicated_handle: *mut XUserHandle,
    ) -> HRESULT;
    pub unsafe fn x_user_close_handle(self: &Self, handle: XUserHandle);
    pub unsafe fn x_user_compare(self: &Self, user1: XUserHandle, user2: XUserHandle) -> u32;
    pub unsafe fn x_user_get_max_users(self: &Self, max_users: *mut u32) -> HRESULT;
    pub unsafe fn x_user_add_async(
        self: &Self,
        options: XUserAddOptions,
        async_: *mut XAsyncBlock,
    ) -> HRESULT;
    pub unsafe fn x_user_add_result(
        self: &Self,
        async_: *mut XAsyncBlock,
        new_user: *mut XUserHandle,
    ) -> HRESULT;
    pub unsafe fn x_user_get_local_id(
        self: &Self,
        user: XUserHandle,
        user_local_id: *mut XUserLocalId,
    ) -> HRESULT;
    pub unsafe fn x_user_find_user_by_local_id(
        self: &Self,
        user_local_id: XUserLocalId,
        handle: *mut XUserHandle,
    ) -> HRESULT;
    pub unsafe fn x_user_get_id(self: &Self, user: XUserHandle, user_id: *mut u64) -> HRESULT;
    pub unsafe fn x_user_find_user_by_id(
        self: &Self,
        user_id: u64,
        handle: *mut XUserHandle,
    ) -> HRESULT;
    pub unsafe fn x_user_get_is_guest(self: &Self, user: XUserHandle, is_guest: *mut u8)
    -> HRESULT;
    pub unsafe fn x_user_get_state(
        self: &Self,
        user: XUserHandle,
        state: *mut XUserState,
    ) -> HRESULT;
    pub unsafe fn ___1(self: &Self);
    pub unsafe fn x_user_get_gamer_picture_async(
        self: &Self,
        user: XUserHandle,
        picture_size: XUserGamerPictureSize,
        async_: *mut XAsyncBlock,
    ) -> HRESULT;
    pub unsafe fn x_user_get_gamer_picture_result_size(
        self: &Self,
        async_: *mut XAsyncBlock,
        buffer_size: *mut usize,
    ) -> HRESULT;
    pub unsafe fn x_user_get_gamer_picture_result(
        self: &Self,
        async_: *mut XAsyncBlock,
        buffer_size: usize,
        buffer: *mut c_void,
        buffer_used: *mut usize,
    ) -> HRESULT;
    pub unsafe fn x_user_get_age_group(
        self: &Self,
        user: XUserHandle,
        age_group: *mut XUserAgeGroup,
    ) -> HRESULT;
    pub unsafe fn x_user_check_privilege(
        self: &Self,
        user: XUserHandle,
        options: XUserPrivilegeOptions,
        privilege: XUserPrivilege,
        has_privilege: *mut u8,
        reason: *mut XUserPrivilegeDenyReason,
    ) -> HRESULT;
    pub unsafe fn x_user_resolve_privilege_with_ui_async(
        self: &Self,
        user: XUserHandle,
        options: XUserPrivilegeOptions,
        privilege: XUserPrivilege,
        async_: *mut XAsyncBlock,
    ) -> HRESULT;
    pub unsafe fn x_user_resolve_privilege_with_ui_result(
        self: &Self,
        async_: *mut XAsyncBlock,
    ) -> HRESULT;
    pub unsafe fn x_user_get_token_and_signature_async(
        self: &Self,
        user: XUserHandle,
        options: XUserGetTokenAndSignatureOptions,
        method: *const c_char,
        url: *const c_char,
        header_count: usize,
        headers: *const XUserGetTokenAndSignatureHttpHeader,
        body_size: usize,
        body_buffer: *const c_void,
        async_: *mut XAsyncBlock,
    ) -> HRESULT;
    pub unsafe fn x_user_get_token_and_signature_result_size(
        self: &Self,
        async_: *mut XAsyncBlock,
        buffer_size: *mut usize,
    ) -> HRESULT;
    pub unsafe fn x_user_get_token_and_signature_result(
        self: &Self,
        async_: *mut XAsyncBlock,
        buffer_size: usize,
        buffer: *mut c_void,
        ptr_to_buffer: *mut *mut XUserGetTokenAndSignatureData,
        buffer_used: *mut usize,
    ) -> HRESULT;
    pub unsafe fn x_user_get_token_and_signature_utf16_async(
        self: &Self,
        user: XUserHandle,
        options: XUserGetTokenAndSignatureOptions,
        method: *const u16,
        url: *const u16,
        header_count: usize,
        headers: *const XUserGetTokenAndSignatureUtf16HttpHeader,
        body_size: usize,
        body_buffer: *const c_void,
        async_: *mut XAsyncBlock,
    ) -> HRESULT;
    pub unsafe fn x_user_get_token_and_signature_utf16_result_size(
        self: &Self,
        async_: *mut XAsyncBlock,
        buffer_size: *mut usize,
    ) -> HRESULT;
    pub unsafe fn x_user_get_token_and_signature_utf16_result(
        self: &Self,
        async_: *mut XAsyncBlock,
        buffer_size: usize,
        buffer: *mut c_void,
        ptr_to_buffer: *mut *mut XUserGetTokenAndSignatureUtf16Data,
        buffer_used: *mut usize,
    ) -> HRESULT;
    pub unsafe fn x_user_resolve_issue_with_ui_async(
        self: &Self,
        user: XUserHandle,
        url: *const c_char,
        async_: *mut XAsyncBlock,
    ) -> HRESULT;
    pub unsafe fn x_user_resolve_issue_with_ui_result(
        self: &Self,
        async_: *mut XAsyncBlock,
    ) -> HRESULT;
    pub unsafe fn x_user_resolve_issue_with_ui_utf16_async(
        self: &Self,
        user: XUserHandle,
        url: *const u16,
        async_: *mut XAsyncBlock,
    ) -> HRESULT;
    pub unsafe fn x_user_resolve_issue_with_ui_utf16_result(
        self: &Self,
        async_: *mut XAsyncBlock,
    ) -> HRESULT;
    pub unsafe fn x_user_register_for_change_event(
        self: &Self,
        queue: XTaskQueueHandle,
        context: *mut c_void,
        callback: *mut XUserChangeEventCallback,
        token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT;
    pub unsafe fn x_user_unregister_for_change_event(
        self: &Self,
        token: XTaskQueueRegistrationToken,
        wait: bool,
    ) -> HRESULT;
    pub unsafe fn x_user_get_sign_out_deferral(
        self: &Self,
        deferral: *mut XUserSignOutDeferralHandle,
    ) -> HRESULT;
    pub unsafe fn x_user_close_sign_out_deferral_handle(
        self: &Self,
        deferral: XUserSignOutDeferralHandle,
    ) -> HRESULT;
    pub unsafe fn x_user_register_for_device_association_changed(
        self: &Self,
        queue: XTaskQueueHandle,
        context: *mut c_void,
        callback: *mut XUserDeviceAssociationChangedCallback,
        token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT;
    pub unsafe fn x_user_unregister_for_device_association_changed(
        self: &Self,
        token: XTaskQueueRegistrationToken,
        wait: bool,
    ) -> HRESULT;
    pub unsafe fn ___2(self: &Self);
    pub unsafe fn ___3(self: &Self);
    pub unsafe fn ___4(self: &Self);
    pub unsafe fn x_user_is_store_user(self: &Self, user: XUserHandle) -> HRESULT;
    pub unsafe fn x_user_platform_remote_connect_set_event_handlers(
        self: &Self,
        queue: XTaskQueueHandle,
        handlers: *mut XUserPlatformRemoteConnectEventHandler,
    ) -> HRESULT;
    pub unsafe fn x_user_platform_remote_connect_cancel_prompt(
        self: &Self,
        operation: XUserPlatformOperation,
    ) -> HRESULT;
    pub unsafe fn x_user_platform_spop_prompt_set_event_handlers(
        self: &Self,
        queue: XTaskQueueHandle,
        handler: *mut XUserPlatformSpopPromptEventHandlers,
        context: *mut c_void,
    ) -> HRESULT;
    pub unsafe fn x_user_platform_spop_prompt_complete(
        self: &Self,
        operation: XUserPlatformOperation,
        result: XUserPlatformSpopOperationResult,
    ) -> HRESULT;
}

#[interface("cef4fac0-7676-4a94-a119-4c43f9eb5b74")]
pub unsafe trait IXUser2: IUnknown {
    unsafe fn x_user_get_gamertag(
        self: &Self,
        _user: XUserHandle,
        _gamertag_component: XUserGamertagComponent,
        _gamertag_size: usize,
        _gamertag: *mut c_char,
        _gamertag_used: *mut usize,
    ) -> HRESULT;
}

#[interface("26f3c674-a2fe-44fa-b6c4-a323bc94ff53")]
pub unsafe trait IXUser3: IXUser {}

#[interface("079415e3-6727-437f-8e9d-8f8f9b2439f7")]
pub unsafe trait IXUser4: IXUser {}

#[interface("eb9bf948-18dc-4d82-bbcc-40e0a809c4c0")]
pub unsafe trait IXUser5: IXUser {}

#[interface("1bf2f8c5-d507-4e52-bb05-f726d0e71161")]
pub unsafe trait IXUser6: IXUser {}

// XUserDefaultAudioEndpointUtf16ChangedCallback
pub type XUserDefaultAudioEndpointUtf16ChangedCallback = unsafe extern "system" fn(
    _context: *mut c_void,
    _user: XUserLocalId,
    _default_audio_endpoint_kind: XUserDefaultAudioEndpointKind,
    _endpoint_id_utf16: *const u16,
) -> ();

// Class _GUID_7d824997_10dc_45ab_86b7_2737767c0bf1
// IID _GUID_7d824997_10dc_45ab_86b7_2737767c0bf1
#[interface("0cc6a956-e7e1-4fdf-9341-9d5da649ebc8")]
pub unsafe trait IXUserDevice: IUnknown {
    // XUserFindForDevice
    unsafe fn x_user_find_for_device(
        self: &Self,
        _device_id: *const c_void,
        _handle: *mut XUserHandle,
    ) -> HRESULT;
    // XUserRegisterForDeviceAssociationChanged
    unsafe fn x_user_register_for_device_association_changed(
        self: &Self,
        _queue: XTaskQueueHandle,
        _context: *mut c_void,
        _callback: Option<XUserDeviceAssociationChangedCallback>,
        _token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT;
    // XUserUnregisterForDeviceAssociationChanged
    unsafe fn x_user_unregister_for_device_association_changed(
        self: &Self,
        _token: XTaskQueueRegistrationToken,
        _wait: BOOL,
    ) -> BOOL;
    // XUserGetDefaultAudioEndpointUtf16
    unsafe fn x_user_get_default_audio_endpoint_utf16(
        self: &Self,
        _user: XUserLocalId,
        _default_audio_endpoint_kind: XUserDefaultAudioEndpointKind,
        _endpoint_id_utf16_count: usize,
        _endpoint_id_utf16: *mut u16,
        _endpoint_id_utf16_used: *mut usize,
    ) -> HRESULT;
    // XUserRegisterForDefaultAudioEndpointUtf16Changed
    unsafe fn x_user_register_for_default_audio_endpoint_utf16_changed(
        self: &Self,
        _queue: XTaskQueueHandle,
        _context: *mut c_void,
        _callback: Option<XUserDefaultAudioEndpointUtf16ChangedCallback>,
        _token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT;
    // XUserUnregisterForDefaultAudioEndpointUtf16Changed
    unsafe fn x_user_unregister_for_default_audio_endpoint_utf16_changed(
        self: &Self,
        _token: XTaskQueueRegistrationToken,
        _wait: BOOL,
    ) -> BOOL;
    // XUserFindControllerForUserWithUiAsync
    unsafe fn x_user_find_controller_for_user_with_ui_async(
        self: &Self,
        _user: XUserHandle,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT;
    // XUserFindControllerForUserWithUiResult
    unsafe fn x_user_find_controller_for_user_with_ui_result(
        self: &Self,
        _async_: *mut XAsyncBlock,
        _device_id: *mut c_void,
    ) -> HRESULT;
}

#[implement(IXUser, IXUser2, IXUser3, IXUser4, IXUser5, IXUser6, IXUserDevice)]
pub struct XUser {
    pub runtime: tokio::runtime::Runtime,
    pub handle: Cell<Option<IXUserHandle>>,
}

#[cfg(feature = "xuser")]
#[interface("01acd177-91f9-4763-a38e-ccbb55ce32e0")]
pub unsafe trait IXUserHandle: IUnknown {
    unsafe fn get_xuid(&self) -> u64;
    unsafe fn get_local_id(&self) -> XUserLocalId;
    unsafe fn get_auth(&self) -> *const Arc<tokio::sync::Mutex<XuserHandleObjectAuth>>;
    unsafe fn get_gamertag_component(&self, component: u32) -> *const c_char;
    unsafe fn get_runtime(&self) -> *const tokio::runtime::Handle;
}

#[cfg(not(feature = "xuser"))]
#[interface("01acd177-91f9-4763-a38e-ccbb55ce32e0")]
pub unsafe trait IXUserHandle: IUnknown {
    unsafe fn get_xuid(&self) -> u64;
    unsafe fn get_local_id(&self) -> XUserLocalId;
    unsafe fn get_runtime(&self) -> *const tokio::runtime::Handle;
}

#[cfg(feature = "xuser")]
struct XuserHandleObjectAuth {
    authenticator: XalAuthenticator,
    auth: xal::response::SisuRPSAuthorizationResponse,
    policy: SignaturePolicyCache,
    def_policy: SignaturePolicyCache,
    device_token: xal::response::DeviceToken,
}

#[cfg(feature = "xuser")]
#[implement(IXUserHandle)]
struct XUserHandleObject {
    gamertags: [std::ffi::CString; 4],
    xuid: u64,
    local_id: XUserLocalId,
    auth: Arc<tokio::sync::Mutex<XuserHandleObjectAuth>>,
    runtime: tokio::runtime::Handle,
}

#[derive(Deserialize, Debug)]
pub struct Game {
    #[serde(rename = "StoreId")]
    pub store_id: String,
    #[serde(rename = "TitleId")]
    pub title_id: String,
    #[serde(rename = "MSAAppId")]
    pub msa_app_id: Option<String>,
}

impl Game {
    pub fn get_title_id(&self) -> i64 {
        i64::from_str_radix(&self.title_id, 16).unwrap()
    }
}

#[cfg(feature = "xuser")]
impl IXUserHandle_Impl for XUserHandleObject_Impl {
    unsafe fn get_xuid(&self) -> u64 {
        self.xuid
    }
    unsafe fn get_local_id(&self) -> XUserLocalId {
        XUserLocalId {
            value: self.local_id.value,
        }
    }

    // unsafe fn get_object(&self,) -> *mut XUserHandleObject {
    //     &mut self.this as *mut XUserHandleObject
    // }
    unsafe fn get_auth(&self) -> *const Arc<tokio::sync::Mutex<XuserHandleObjectAuth>> {
        &self.auth
    }

    unsafe fn get_gamertag_component(&self, component: u32) -> *const c_char {
        self.gamertags.get(component as usize).map_or(std::ptr::null(), |s| s.as_ptr())
    }

    unsafe fn get_runtime(&self) -> *const tokio::runtime::Handle {
        &self.runtime
    }
}

impl IXUser2_Impl for XUser_Impl {
    unsafe fn x_user_get_gamertag(
        &self,
        user: XUserHandle,
        gamertag_component: XUserGamertagComponent,
        gamertag_size: usize,
        gamertag: *mut c_char,
        gamertag_used: *mut usize,
    ) -> HRESULT {
        #[cfg(feature = "xuser")]
        {
            if gamertag.is_null() { return E_POINTER; }
            let Some(handle) = (unsafe { IXUserHandle::from_raw_borrowed(&user) }) else {
                return E_POINTER;
            };
            let component = gamertag_component as u32;
            let value = unsafe { handle.get_gamertag_component(component) };
            if value.is_null() { return HRESULT(0x80070057u32 as i32); }
            let bytes = unsafe { CStr::from_ptr(value) }.to_bytes_with_nul();
            if bytes.len() == 1 && component != 2 {
                return E_FAIL;
            }
            if gamertag_size < bytes.len() { return HRESULT(0x8007007au32 as i32); }
            unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), gamertag.cast::<u8>(), bytes.len()); }
            if !gamertag_used.is_null() { unsafe { *gamertag_used = bytes.len(); } }
            S_OK
        }
        #[cfg(not(feature = "xuser"))]
        { crate::E_NOTIMPL }
    }
}

impl IXUser3_Impl for XUser_Impl {}
impl IXUser4_Impl for XUser_Impl {}
impl IXUser5_Impl for XUser_Impl {}
impl IXUser6_Impl for XUser_Impl {}

impl IXUser_Impl for XUser_Impl {
    unsafe fn x_user_duplicate_handle(
        &self,
        handle: XUserHandle,
        duplicated_handle: *mut XUserHandle,
    ) -> HRESULT {
        unsafe {
            IXUserHandle::from_raw_borrowed(&handle)
                .map(|f| {
                    *duplicated_handle = f.clone().into_raw();
                    S_OK
                })
                .unwrap_or(E_FAIL)
        }
    }

    unsafe fn x_user_close_handle(&self, handle: XUserHandle) {
        unsafe { IXUserHandle::from_raw(handle) };
    }

    unsafe fn x_user_compare(&self, user1: XUserHandle, user2: XUserHandle) -> u32 {
        let a = unsafe { IXUserHandle::from_raw_borrowed(&user1) };
        let b = unsafe { IXUserHandle::from_raw_borrowed(&user2) };
        let (Some(a), Some(b)) = (a, b) else {
            return 1;
        };
        (unsafe { a.get_xuid().cmp(&b.get_xuid()) }) as u32
    }

    unsafe fn x_user_get_max_users(&self, max_users: *mut u32) -> HRESULT {
        unsafe { *max_users = 4 };
        S_OK
    }

    unsafe fn x_user_add_async(
        &self,
        _options: XUserAddOptions,
        async_: *mut XAsyncBlock,
    ) -> HRESULT {
        // xasync::run(async_, async {
        //     Err::<(),_>(E_FAIL)
        // })
        /* Authentication stdout logging suppressed. */
        crate::diag("x_user_add_async entered");
        #[cfg(feature = "xuser")]
        let handle = self.runtime.handle().clone();
        #[cfg(feature = "xuser")]
        let handle2 = self.runtime.handle().clone();
        unsafe {
            xasync::run(async_, {
                async move {
                    #[cfg(feature = "xuser")]
                    {
                        let user = handle
                            .spawn(async move {
                                use std::path::Path;

                                use windows::{libloaderapi::GetModuleFileNameW, minwindef::MAX_PATH};

                                let client = reqwest::Client::builder()
                                    .use_rustls_tls()
                                    .http1_only()
                                    .connection_verbose(false)
                                    .pool_max_idle_per_host(0)
                                    .connect_timeout(std::time::Duration::from_secs(5))
                                    .timeout(std::time::Duration::from_secs(10))
                                    .build()
                                    .unwrap(); // let manager = xodus::auth::Manager::new();

                                let r = client
                                    .get("https://title.mgt.xboxlive.com/titles/default/endpoints?type=1")
                                    .send()
                                    .await
                                    .unwrap()
                                    .json::<xal_new::response::TitleEndpointsResponse>()
                                    .await
                                    .unwrap();
                                /* Authentication stdout logging suppressed. */

                                let def_policy = SignaturePolicyCache::new(r);

                                std::env::set_var("HOME", std::env::var_os("USERPROFILE").unwrap());
                                /* Authentication stdout logging suppressed. */
                                secrets::init_secrets().expect("Unable to initialize credentials");
                                let tokens: TokenManager = TokenManager::with_keychain_and_memory();

                                let mut path = [0u16; MAX_PATH as usize];
                                let len = GetModuleFileNameW(None, &mut path);
                                let path = String::from_utf16_lossy(&path[..len as usize]);
                                let mut path = Path::new(&path);
                                let mut config: Option<Game> = None;
                                loop {
                                    let Some(parent) = path.parent()else {
                                        break;
                                    };

                                    if let Ok(mut fs) = tokio::fs::File::open(path.join("MicrosoftGame.config")).await {
                                        use tokio::io::AsyncReadExt;
                                        let mut bytes = String::new();
                                        fs.read_to_string(&mut bytes).await.unwrap();

                                        config = Some(quick_xml::de::from_str(&bytes).unwrap());
                                        break;
                                    }

                                    path = parent;
                                }
                                if config.is_none() {
                                    panic!("No gdk game?");
                                }
                                let config = config.unwrap();

                                // do_license_token(&client, &tokens).await.unwrap();

                                let (c, resp, device) =
                                    do_sisu(&client, &tokens, config.msa_app_id.as_ref().map_or_else(|| "", |x|x), i64::from_str_radix(&config.title_id, 16).unwrap(), def_policy.clone())
                                        .await
                                        .unwrap();

                                /* Authentication stdout logging suppressed. */
                                /* Authentication stdout logging suppressed. */
                                /* Authentication stdout logging suppressed. */

                                let r = client
                                    .get("https://title.mgt.xboxlive.com/titles/current/endpoints")
                                    .header("x-xbl-contract-version", "2")
                                    .header(
                                        "Authorization",
                                        resp.authorization_token.authorization_header_value(),
                                    )
                                    .send()
                                    .await
                                    .unwrap()
                                    .json::<xal_new::response::TitleEndpointsResponse>()
                                    .await
                                    .unwrap();
                                /* Authentication stdout logging suppressed. */

                                let policy = SignaturePolicyCache::new(r);

                                let xid = resp
                                    .authorization_token
                                    .display_claims
                                    .as_ref()
                                    .map(|d| d.xui[0]["xid"].clone())
                                    .unwrap();

                                // Retrieve only this authenticated user's profile once and keep
                                // display components in the handle. Never log profile identifiers.
                                let profiles = xbox_utils::fetch_user_profiles(
                                    &client,
                                    &resp.authorization_token.authorization_header_value(),
                                    &[&xid],
                                ).await;
                                // Display metadata failure must not turn a valid sign-in into failure.
                                let settings = profiles.ok().and_then(|mut p| p.remove(&xid))
                                    .map(|p| p.settings).unwrap_or_default();
                                let classic = settings.get("Gamertag").cloned().unwrap_or_default();
                                let modern = settings.get("ModernGamertag").filter(|s| !s.is_empty()).cloned().unwrap_or_else(|| classic.clone());
                                let suffix = settings.get("ModernGamertagSuffix").cloned().unwrap_or_default();
                                let unique = settings.get("UniqueModernGamertag").filter(|s| !s.is_empty()).cloned().unwrap_or_else(|| classic.clone());
                                let gamertags = [classic, modern, suffix, unique].map(|value|
                                    std::ffi::CString::new(value).unwrap_or_default());
                                crate::diag("authenticated profile cached");
                                let handle = XUserHandleObject {
                                    gamertags,
                                    xuid: xid.parse::<u64>().unwrap(),
                                    local_id: XUserLocalId { value: 987654321 },
                                    auth: Arc::new(tokio::sync::Mutex::new(XuserHandleObjectAuth {
                                        authenticator: c,
                                        auth: resp,
                                        policy,
                                        device_token: device,
                                        def_policy: def_policy,
                                    })),
                                    runtime: handle2,
                                };
                                let h: IXUserHandle = handle.into();

                                crate::diag("XUser handle created");
                                Ok::<_, HRESULT>(h.into_raw() as u64)
                            })
                            .await
                            .unwrap()?;
                        return Ok::<_, HRESULT>(user as *mut c_void);
                    }
                    #[cfg(not(feature = "xuser"))]
                    {
                        use crate::results::E_ABORT;

                        /* Authentication stdout logging suppressed. */
                        return Err::<*mut c_void, _>(E_ABORT);
                    }
                }
            })
        }
    }

    unsafe fn x_user_add_result(
        &self,
        async_: *mut XAsyncBlock,
        new_user: *mut XUserHandle,
    ) -> HRESULT {
        /* Authentication stdout logging suppressed. */
        let err = unsafe { xasync::get_result(async_, null_mut(), new_user) }
            .map(|_| S_OK)
            .unwrap_or_else(|e| e);
        /* Authentication stdout logging suppressed. */
        crate::diag(&format!("x_user_add_result hr={:#X}", err.0));
        if err.is_ok() {
            self.handle.replace(unsafe {
                Some(
                    IXUserHandle::from_raw_borrowed(&*new_user)
                        .cloned()
                        .unwrap(),
                )
            });
        }
        err
    }

    unsafe fn x_user_get_local_id(
        &self,
        user: XUserHandle,
        user_local_id: *mut XUserLocalId,
    ) -> HRESULT {
        /* Authentication stdout logging suppressed. */
        unsafe {
            IXUserHandle::from_raw_borrowed(&user)
                .map(|f| {
                    *user_local_id = f.get_local_id();
                    S_OK
                })
                .unwrap_or(E_FAIL)
        }
    }

    unsafe fn x_user_find_user_by_local_id(
        &self,
        user_local_id: XUserLocalId,
        handle: *mut XUserHandle,
    ) -> HRESULT {
        E_FAIL
    }

    unsafe fn x_user_get_id(&self, user: XUserHandle, user_id: *mut u64) -> HRESULT {
        /* Authentication stdout logging suppressed. */
        let err = unsafe {
            IXUserHandle::from_raw_borrowed(&user)
                .map(|f| {
                    *user_id = f.get_xuid();
                    S_OK
                })
                .unwrap_or(E_FAIL)
        };
        /* Authentication stdout logging suppressed. */
        err
    }

    unsafe fn x_user_find_user_by_id(&self, user_id: u64, handle: *mut XUserHandle) -> HRESULT {
        /* Authentication stdout logging suppressed. */
        crate::diag("x_user_find_user_by_id entered");
        let h = self.handle.as_ptr();
        if h.is_null() || handle.is_null() {
            crate::diag("x_user_find_user_by_id FAILED");
            return E_FAIL;
        }
        *handle = (*h).clone().unwrap().into_raw();
        /* Authentication stdout logging suppressed. */
        crate::diag("x_user_find_user_by_id OK");
        S_OK
    }

    unsafe fn x_user_get_is_guest(&self, _user: XUserHandle, is_guest: *mut u8) -> HRESULT {
        /* Authentication stdout logging suppressed. */
        unsafe {
            *is_guest = 0u8;
        };
        S_OK
    }

    unsafe fn x_user_get_state(&self, _user: XUserHandle, state: *mut XUserState) -> HRESULT {
        /* Authentication stdout logging suppressed. */
        unsafe {
            *state = XUserState::SignedIn;
        };
        crate::diag("x_user_get_state -> SignedIn");
        S_OK
    }

    unsafe fn ___1(&self) {
        todo!()
    }

    unsafe fn x_user_get_gamer_picture_async(
        &self,
        user: XUserHandle,
        picture_size: XUserGamerPictureSize,
        async_: *mut XAsyncBlock,
    ) -> HRESULT {
        /* Authentication stdout logging suppressed. */
        #[cfg(feature = "xuser")]
        {
            let user = unsafe { IXUserHandle::from_raw_borrowed(&user) };
            let xuid = user.map(|f| f.get_xuid());
            let handle = user.map(|f| unsafe { (*f.get_runtime()).clone() }).unwrap();
            let user = unsafe { (*user.unwrap().get_auth()).clone() };
            unsafe {
                xasync::run_dyn(async_, {
                    async move {
                        /* Authentication stdout logging suppressed. */
                        let profile =
                            get_gamerpicture(handle, user, xuid.unwrap(), picture_size).await;
                        /* Authentication stdout logging suppressed. */
                        let req_size = profile.len();
                        Ok::<_, HRESULT>((
                            move |b: *mut c_void, s: usize| {
                                std::ptr::copy_nonoverlapping(
                                    profile.as_ptr(),
                                    b as *mut u8,
                                    profile.len(),
                                );
                                return s;
                            },
                            req_size,
                        ))
                    }
                })
            }
        }
        #[cfg(not(feature = "xuser"))]
        {
            let _ = async_;
            let _ = user;
            crate::E_NOTIMPL
        }
    }

    unsafe fn x_user_get_gamer_picture_result_size(
        &self,
        async_: *mut XAsyncBlock,
        buffer_size: *mut usize,
    ) -> HRESULT {
        /* Authentication stdout logging suppressed. */
        let size = unsafe { xasync::get_result_size(async_).unwrap() };
        unsafe { *buffer_size = size };
        S_OK
    }

    unsafe fn x_user_get_gamer_picture_result(
        &self,
        async_: *mut XAsyncBlock,
        buffer_size: usize,
        buffer: *mut c_void,
        buffer_used: *mut usize,
    ) -> HRESULT {
        /* Authentication stdout logging suppressed. */
        match unsafe {
            xasync::get_result_dyn(async_, null_mut(), buffer_size, buffer, buffer_used)
        } {
            Err(hr) => hr,
            _ => S_OK,
        }
    }

    unsafe fn x_user_get_age_group(
        &self,
        _user: XUserHandle,
        age_group: *mut XUserAgeGroup,
    ) -> HRESULT {
        /* Authentication stdout logging suppressed. */
        unsafe {
            *age_group = XUserAgeGroup::Adult;
        };
        S_OK
    }

    unsafe fn x_user_check_privilege(
        &self,
        _user: XUserHandle,
        _options: XUserPrivilegeOptions,
        privilege: XUserPrivilege,
        has_privilege: *mut u8,
        reason: *mut XUserPrivilegeDenyReason,
    ) -> HRESULT {
        /* Authentication stdout logging suppressed. */
        unsafe {
            *has_privilege = 1;
        };
        if !reason.is_null() {
            unsafe {
                *reason = XUserPrivilegeDenyReason::None;
            };
        }
        S_OK
    }

    unsafe fn x_user_resolve_privilege_with_ui_async(
        &self,
        _user: XUserHandle,
        _options: XUserPrivilegeOptions,
        _privilege: XUserPrivilege,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        todo!()
    }

    unsafe fn x_user_resolve_privilege_with_ui_result(&self, _async_: *mut XAsyncBlock) -> HRESULT {
        todo!()
    }

    // struct FakeToken

    unsafe fn x_user_get_token_and_signature_async(
        &self,
        user: XUserHandle,
        _options: XUserGetTokenAndSignatureOptions,
        _method: *const c_char,
        url: *const c_char,
        _header_count: usize,
        _headers: *const XUserGetTokenAndSignatureHttpHeader,
        _body_size: usize,
        _body_buffer: *const c_void,
        async_: *mut XAsyncBlock,
    ) -> HRESULT {
        let url = unsafe { CStr::from_ptr(url) }.to_string_lossy().to_string();

        let safe_url = url::Url::parse(&url)
            .map(|u| format!(
                "{}://{}{}",
                u.scheme(),
                u.host_str().unwrap_or("?"),
                u.path()
            ))
            .unwrap_or_else(|_| "<unparseable URL>".to_string());

        crate::diag(&format!("token request ANSI: {}", safe_url));

        /* Authentication stdout logging suppressed. */

        #[cfg(feature = "xuser")]
        {
            let user = unsafe { IXUserHandle::from_raw_borrowed(&user) };
            let handle = user.map(|f| unsafe { (*f.get_runtime()).clone() }).unwrap();
            let user = unsafe { (*user.unwrap().get_auth()).clone() };
            unsafe {
                xasync::run_dyn(async_, {
                    async move {
                        let token = get_xsts_token(handle, user, url).await;
                        crate::diag(&format!("XSTS token generated, bytes={}", token.len()));

                        let req_size = size_of::<XUserGetTokenAndSignatureData>() + token.len() + 1;
                        Ok::<_, HRESULT>((
                            move |b: *mut c_void, s: usize| {
                                let data = &mut *b.cast::<XUserGetTokenAndSignatureData>();
                                data.signature = null_mut();
                                data.signature_size = 0;
                                data.token =
                                    b.add(size_of::<XUserGetTokenAndSignatureData>()).cast();
                                data.token_size = token.len() + 1;
                                std::ptr::copy_nonoverlapping(
                                    token.as_ptr(),
                                    data.token as *mut u8,
                                    token.len(),
                                );
                                // GDK requires a terminator even in a dirty caller buffer.
                                *(data.token as *mut u8).add(token.len()) = 0;
                                return s;
                            },
                            req_size,
                        ))
                    }
                })
            }
        }
        #[cfg(not(feature = "xuser"))]
        {
            let _ = async_;
            let _ = user;
            crate::E_NOTIMPL
        }
    }

    unsafe fn x_user_get_token_and_signature_result_size(
        &self,
        async_: *mut XAsyncBlock,
        buffer_size: *mut usize,
    ) -> HRESULT {
        /* Authentication stdout logging suppressed. */
        if buffer_size.is_null() {
            return E_POINTER;
        }
        match unsafe { xasync::get_result_size(async_) } {
            Err(hr) => hr,
            Ok(size) => unsafe {
                *buffer_size = size;
                S_OK
            },
        }
    }

    unsafe fn x_user_get_token_and_signature_result(
        &self,
        async_: *mut XAsyncBlock,
        buffer_size: usize,
        buffer: *mut c_void,
        ptr_to_buffer: *mut *mut XUserGetTokenAndSignatureData,
        buffer_used: *mut usize,
    ) -> HRESULT {
        /* Authentication stdout logging suppressed. */
        match unsafe {
            xasync::get_result_dyn(async_, null_mut(), buffer_size, buffer, buffer_used)
        } {
            Err(hr) => return hr,
            _ => (),
        }
        /* Authentication stdout logging suppressed. */
        if !ptr_to_buffer.is_null() {
            unsafe { *ptr_to_buffer = buffer as *mut XUserGetTokenAndSignatureData };
        }
        /* Authentication stdout logging suppressed. */
        crate::diag("token result ANSI OK");
        S_OK
    }

    unsafe fn x_user_get_token_and_signature_utf16_async(
        &self,
        user: XUserHandle,
        _options: XUserGetTokenAndSignatureOptions,
        _method: *const u16,
        url: *const u16,
        _header_count: usize,
        _headers: *const XUserGetTokenAndSignatureUtf16HttpHeader,
        _body_size: usize,
        _body_buffer: *const c_void,
        async_: *mut XAsyncBlock,
    ) -> HRESULT {
        let url = unsafe { windows_strings::PCWSTR::from_raw(url).to_string() }.unwrap();

        let safe_url = url::Url::parse(&url)
            .map(|u| format!(
                "{}://{}{}",
                u.scheme(),
                u.host_str().unwrap_or("?"),
                u.path()
            ))
            .unwrap_or_else(|_| "<unparseable URL>".to_string());

        crate::diag(&format!("token request UTF16: {}", safe_url));

        /* Authentication stdout logging suppressed. */
        #[cfg(feature = "xuser")]
        {
            let user = unsafe { IXUserHandle::from_raw_borrowed(&user) };
            let handle = user.map(|f| unsafe { (*f.get_runtime()).clone() }).unwrap();
            let user = unsafe { (*user.unwrap().get_auth()).clone() };
            /* Authentication stdout logging suppressed. */
            unsafe {
                xasync::run_dyn(async_, {
                    async move {
                        let token = get_xsts_token(handle, user, url).await;
                        crate::diag(&format!("XSTS UTF16 generated, bytes={}", token.len()));

                        let token_count = token.encode_utf16().count();
                        let token_start = size_of::<XUserGetTokenAndSignatureUtf16Data>();

                        let req_size = token_start + (token_count + 1) * 2;
                        Ok::<_, HRESULT>((
                            move |b: *mut c_void, s: usize| {
                                let data = &mut *b.cast::<XUserGetTokenAndSignatureUtf16Data>();
                                data.signature = null_mut();
                                data.signature_count = 0;
                                data.token = b.add(token_start).cast();
                                data.token_count = token_count + 1;
                                let token_raw = &mut *(slice_from_raw_parts_mut(
                                    data.token as *mut u16,
                                    token_count + 1,
                                ));
                                token_raw.iter_mut().zip(token.encode_utf16()).for_each(
                                    |(dst, src)| {
                                        *dst = src;
                                    },
                                );
                                token_raw[token_count] = 0;
                                return s;
                            },
                            req_size,
                        ))
                    }
                })
            }
        }
        #[cfg(not(feature = "xuser"))]
        {
            let _ = async_;
            let _ = user;
            crate::E_NOTIMPL
        }
    }

    unsafe fn x_user_get_token_and_signature_utf16_result_size(
        &self,
        async_: *mut XAsyncBlock,
        buffer_size: *mut usize,
    ) -> HRESULT {
        if buffer_size.is_null() {
            return E_POINTER;
        }
        match unsafe { xasync::get_result_size(async_) } {
            Err(hr) => hr,
            Ok(size) => unsafe {
                *buffer_size = size;
                S_OK
            },
        }
    }

    unsafe fn x_user_get_token_and_signature_utf16_result(
        &self,
        async_: *mut XAsyncBlock,
        buffer_size: usize,
        buffer: *mut c_void,
        ptr_to_buffer: *mut *mut XUserGetTokenAndSignatureUtf16Data,
        buffer_used: *mut usize,
    ) -> HRESULT {
        match unsafe {
            xasync::get_result_dyn(async_, null_mut(), buffer_size, buffer, buffer_used)
        } {
            Err(hr) => {
                crate::diag(&format!("token result UTF16 FAILED hr={:#X}", hr.0));
                return hr;
            },
            _ => (),
        }
        /* Authentication stdout logging suppressed. */
        if !ptr_to_buffer.is_null() {
            unsafe { *ptr_to_buffer = buffer as *mut XUserGetTokenAndSignatureUtf16Data };
        }
        /* Authentication stdout logging suppressed. */
        crate::diag("token result UTF16 OK");
        S_OK
    }

    unsafe fn x_user_resolve_issue_with_ui_async(
        &self,
        _user: XUserHandle,
        _url: *const c_char,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        todo!()
    }

    unsafe fn x_user_resolve_issue_with_ui_result(&self, _async_: *mut XAsyncBlock) -> HRESULT {
        todo!()
    }

    unsafe fn x_user_resolve_issue_with_ui_utf16_async(
        &self,
        _user: XUserHandle,
        _url: *const u16,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        todo!()
    }

    unsafe fn x_user_resolve_issue_with_ui_utf16_result(
        &self,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        todo!()
    }

    unsafe fn x_user_register_for_change_event(
        &self,
        _queue: XTaskQueueHandle,
        _context: *mut c_void,
        _callback: *mut XUserChangeEventCallback,
        _token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT {
        /* Authentication stdout logging suppressed. */
        S_OK
    }

    unsafe fn x_user_unregister_for_change_event(
        &self,
        _token: XTaskQueueRegistrationToken,
        _wait: bool,
    ) -> HRESULT {
        // todo!()
        S_OK
    }

    unsafe fn x_user_get_sign_out_deferral(
        &self,
        _deferral: *mut XUserSignOutDeferralHandle,
    ) -> HRESULT {
        todo!()
    }

    unsafe fn x_user_close_sign_out_deferral_handle(
        &self,
        _deferral: XUserSignOutDeferralHandle,
    ) -> HRESULT {
        todo!()
    }

    unsafe fn x_user_register_for_device_association_changed(
        &self,
        _queue: XTaskQueueHandle,
        _context: *mut c_void,
        _callback: *mut XUserDeviceAssociationChangedCallback,
        _token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT {
        todo!()
    }

    unsafe fn x_user_unregister_for_device_association_changed(
        &self,
        _token: XTaskQueueRegistrationToken,
        _wait: bool,
    ) -> HRESULT {
        todo!()
    }

    unsafe fn ___2(&self) {
        todo!()
    }

    unsafe fn ___3(&self) {
        todo!()
    }

    unsafe fn ___4(&self) {
        todo!()
    }

    unsafe fn x_user_is_store_user(&self, _user: XUserHandle) -> HRESULT {
        todo!()
    }

    unsafe fn x_user_platform_remote_connect_set_event_handlers(
        &self,
        _queue: XTaskQueueHandle,
        _handlers: *mut XUserPlatformRemoteConnectEventHandler,
    ) -> HRESULT {
        todo!()
    }

    unsafe fn x_user_platform_remote_connect_cancel_prompt(
        &self,
        _operation: XUserPlatformOperation,
    ) -> HRESULT {
        todo!()
    }

    unsafe fn x_user_platform_spop_prompt_set_event_handlers(
        &self,
        _queue: XTaskQueueHandle,
        _handler: *mut XUserPlatformSpopPromptEventHandlers,
        _context: *mut c_void,
    ) -> HRESULT {
        todo!()
    }

    unsafe fn x_user_platform_spop_prompt_complete(
        &self,
        _operation: XUserPlatformOperation,
        _result: XUserPlatformSpopOperationResult,
    ) -> HRESULT {
        todo!()
    }
}

impl IXUserDevice_Impl for XUser_Impl {
    unsafe fn x_user_find_for_device(
        &self,
        _device_id: *const c_void,
        _handle: *mut XUserHandle,
    ) -> HRESULT {
        todo!()
    }

    unsafe fn x_user_register_for_device_association_changed(
        &self,
        _queue: XTaskQueueHandle,
        _context: *mut c_void,
        _callback: Option<XUserDeviceAssociationChangedCallback>,
        _token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT {
        todo!()
    }

    unsafe fn x_user_unregister_for_device_association_changed(
        &self,
        _token: XTaskQueueRegistrationToken,
        _wait: BOOL,
    ) -> BOOL {
        todo!()
    }

    unsafe fn x_user_get_default_audio_endpoint_utf16(
        &self,
        _user: XUserLocalId,
        _default_audio_endpoint_kind: XUserDefaultAudioEndpointKind,
        _endpoint_id_utf16_count: usize,
        _endpoint_id_utf16: *mut u16,
        _endpoint_id_utf16_used: *mut usize,
    ) -> HRESULT {
        todo!()
    }

    unsafe fn x_user_register_for_default_audio_endpoint_utf16_changed(
        &self,
        _queue: XTaskQueueHandle,
        _context: *mut c_void,
        _callback: Option<XUserDefaultAudioEndpointUtf16ChangedCallback>,
        _token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT {
        todo!()
    }

    unsafe fn x_user_unregister_for_default_audio_endpoint_utf16_changed(
        &self,
        _token: XTaskQueueRegistrationToken,
        _wait: BOOL,
    ) -> BOOL {
        todo!()
    }

    unsafe fn x_user_find_controller_for_user_with_ui_async(
        &self,
        _user: XUserHandle,
        _async_: *mut XAsyncBlock,
    ) -> HRESULT {
        todo!()
    }

    unsafe fn x_user_find_controller_for_user_with_ui_result(
        &self,
        _async_: *mut XAsyncBlock,
        _device_id: *mut c_void,
    ) -> HRESULT {
        todo!()
    }
}

#[cfg(feature = "xuser")]
pub async fn do_sisu(
    client: &Client,
    manager: &TokenManager,
    client_id: &str,
    title_id: i64,
    cache: SignaturePolicyCache,
) -> Result<
    (
        XalAuthenticator,
        xal_new::response::SisuRPSAuthorizationResponse,
        xal_new::response::DeviceToken,
    ),
    Box<dyn std::error::Error>,
> {
    let Token::Legacy(token) = manager.get_user_sts_token()? else {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "error",
        )));
    };
    let scope = "xboxlive.signin";
    let Token::Legacy(device_token) = manager.get_device_sts_token()? else {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "error",
        )));
    };
    let device_token_resp: xodus::models::soap::RequestSecurityTokenResponse =
        xodus::api::live::exchange_device_token(
            client,
            device_token.clone(),
            "{28C08266-F973-4AE6-FFE4-409B249F138F}".to_string(),
            "scope=service::user.auth.xboxlive.com::MBI_SSL&api-version=2.0".to_owned(),
            Some(xodus::models::soap::PolicyReference::token_broker()),
        )
        .await?;

    let Token::Compact(ms_device_token) = device_token_resp.into() else {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "error",
        )));
    };

    let user_token = xodus::api::live::exchange_user_token(
        client,
        token,
        "USERNAME".to_string(),
        device_token,
        None,
        Some("Silent".to_string()),
        client_id.to_string(),
        &[
            (
                format!("scope={scope}&api-version=2.0&clientid={client_id}"),
                Some(xodus::models::soap::PolicyReference::token_broker()),
            ),
            ("http://Passport.NET/tb".to_string(), None),
        ],
    )
    .await?;

    let ExchangeUserTokenOutcome::Issued(
        xodus::models::soap::BodyContent::RequestSecurityTokenResponseCollection(mut collection),
    ) = user_token
    else {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "error",
        )));
    };

    if let Some(sts) = collection.security_tokens.pop() {
        let address = sts.applies_to.endpoint_reference.address.clone();
        let sts: Token = sts.into();
        let address = if let Token::Legacy(legacy) = &sts {
            legacy.key_name.clone().unwrap_or(address)
        } else {
            address
        };
        if let Err(_err) = manager.save_user_token(address, sts) {
            log::warn!("Failed to persist refreshed STS token");
        }
    }
    let token: xodus::models::soap::RequestSecurityTokenResponse =
        collection.security_tokens.remove(0);
    let token: Token = token.into();
    let Token::Compact(user_token) = token else {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "error",
        )));
    };

    let mut auth = XalAuthenticator::new(
        client.clone(),
        XalAppParameters {
            client_id: client_id.to_owned(),
            title_id: Some(title_id.to_string()),
            auth_scopes: vec![],
            redirect_uri: None,
            client_secret: None,
        },
        XalClientParameters {
            user_agent: "XAL GRTS 2025.11.20251105.000".to_string(),
            device_type: DeviceType::WIN32,
            client_version: "10.0.22621".to_string(),
            query_display: String::new(),
        },
        "RETAIL".to_owned(),
        cache,
    );

    let data = auth
        .get_device_token_rps(ms_device_token.to_owned())
        .await?;
    let resp = auth
        .sisu_authorize_rps(&user_token, &data.token, None)
        .await
        .expect("ok");
    Ok((auth, resp, data))
}

#[cfg(feature = "xuser")]
async fn get_xsts_token(
    handle: tokio::runtime::Handle,
    user: Arc<tokio::sync::Mutex<XuserHandleObjectAuth>>,
    url: String,
) -> String {
    let token = handle
        .spawn(async move {
            let mut user = user.lock().await;
            let device_token = user.device_token.clone();
            let title_token = user.auth.title_token.clone();
            let user_token = user.auth.user_token.clone();
            let mut pol = user.policy.clone();
            let ra = pol.find_relying_party_for_url(&url).await;
            let rb = user.def_policy.find_relying_party_for_url(&url).await;
            let relying_party = match ra {
                Ok(Some(rp)) => rp,
                _ => match rb {
                    Ok(Some(rp)) => rp,
                    _ => {
                        let url = url::Url::parse(&url).unwrap();
                        let host = url.host_str().unwrap();
                        if host.ends_with(".atoma.cloud")
                            || host.ends_with(".fatshark.services")
                            || host.ends_with(".fatsharkgames.se")
                        {
                            "rp://bsp-auth.fatsharkgames.se/".to_owned()
                        } else {
                            panic!("No relying party found for url: {}", url);
                        }
                    }
                },
            };
            let token = user
                .authenticator
                .get_xsts_token(
                    Some(&device_token),
                    Some(&title_token),
                    Some(&user_token),
                    &relying_party,
                )
                .await
                .unwrap();
            /* Authentication stdout logging suppressed. */
            token.authorization_header_value()
        })
        .await
        .unwrap();
    token
}

#[cfg(feature = "xuser")]
async fn get_gamerpicture(
    handle: tokio::runtime::Handle,
    user: Arc<tokio::sync::Mutex<XuserHandleObjectAuth>>,
    xuid: u64,
    size: XUserGamerPictureSize,
) -> Bytes {
    let token = handle
        .spawn(async move {
            let user = user.lock().await;
            let user_token = user.auth.authorization_token.authorization_header_value();
            let client = reqwest::Client::builder()
                .use_rustls_tls()
                .http1_only()
                .connection_verbose(false)
                .pool_max_idle_per_host(0)
                .connect_timeout(std::time::Duration::from_secs(5))
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap();
            let xuid = xuid.to_string();
            let prof = xbox_utils::fetch_user_profiles(&client, &user_token, &[&xuid])
                .await
                .unwrap();
            let prof = prof.into_iter().map(|(_, v)| v).next().unwrap();

            let pic = prof.picture.unwrap()
                + match size {
                    XUserGamerPictureSize::Small => "&w=64&h=64",
                    XUserGamerPictureSize::Medium => "&w=208&h=208",
                    XUserGamerPictureSize::Large => "&w=424&h=424",
                    XUserGamerPictureSize::ExtraLarge => "&w=1080&h=1080",
                };

            let req = client.get(&pic).send().await.unwrap();
            let data = req.bytes().await.unwrap();
            Ok::<_, HRESULT>(data)
        })
        .await
        .unwrap()
        .unwrap();
    token
}

#[cfg(feature = "xuser")]
pub async fn do_license_token(
    client: &Client,
    manager: &TokenManager,
    products: Vec<String>,
    custom_developer_string: String,
) -> Result<String, Box<dyn std::error::Error>> {
    let Token::Legacy(token) = manager.get_user_sts_token()? else {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::AddrNotAvailable,
            "error",
        )));
    };
    let Token::Legacy(device_token) = manager.get_device_sts_token()? else {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "error",
        )));
    };
    let device_token_resp: xodus::models::soap::RequestSecurityTokenResponse =
        xodus::api::live::exchange_device_token(
            client,
            device_token.clone(),
            "{d6d5a677-0872-4ab0-9442-bb792fce85c5}".to_string(),
            "www.microsoft.com".to_owned(),
            Some(xodus::models::soap::PolicyReference::mbi_ssl()),
        )
        .await?;

    let Token::Compact(ms_device_token) = device_token_resp.into() else {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::ArgumentListTooLong,
            "error",
        )));
    };
    let user = manager.get_user().unwrap();

    let user_token = xodus::api::live::exchange_user_token(
        client,
        token,
        user.username,
        device_token,
        None,
        Some("Silent".to_string()),
        "{d6d5a677-0872-4ab0-9442-bb792fce85c5}".to_string(),
        &[(
            "www.microsoft.com".to_owned(),
            Some(xodus::models::soap::PolicyReference::mbi_ssl()),
        )],
    )
    .await?;

    let user_token: Token = match user_token {
        ExchangeUserTokenOutcome::Fault(_) => {
            todo!()
        }
        ExchangeUserTokenOutcome::Issued(BodyContent::RequestSecurityTokenResponseCollection(
            mut collection,
        )) => {
            let token = collection.security_tokens.remove(0);
            token.into()
        }
        ExchangeUserTokenOutcome::Issued(BodyContent::RequestSecurityTokenResponse(token)) => {
            (*token).into()
        }
        _ => unreachable!("Only responses are handled"),
    };
    let Token::Compact(user_token) = user_token else {
        todo!();
    };

    let token = get_license_token(
        client,
        ms_device_token,
        user_token,
        user.puid,
        products,
        custom_developer_string,
    )
    .await?;

    Ok(token)
}

// {"parentProductId":"9PGW18NPBZV5","enforceSellableBy":true,"relatedProductIds":"[\"9NZ12RV7B7R3\",\"9P0WDBKKS7MK\",\"9MV1BF8J0TTX\",\"9P427LFN9KCD\",\"9NBLGGH2JHXJ\",\"9MV69L4JSD31\",\"9P1XF6ZQGV3R\",\"9P5JQ1XPRGN6\",\"9N98Z825TNFW\",\"9P4BFQNXLMDR\",\"9NGG3CWJMC7V\",\"9PBP71DDVCT9\",\"9PB1LJZFN9XK\",\"9P2VR3K66TJX\",\"9P4HCS6S5C2K\",\"9N5KX36SQJ9Q\",\"9P8MK4NC0LJB\",\"9P5KH0238TPW\",\"9P45BPZCP004\",\"9PKCNQ57B2JG\",\"9N6184JJ7NSG\"]","customDeveloperString":"c0c83208-ea4f-4c64-b4f4-9667120cf9f2","beneficiaries":[{"identityValue":"t=<token>","localTicketReference":"<reference>","identityType":"Msa"}]}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseTokenRequest {
    pub parent_product_id: String,
    pub enforce_sellable_by: bool,
    pub related_product_ids: Vec<String>,
    pub custom_developer_string: String,
    pub beneficiaries: Vec<LicenseUserIdentity>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseTokenResponse {
    pub license_token: String,
}

pub async fn get_license_token(
    client: &reqwest::Client,
    device_ms_token: String,
    user_ms_token: String,
    ticket_reference: String,
    products: Vec<String>,
    custom_developer_string: String,
) -> Result<String, Box<dyn std::error::Error>> {
    let response = client
        .post("https://licensing.mp.microsoft.com/v8.0/licenseToken")
        .header("from", "XboxLicenseManager")
        .header("Authorization", device_ms_token)
        .header(
            "user-agent",
            "XboxLm-PC/Microsoft.GamingServices_32.107.4002.0_x64__8wekyb3d8bbwe",
        )
        .json(&LicenseTokenRequest {
            parent_product_id: load_game_config().unwrap().store_id, //
            enforce_sellable_by: true,
            related_product_ids: products,
            custom_developer_string: custom_developer_string,
            beneficiaries: vec![LicenseUserIdentity {
                identity_type: "Msa".to_string(),
                identity_value: user_ms_token,
                local_ticket_reference: ticket_reference,
            }],
        })
        .send()
        .await?;

    // let resp = response.text().await.unwrap();

    let token_resp: LicenseTokenResponse = response.json().await?;

    Ok(token_resp.license_token)
}

#[tokio::test]
#[ignore = "Requires an explicitly enabled licensed Microsoft account and credential broker"]
async fn live_license_token() {
    unsafe {
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
        /* Authentication stdout logging suppressed. */
        secrets::init_secrets().expect("Unable to initialize credentials");
        let tokens: TokenManager = TokenManager::with_keychain_and_memory();

        // let r = client
        //     .get("https://title.mgt.xboxlive.com/titles/default/endpoints?type=1")
        //     .send()
        //     .await
        //     .unwrap()
        //     .json::<xal_new::response::TitleEndpointsResponse>()
        //     .await
        //     .unwrap();


        // let def_policy = SignaturePolicyCache::new(r);

        // let (_, resp, _) = do_sisu(&client, &tokens, "0000000040159362", 896928775, def_policy)
        //     .await
        //     .expect("ok");




        let token = do_license_token(
            &client,
            &tokens,
            vec!["9NZ12RV7B7R3".to_owned(), "9P0WDBKKS7MK".to_owned()],
            "0A5E1450-0D5F-40E3-A8BC-543707684BF4".to_owned(),
        )
        .await
        .unwrap();
        /* Authentication stdout logging suppressed. */
        // do_sisu(&client, &tokens, ).await.unwrap();
    }
}

pub async fn load_game_config_async() -> Option<Game> {
    let mut config: Option<Game> = None;
    loop {
        let mut path = [0u16; MAX_PATH as usize];
        let len = unsafe { GetModuleFileNameW(None, &mut path) };
        let path = String::from_utf16_lossy(&path[..len as usize]);
        let mut path = Path::new(&path);

        let Some(parent) = path.parent() else {
            break;
        };

        if let Ok(mut fs) = tokio::fs::File::open(path.join("MicrosoftGame.config")).await {
            use tokio::io::AsyncReadExt;
            let mut bytes = String::new();
            fs.read_to_string(&mut bytes).await.unwrap();

            config = Some(quick_xml::de::from_str(&bytes).unwrap());
            break;
        }

        path = parent;
    }
    config
}

pub fn load_game_config() -> Option<Game> {
    let mut path = [0u16; MAX_PATH as usize];
    let len = unsafe { GetModuleFileNameW(None, &mut path) };
    let path = String::from_utf16_lossy(&path[..len as usize]);
    let mut path = Path::new(&path);
    let mut config: Option<Game> = None;
    loop {
        /* Authentication stdout logging suppressed. */

        let Some(parent) = path.parent() else {
            break;
        };

        if let Ok(mut fs) = std::fs::File::open(path.join("MicrosoftGame.config")) {
            let mut bytes = String::new();
            fs.read_to_string(&mut bytes).unwrap();

            config = Some(quick_xml::de::from_str(&bytes).unwrap());
            break;
        }

        path = parent;
    }
    config
}

/// Resolve only an authenticated user; save storage must never use a fallback identity.
pub(crate) unsafe fn authenticated_xuid(user: XUserHandle) -> Option<u64> {
    unsafe { IXUserHandle::from_raw_borrowed(&user).map(|handle| handle.get_xuid()).filter(|id| *id != 0) }
}
