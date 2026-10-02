use std::ffi::{c_char,c_void,CStr}; use std::mem::{size_of,align_of,offset_of}; use std::ptr::{null_mut,slice_from_raw_parts_mut};
#[repr(C)]
pub struct XUserGetTokenAndSignatureData {
    pub token_size: usize,
    pub signature_size: usize,
    pub token: *const c_char,
    pub signature: *const c_char,
}
#[repr(C)]
pub struct XUserGetTokenAndSignatureUtf16Data {
    pub token_count: usize,
    pub signature_count: usize,
    pub token: *const u16,
    pub signature: *const u16,
}
#[test] fn token_layout_matches_gdk_win64() {
 assert_eq!(size_of::<XUserGetTokenAndSignatureData>(),32);assert_eq!(align_of::<XUserGetTokenAndSignatureData>(),8);
 assert_eq!(offset_of!(XUserGetTokenAndSignatureData,token),16);assert_eq!(offset_of!(XUserGetTokenAndSignatureData,signature),24);
 assert_eq!(size_of::<XUserGetTokenAndSignatureUtf16Data>(),32);assert_eq!(offset_of!(XUserGetTokenAndSignatureUtf16Data,token),16);
}
#[test] fn ansi_token_terminates_in_dirty_buffer(){unsafe{
 let token=String::from("synthetic-token");let expected=token.as_bytes().to_vec();let needed=32+expected.len()+1;
 let write=move |b: *mut c_void, s: usize| {
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
                            };let mut buffer=[0xa5a5a5a5a5a5a5a5u64;64];let b=buffer.as_mut_ptr().cast::<c_void>();write(b,needed);
 let data=&*b.cast::<XUserGetTokenAndSignatureData>();assert_eq!(data.token_size,expected.len()+1);assert_eq!(data.signature_size,0);assert!(data.signature.is_null());
 assert_eq!(data.token.cast::<u8>(),b.cast::<u8>().add(32));assert_eq!(std::slice::from_raw_parts(data.token.cast::<u8>(),expected.len()),expected);
 assert_eq!(*data.token.cast::<u8>().add(expected.len()),0);assert_eq!(*b.cast::<u8>().add(needed),0xa5);
}}
#[test] fn utf16_counts_code_units_and_terminates(){unsafe{
 let token=String::from("synthetic-♥-🙂");let expected:Vec<u16>=token.encode_utf16().collect();let token_count=expected.len();let token_start=32;let needed=32+(token_count+1)*2;
 let write=move |b: *mut c_void, s: usize| {
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
                            };let mut buffer=[0xa5a5a5a5a5a5a5a5u64;64];let b=buffer.as_mut_ptr().cast::<c_void>();write(b,needed);
 let data=&*b.cast::<XUserGetTokenAndSignatureUtf16Data>();assert_eq!(data.token_count,expected.len()+1);assert_eq!(data.signature_count,0);assert!(data.signature.is_null());
 assert_eq!(std::slice::from_raw_parts(data.token,expected.len()),expected);assert_eq!(*data.token.add(expected.len()),0);assert_eq!(*b.cast::<u8>().add(needed),0xa5);
}}

#[derive(Debug,PartialEq)] struct HRESULT(i32); const E_FAIL:HRESULT=HRESULT(0x80004005u32 as i32); const S_OK:HRESULT=HRESULT(0);
fn copy_cached_component(value:*const c_char,component:u32,gamertag_size:usize,gamertag:*mut c_char,gamertag_used:*mut usize)->HRESULT{            if value.is_null() { return HRESULT(0x80070057u32 as i32); }
            let bytes = unsafe { CStr::from_ptr(value) }.to_bytes_with_nul();
            if bytes.len() == 1 && component != 2 {
                return E_FAIL;
            }
            if gamertag_size < bytes.len() { return HRESULT(0x8007007au32 as i32); }
            unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), gamertag.cast::<u8>(), bytes.len()); }
            if !gamertag_used.is_null() { unsafe { *gamertag_used = bytes.len(); } }
            S_OK}

#[test] fn gamertag_copy_checks_capacity_without_overwrite(){
 let value=std::ffi::CString::new("SyntheticTag").unwrap();let mut out=[0xa5u8;16];let mut used=0usize;
 assert_eq!(copy_cached_component(value.as_ptr(),0,4,out.as_mut_ptr().cast(),&mut used),HRESULT(0x8007007au32 as i32));assert_eq!(out,[0xa5;16]);
 assert_eq!(copy_cached_component(value.as_ptr(),0,16,out.as_mut_ptr().cast(),&mut used),S_OK);assert_eq!(used,13);assert_eq!(&out[..13],b"SyntheticTag\0");assert_eq!(out[13],0xa5);
}
#[test] fn empty_suffix_allowed_but_missing_name_is_not_fabricated(){
 let value=std::ffi::CString::new("").unwrap();let mut out=[0xa5u8;4];let mut used=0;
 assert_eq!(copy_cached_component(value.as_ptr(),0,4,out.as_mut_ptr().cast(),&mut used),E_FAIL);
 assert_eq!(copy_cached_component(value.as_ptr(),2,4,out.as_mut_ptr().cast(),&mut used),S_OK);assert_eq!(used,1);assert_eq!(out,[0,0xa5,0xa5,0xa5]);
}
