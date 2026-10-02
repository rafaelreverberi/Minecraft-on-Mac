use crate::com::query_api_impl;
use crate::{E_FAIL, S_OK};

use crate::results::*;
pub use crate::threading::{IXAsync, XAsyncBlock, XAsyncOp, XAsyncProvider, XAsyncProviderData};
use std::ffi::{c_char, c_void};
use std::mem::size_of;
use std::pin::Pin;
use std::ptr::null_mut;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use windows_core::{GUID, HRESULT, Interface};

pub const CLSID_XASYNC: GUID = GUID::from_u128(0x073b7dcb_1fcf_4030_94be_e3c9eb623428);

pub fn interface() -> Result<IXAsync, HRESULT> {
    let mut out = std::ptr::null_mut();
    let hr = query_api_impl(&CLSID_XASYNC, &IXAsync::IID, &mut out);
    if hr != S_OK {
        return Err(hr);
    }
    Ok(unsafe { IXAsync::from_raw(out) })
}

fn result<T>(r: T, h: HRESULT) -> Result<T, HRESULT> {
    if h == S_OK { Ok(r) } else { Err(h) }
}

pub unsafe fn begin(
    async_block: *mut XAsyncBlock,
    context: *mut c_void,
    identity: *const c_void,
    identity_name: *const c_char,
    provider: XAsyncProvider,
) -> Result<(), HRESULT> {
    let xasync = interface()?;
    let hr = unsafe {
        xasync.x_async_begin(
            async_block.cast(),
            context,
            identity.cast_mut(),
            identity_name.cast_mut(),
            Some(provider),
        )
    };
    result((), hr)
}

pub unsafe fn schedule(async_block: *mut XAsyncBlock, delay_ms: u32) -> Result<(), HRESULT> {
    let xasync = interface()?;
    let hr = unsafe { xasync.x_async_schedule(async_block.cast(), delay_ms) };
    result((), hr)
}

pub unsafe fn complete(
    async_block: *mut XAsyncBlock,
    result: HRESULT,
    required_buffer_size: usize,
) -> Result<(), HRESULT> {
    let xasync = interface()?;
    unsafe { xasync.x_async_complete(async_block.cast(), result, required_buffer_size) };
    Ok(())
}

pub unsafe fn get_result<T>(
    async_block: *mut XAsyncBlock,
    identity: *const c_void,
    out: *mut T,
) -> Result<(), HRESULT> {
    let xasync = interface()?;
    let mut buffer_used = 0usize;
    let hr = unsafe {
        xasync.x_async_get_result(
            async_block.cast(),
            identity.cast_mut(),
            size_of::<T>() as usize,
            out.cast(),
            &mut buffer_used,
        )
    };
    result((), hr)
}

pub unsafe fn get_result_dyn(
    async_block: *mut XAsyncBlock,
    identity: *const c_void,
    buffer_size: usize,
    buffer: *mut c_void,
    buffer_used: *mut usize,
) -> Result<(), HRESULT> {
    let xasync = interface()?;
    if !buffer_used.is_null() {
        unsafe { *buffer_used = 0 };
    }
    let size = unsafe { get_result_size(async_block) }?;
    if buffer_size < size {
        return Err(E_FAIL);
    }
    let hr = unsafe {
        xasync.x_async_get_result(
            async_block.cast(),
            identity.cast_mut(),
            buffer_size,
            buffer,
            buffer_used,
        )
    };
    result((), hr)
}

pub unsafe fn get_status(async_block: *mut XAsyncBlock, wait: bool) -> Result<(), HRESULT> {
    let xasync = interface()?;
    let hr = unsafe { xasync.x_async_get_status(async_block.cast(), wait.into()) };
    result((), hr)
}

pub unsafe fn get_result_size(async_block: *mut XAsyncBlock) -> Result<usize, HRESULT> {
    let xasync = interface()?;
    let mut buffer_size: usize = 0;
    let hr = unsafe { xasync.x_async_get_result_size(async_block.cast(), &mut buffer_size) };
    result(buffer_size, hr)
}

struct XAsyncContextHelper<T: Sized> {
    result: HRESULT,
    canceled: bool,
    payload: Option<T>,
    future: Pin<Box<dyn Future<Output = Result<T, HRESULT>> + Send + 'static>>,
}

struct XAsyncWaker {
    block: *mut XAsyncBlock,
}

unsafe impl Sync for XAsyncWaker {}
unsafe impl Send for XAsyncWaker {}

impl Wake for XAsyncWaker {
    fn wake(self: Arc<Self>) {
        let _ = unsafe { schedule(self.block, 0) };
    }
}

unsafe extern "system" fn run_async_helper<T: Sized>(
    op: XAsyncOp,
    data: *const XAsyncProviderData,
) -> HRESULT {
    let Some(data) = (unsafe { data.as_ref() }) else {
        return E_POINTER;
    };
    let Some(async_context) = (unsafe { (data.context as *mut XAsyncContextHelper<T>).as_mut() })
    else {
        return E_POINTER;
    };

    assert_ne!(unsafe { &*data.async_ }.queue, null_mut());

    match op {
        XAsyncOp::Begin => unsafe { schedule(data.async_, 0) }
            .map(|_| S_OK)
            .unwrap_or_else(|hr| hr),
        XAsyncOp::DoWork => {
            if async_context.canceled {
                async_context.result = E_ABORT;
            } else {
                let waker = Waker::from(Arc::new(XAsyncWaker { block: data.async_ }));
                let mut cx = Context::from_waker(&waker);
                match async_context.future.as_mut().poll(&mut cx) {
                    Poll::Ready(value) => {
                        match value {
                            Ok(value) => {
                                async_context.result = S_OK;
                                async_context.payload = Some(value);
                            }
                            Err(hr) => async_context.result = hr,
                        };
                    }
                    Poll::Pending => {
                        return E_PENDING;
                    }
                }
            }
            unsafe { complete(data.async_, async_context.result, size_of::<T>()) }
                .map(|_| S_OK)
                .unwrap_or_else(|hr| hr)
        }
        XAsyncOp::GetResult => {
            if async_context.result == S_OK
                && let Some(payload) = &async_context.payload
            {
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        (payload as *const T).cast::<u8>(),
                        data.buffer.cast::<u8>(),
                        size_of::<T>(),
                    );
                }
            }
            S_OK
        }
        XAsyncOp::Cancel => {
            async_context.canceled = true;
            S_OK
        }
        XAsyncOp::Cleanup => {
            unsafe {
                drop(Box::from_raw(async_context));
            }
            S_OK
        }
    }
}

pub unsafe fn run<T: Sized, F>(async_: *mut XAsyncBlock, future: F) -> HRESULT
where
    F: Future<Output = Result<T, HRESULT>> + Send + 'static,
{
    if async_.is_null() {
        return S_OK;
    }

    let async_context = Box::into_raw(Box::new(XAsyncContextHelper {
        canceled: false,
        payload: None as Option<T>,
        result: E_ABORT,
        future: Box::pin(future),
    }));
    match unsafe {
        begin(
            async_,
            async_context.cast(),
            null_mut(),
            c"run_async".as_ptr(),
            run_async_helper::<T>,
        )
    } {
        Ok(_) => S_OK,
        Err(hr) => {
            unsafe {
                drop(Box::from_raw(async_context));
            }
            return hr;
        }
    }
}

struct XAsyncContextHelperDyn<F, R>
where
    F: Future<Output = Result<(R, usize), HRESULT>> + Send + 'static,
    R: FnMut(*mut c_void, usize) -> usize,
{
    result: HRESULT,
    canceled: bool,
    payload: Option<R>,
    future: Pin<Box<F>>,
}

unsafe extern "system" fn run_async_helper_dyn<F, R>(
    op: XAsyncOp,
    data: *const XAsyncProviderData,
) -> HRESULT
where
    F: Future<Output = Result<(R, usize), HRESULT>> + Send + 'static,
    R: FnMut(*mut c_void, usize) -> usize,
{
    let Some(data) = (unsafe { data.as_ref() }) else {
        return E_POINTER;
    };
    let Some(async_context) =
        (unsafe { (data.context as *mut XAsyncContextHelperDyn<F, R>).as_mut() })
    else {
        return E_POINTER;
    };

    assert_ne!(unsafe { &*data.async_ }.queue, null_mut());

    match op {
        XAsyncOp::Begin => unsafe { schedule(data.async_, 0) }
            .map(|_| S_OK)
            .unwrap_or_else(|hr| hr),
        XAsyncOp::DoWork => {
            let mut required_buffer_size = 0;
            if async_context.canceled {
                async_context.result = E_ABORT;
            } else {
                let waker = Waker::from(Arc::new(XAsyncWaker { block: data.async_ }));
                let mut cx = Context::from_waker(&waker);
                match async_context.future.as_mut().poll(&mut cx) {
                    Poll::Ready(value) => {
                        match value {
                            Ok((value, len)) => {
                                async_context.result = S_OK;
                                required_buffer_size = len;
                                async_context.payload = Some(value);
                            }
                            Err(hr) => async_context.result = hr,
                        };
                    }
                    Poll::Pending => {
                        return E_PENDING;
                    }
                }
            }
            unsafe { complete(data.async_, async_context.result, required_buffer_size) }
                .map(|_| S_OK)
                .unwrap_or_else(|hr| hr)
        }
        XAsyncOp::GetResult => {
            if async_context.result == S_OK
                && let Some(payload) = &mut async_context.payload
            {
                payload(data.buffer, data.buffer_size);
            }
            S_OK
        }
        XAsyncOp::Cancel => {
            async_context.canceled = true;
            S_OK
        }
        XAsyncOp::Cleanup => {
            unsafe {
                drop(Box::from_raw(async_context));
            }
            S_OK
        }
    }
}

pub unsafe fn run_dyn<F, R>(async_: *mut XAsyncBlock, future: F) -> HRESULT
where
    F: Future<Output = Result<(R, usize), HRESULT>> + Send + 'static,
    R: FnMut(*mut c_void, usize) -> usize,
{
    if async_.is_null() {
        return S_OK;
    }

    let async_context = Box::into_raw(Box::new(XAsyncContextHelperDyn {
        canceled: false,
        payload: None as Option<R>,
        result: E_ABORT,
        future: Box::pin(future),
    }));
    match unsafe {
        begin(
            async_,
            async_context.cast(),
            null_mut(),
            c"run_async_dync_size".as_ptr(),
            run_async_helper_dyn::<F, R>,
        )
    } {
        Ok(_) => S_OK,
        Err(hr) => {
            unsafe {
                drop(Box::from_raw(async_context));
            }
            return hr;
        }
    }
}

struct XsyncContextHelper<T: Sized, F: Fn() -> Result<T, HRESULT>> {
    result: HRESULT,
    canceled: bool,
    payload: Option<T>,
    future: F,
}

unsafe extern "system" fn run_sync_helper<T: Sized, F: Fn() -> Result<T, HRESULT>>(
    op: XAsyncOp,
    data: *const XAsyncProviderData,
) -> HRESULT {
    let Some(data) = (unsafe { data.as_ref() }) else {
        return E_POINTER;
    };
    let Some(async_context) = (unsafe { (data.context as *mut XsyncContextHelper<T, F>).as_mut() })
    else {
        return E_POINTER;
    };

    match op {
        XAsyncOp::Begin => unsafe {
            assert_ne!((&*data.async_).queue, null_mut());
            match (async_context.future)() {
                Ok(value) => {
                    async_context.result = S_OK;
                    async_context.payload = Some(value);
                }
                Err(hr) => async_context.result = hr,
            };
            complete(data.async_, async_context.result, size_of::<T>())
                .map(|_| S_OK)
                .unwrap_or_else(|hr| hr)
        },
        XAsyncOp::DoWork => S_OK,
        XAsyncOp::GetResult => {
            if async_context.result == S_OK
                && let Some(payload) = &async_context.payload
            {
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        (payload as *const T).cast::<u8>(),
                        data.buffer.cast::<u8>(),
                        size_of::<T>(),
                    );
                }
            }
            S_OK
        }
        XAsyncOp::Cancel => {
            async_context.canceled = true;
            S_OK
        }
        XAsyncOp::Cleanup => {
            unsafe {
                drop(Box::from_raw(async_context));
            }
            S_OK
        }
    }
}

pub unsafe fn run_sync<T: Sized, F>(async_: *mut XAsyncBlock, future: F) -> HRESULT
where
    F: Fn() -> Result<T, HRESULT>,
{
    if async_.is_null() {
        return S_OK;
    }

    let async_context = Box::into_raw(Box::new(XsyncContextHelper {
        canceled: false,
        payload: None as Option<T>,
        result: E_ABORT,
        future: future,
    }));
    match unsafe {
        begin(
            async_,
            async_context.cast(),
            null_mut(),
            c"run_sync".as_ptr(),
            run_sync_helper::<T, F>,
        )
    } {
        Ok(_) => S_OK,
        Err(hr) => {
            unsafe {
                drop(Box::from_raw(async_context));
            }
            return hr;
        }
    }
}
