#!/usr/bin/env python3
"""Run the actual result-writing closures with synthetic tokens and dirty buffers.
No game, account, keychain, or network calls are made by these tests.
"""
from pathlib import Path
import re,subprocess
ROOT=Path(__file__).resolve().parent
s=(ROOT.parent/'xgameruntime-rs-pr12/src/user.rs').read_text()
def closure(start):
 i=s.index('move |b: *mut c_void, s: usize| {',s.index(start));end=i;depth=0
 for j in range(s.index('{',i),len(s)):
  if s[j]=='{':depth+=1
  if s[j]=='}':
   depth-=1
   if not depth:return s[i:j+1]
 raise AssertionError('Closure not found')
ansi=closure('let req_size = size_of::<XUserGetTokenAndSignatureData>()')
wide=closure('let token_start = size_of::<XUserGetTokenAndSignatureUtf16Data>()')
types='\n'.join(re.search(r'#\[repr\(C\)\]\npub struct '+name+r' \{[^}]+\}',s).group() for name in ['XUserGetTokenAndSignatureData','XUserGetTokenAndSignatureUtf16Data'])
code='use std::ffi::{c_char,c_void,CStr}; use std::mem::{size_of,align_of,offset_of}; use std::ptr::{null_mut,slice_from_raw_parts_mut};\n'+types+'\n'
code+='''#[test] fn token_layout_matches_gdk_win64() {
 assert_eq!(size_of::<XUserGetTokenAndSignatureData>(),32);assert_eq!(align_of::<XUserGetTokenAndSignatureData>(),8);
 assert_eq!(offset_of!(XUserGetTokenAndSignatureData,token),16);assert_eq!(offset_of!(XUserGetTokenAndSignatureData,signature),24);
 assert_eq!(size_of::<XUserGetTokenAndSignatureUtf16Data>(),32);assert_eq!(offset_of!(XUserGetTokenAndSignatureUtf16Data,token),16);
}
#[test] fn ansi_token_terminates_in_dirty_buffer(){unsafe{
 let token=String::from("synthetic-token");let expected=token.as_bytes().to_vec();let needed=32+expected.len()+1;
 let write='''+ansi+''';let mut buffer=[0xa5a5a5a5a5a5a5a5u64;64];let b=buffer.as_mut_ptr().cast::<c_void>();write(b,needed);
 let data=&*b.cast::<XUserGetTokenAndSignatureData>();assert_eq!(data.token_size,expected.len()+1);assert_eq!(data.signature_size,0);assert!(data.signature.is_null());
 assert_eq!(data.token.cast::<u8>(),b.cast::<u8>().add(32));assert_eq!(std::slice::from_raw_parts(data.token.cast::<u8>(),expected.len()),expected);
 assert_eq!(*data.token.cast::<u8>().add(expected.len()),0);assert_eq!(*b.cast::<u8>().add(needed),0xa5);
}}
#[test] fn utf16_counts_code_units_and_terminates(){unsafe{
 let token=String::from("synthetic-♥-🙂");let expected:Vec<u16>=token.encode_utf16().collect();let token_count=expected.len();let token_start=32;let needed=32+(token_count+1)*2;
 let write='''+wide+''';let mut buffer=[0xa5a5a5a5a5a5a5a5u64;64];let b=buffer.as_mut_ptr().cast::<c_void>();write(b,needed);
 let data=&*b.cast::<XUserGetTokenAndSignatureUtf16Data>();assert_eq!(data.token_count,expected.len()+1);assert_eq!(data.signature_count,0);assert!(data.signature.is_null());
 assert_eq!(std::slice::from_raw_parts(data.token,expected.len()),expected);assert_eq!(*data.token.add(expected.len()),0);assert_eq!(*b.cast::<u8>().add(needed),0xa5);
}}
'''
# Exercise the exact production cached-component copy logic, including bounds.
start=s.index('            if value.is_null() { return HRESULT(0x80070057u32 as i32); }',s.index('impl IXUser2_Impl'))
end=s.index('            S_OK',start)+len('            S_OK')
copy=s[start:end]
code += '\n#[derive(Debug,PartialEq)] struct HRESULT(i32); const E_FAIL:HRESULT=HRESULT(0x80004005u32 as i32); const S_OK:HRESULT=HRESULT(0);\n'
code += 'fn copy_cached_component(value:*const c_char,component:u32,gamertag_size:usize,gamertag:*mut c_char,gamertag_used:*mut usize)->HRESULT{'+copy+'}\n'
code += r"""
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
"""
p=ROOT/'token-abi-tests.rs';p.write_text(code)
rustc=subprocess.check_output(['rustup','which','--toolchain','stable','rustc'],text=True).strip()
subprocess.run([rustc,'--test','--edition=2024',str(p),'-o',str(ROOT/'token-abi-tests')],check=True)
subprocess.run([str(ROOT/'token-abi-tests')],check=True)
subprocess.run([rustc,'--test','--edition=2024','--target','x86_64-pc-windows-gnu','-C','linker=x86_64-w64-mingw32-gcc',str(p),'-o',str(ROOT/'token-abi-tests.exe')],check=True)
print('Cross-compiled the same ABI tests for Win64.')
