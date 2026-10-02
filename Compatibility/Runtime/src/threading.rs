use std::{
    cell::Cell,
    ffi::{c_char, c_void},
    hint::spin_loop,
    io, mem,
    ptr::null_mut,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicPtr, AtomicU32, AtomicU64, Ordering},
    },
};

use tokio_util::sync::CancellationToken;
use windows::{
    threadpoolapiset::{
        CloseThreadpoolWait, CreateThreadpoolWait, SetThreadpoolWait,
        WaitForThreadpoolWaitCallbacks,
    },
    winbase::WAIT_OBJECT_0,
    winnt::{self, HANDLE, PTP_CALLBACK_INSTANCE, PTP_WAIT, TP_WAIT_RESULT},
};
use windows_core::{
    BOOL, ComObjectInterface, HRESULT, IUnknown, Interface, InterfaceRef, implement, interface,
};

use crate::{
    E_FAIL,
    results::{E_ABORT, E_PENDING, E_POINTER, S_OK},
    xasync,
};

#[repr(u32)]
pub enum XAsyncOp {
    Begin,
    DoWork,
    GetResult,
    Cancel,
    Cleanup,
}

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum XTaskQueueDispatchMode {
    Manual,
    ThreadPool,
    SerializedThreadPool,
    Immediate,
    Invalid,
}
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum XTaskQueuePort {
    Work,
    Completion,
}

#[repr(C)]
pub struct XAsyncProviderData {
    pub async_: *mut XAsyncBlock,
    pub buffer_size: usize,
    pub buffer: *mut c_void,
    pub context: *mut c_void,
}

pub type XTaskQueueHandle = *mut c_void;
pub type XTaskQueuePortHandle = *mut c_void;
pub type XAsyncCompletionRoutine = unsafe extern "system" fn(*mut XAsyncBlock);
pub type XAsyncWork = unsafe extern "system" fn(*mut XAsyncBlock) -> HRESULT;
pub type XAsyncProvider =
    unsafe extern "system" fn(op: XAsyncOp, data: *const XAsyncProviderData) -> HRESULT;
pub type XTaskQueueCallback = unsafe extern "system" fn(context: *mut c_void, cancelled: bool);
pub type XTaskQueueTerminatedCallback = unsafe extern "system" fn(context: *mut c_void);
pub type XTaskQueueMonitorCallback =
    unsafe extern "system" fn(context: *mut c_void, queue: XTaskQueueHandle, port: XTaskQueuePort);
pub type XTaskQueueRegistrationToken = u64;

const XASYNC_INT_MAGIC: u32 = 0x5853594e; // "XSYN"
const XASYNC_INITIALIZE_MAGIC: u32 = 0x5853394e; // "XSIN"

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XAsyncBlock {
    pub queue: XTaskQueueHandle,
    pub context: *mut c_void,
    pub callback: Option<XAsyncCompletionRoutine>,
    pub internal: [u8; size_of::<*mut c_void>() * 4],
}

#[interface("ECD1C26B-E34D-4E41-987E-C4C349C667CF")]
unsafe trait IXAsyncState: IUnknown {
    unsafe fn get_local_block(&self) -> *mut XAsyncBlock;
    unsafe fn get_user_block(&self) -> *mut XAsyncBlock;
    unsafe fn get_provider_data(&self) -> *mut XAsyncProviderData;
    unsafe fn get_result_size(&self) -> usize;
    unsafe fn set_result_size(&self, required_buffer_size: usize);
    unsafe fn get_provider(&self) -> XAsyncProvider;
    unsafe fn notify_all(&self);
    unsafe fn wait(&self);
    unsafe fn get_queue(&self) -> ITaskQueue;
}

#[implement(IXAsyncState)]
struct XAsyncState {
    local_block: XAsyncBlock,
    user_block: *mut XAsyncBlock,
    provider_data: XAsyncProviderData,
    provider: XAsyncProvider,
    waiter: Arc<(Mutex<usize>, Condvar)>,
    queue: ITaskQueue,
}

impl Drop for XAsyncState {
    fn drop(&mut self) {
        println!(
            "XAsyncState::drop called for local_block: {:p}, user_block: {:p}, provider_data: async_: {:p}, buffer_size: {}, buffer: {:p}, context: {:p}, thread id: {:?}",
            &self.local_block,
            self.user_block,
            self.provider_data.async_,
            self.provider_data.buffer_size,
            self.provider_data.buffer,
            self.provider_data.context,
            std::thread::current().id()
        );
        let hr = unsafe { (self.provider)(XAsyncOp::Cleanup, &self.provider_data) };
        println!(
            "XAsyncState::drop: provider cleanup called with hr: {:?}, thread id: {:?}",
            hr,
            std::thread::current().id()
        );
    }
}

impl IXAsyncState_Impl for XAsyncState_Impl {
    unsafe fn get_local_block(&self) -> *mut XAsyncBlock {
        &self.local_block as *const _ as *mut XAsyncBlock
    }

    unsafe fn get_user_block(&self) -> *mut XAsyncBlock {
        self.user_block
    }

    unsafe fn get_provider_data(&self) -> *mut XAsyncProviderData {
        &self.provider_data as *const _ as *mut XAsyncProviderData
    }

    unsafe fn get_result_size(&self) -> usize {
        self.waiter.0.lock().unwrap().clone()
    }

    unsafe fn get_provider(&self) -> XAsyncProvider {
        self.provider
    }

    unsafe fn set_result_size(&self, required_buffer_size: usize) {
        let mut res_size = self.waiter.0.lock().unwrap();
        *res_size = required_buffer_size;
    }

    unsafe fn get_queue(&self) -> ITaskQueue {
        self.queue.clone()
    }

    unsafe fn notify_all(&self) {
        self.waiter.1.notify_all()
    }

    unsafe fn wait(&self) {
        let waiter = &self.waiter;

        let ublk = self.user_block;

        let lck = waiter.0.lock().unwrap();
        let _lck = waiter
            .1
            .wait_while(lck, |_| unsafe { &*ublk }.get_state().1.is_none())
            .unwrap();
    }
}

#[repr(C)]
struct XAsyncInternal {
    state: AtomicPtr<c_void>,
    magic_result: AtomicU64,
}

// pub unsafe fn x_task_queue_monitor_callback (self: &Self, context: *mut c_void, queue: XTaskQueueHandle, port: XTaskQueuePort);

#[interface("073b7dcb-1fcf-4030-94be-e3c9eb623428")]
pub unsafe trait IXAsync: IUnknown {
    // get status / wait for completion.
    pub unsafe fn x_async_get_status(
        self: &Self,
        async_block: *mut XAsyncBlock,
        wait: bool,
    ) -> HRESULT;
    // Access stored result size, maybe return an error once it is fetched
    pub unsafe fn x_async_get_result_size(
        self: &Self,
        async_block: *mut XAsyncBlock,
        buffer_size: *mut usize,
    ) -> HRESULT;
    // Call cancel of the provider, everything else seems to be up to the receiver
    pub unsafe fn x_async_cancel(self: &Self, async_block: *mut XAsyncBlock) -> HRESULT;
    // Just a wrapper of x_async_begin with only work callback
    pub unsafe fn x_async_run(
        self: &Self,
        async_block: *mut XAsyncBlock,
        work: Option<XAsyncWork>,
    ) -> HRESULT;
    // Calls begin of provider synchronously, may already be completed on return of immediate mode
    pub unsafe fn x_async_begin(
        self: &Self,
        async_block: *mut XAsyncBlock,
        context: *mut c_void,
        identity: *mut c_void,
        identity_name: *const c_char,
        provider: Option<XAsyncProvider>,
    ) -> HRESULT;
    // No clue
    pub unsafe fn ___1(self: &Self);
    // Wrapper of x_task_queue_submit_delayed_callback that invokes provider with DoWork
    pub unsafe fn x_async_schedule(
        self: &Self,
        async_block: *mut XAsyncBlock,
        delay_in_ms: u32,
    ) -> HRESULT;
    // First one wins and set the result code and payload
    pub unsafe fn x_async_complete(
        self: &Self,
        async_block: *mut XAsyncBlock,
        result: HRESULT,
        required_buffer_size: usize,
    );
    // Read the result, after success deallocate the state and return errors
    pub unsafe fn x_async_get_result(
        self: &Self,
        async_block: *mut XAsyncBlock,
        identity: *const c_void,
        buffer_size: usize,
        buffer: *mut c_void,
        buffer_used: *mut usize,
    ) -> HRESULT;
    // create two task queue ports
    pub unsafe fn x_task_queue_create(
        self: &Self,
        work_dispatch_mode: XTaskQueueDispatchMode,
        completion_dispatch_mode: XTaskQueueDispatchMode,
        queue: *mut XTaskQueueHandle,
    ) -> HRESULT;
    // reuses a port of another task_queue, so those needs to be arc handles internally
    pub unsafe fn x_task_queue_create_composite(
        self: &Self,
        work_port: XTaskQueuePortHandle,
        completion_port: XTaskQueuePortHandle,
        queue: *mut XTaskQueueHandle,
    ) -> HRESULT;
    // weak access to queue port, which is owned by this queue
    pub unsafe fn x_task_queue_get_port(
        self: &Self,
        queue: XTaskQueueHandle,
        port: XTaskQueuePort,
        port_handle: *mut XTaskQueuePortHandle,
    ) -> HRESULT;
    pub unsafe fn x_task_queue_duplicate_handle(
        self: &Self,
        queue_handle: XTaskQueueHandle,
        duplicated_handle: *mut XTaskQueueHandle,
    ) -> HRESULT;
    // Manual Queue Dispatching
    pub unsafe fn x_task_queue_dispatch(
        self: &Self,
        queue: XTaskQueueHandle,
        port: XTaskQueuePort,
        timeout_in_ms: u32,
    ) -> BOOL;
    pub unsafe fn x_task_queue_close_handle(self: &Self, queue: XTaskQueueHandle);
    // Submit work to the queue port
    // Notifies callbacks of x_task_queue_register_monitor
    pub unsafe fn x_task_queue_submit_callback(
        self: &Self,
        queue: XTaskQueueHandle,
        port: XTaskQueuePort,
        callback_context: *mut c_void,
        callback: Option<XTaskQueueCallback>,
    ) -> HRESULT;
    pub unsafe fn x_task_queue_submit_delayed_callback(
        self: &Self,
        queue: XTaskQueueHandle,
        port: XTaskQueuePort,
        delay_ms: u32,
        callback_context: *mut c_void,
        callback: Option<XTaskQueueCallback>,
    ) -> HRESULT;
    // Who uses this? win32 only but supported by all gdk platforms
    pub unsafe fn x_task_queue_register_waiter(
        self: &Self,
        queue: XTaskQueueHandle,
        port: XTaskQueuePort,
        wait_handle: HANDLE,
        callback_context: *mut c_void,
        callback: Option<XTaskQueueCallback>,
        token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT;
    pub unsafe fn x_task_queue_unregister_waiter(
        &self,
        queue: XTaskQueueHandle,
        token: XTaskQueueRegistrationToken,
    );
    pub unsafe fn x_task_queue_terminate(
        self: &Self,
        queue: XTaskQueueHandle,
        wait: bool,
        callback_context: *mut c_void,
        callback: Option<XTaskQueueTerminatedCallback>,
    ) -> HRESULT;
    // For manual queue notification for queued items
    pub unsafe fn x_task_queue_register_monitor(
        self: &Self,
        queue: XTaskQueueHandle,
        callback_context: *mut c_void,
        callback: Option<XTaskQueueMonitorCallback>,
        token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT;
    pub unsafe fn x_task_queue_unregister_monitor(
        self: &Self,
        queue: XTaskQueueHandle,
        token: XTaskQueueRegistrationToken,
    );
    // global static reference, e.g. store it in the global com object?
    pub unsafe fn x_task_queue_get_current_process_task_queue(
        self: &Self,
        queue: *mut XTaskQueueHandle,
    ) -> BOOL;
    pub unsafe fn x_task_queue_set_current_process_task_queue(
        self: &mut Self,
        queue: XTaskQueueHandle,
    ) -> HRESULT;
    // The game forbids functions that are slow
    pub unsafe fn x_thread_set_time_sensitive(
        self: &Self,
        is_time_sensitive_thread: bool,
    ) -> HRESULT;
    // Private stuff of the gdk runtime, using thread locals etc.
    // No clue
    pub unsafe fn ___2(self: &Self);
    pub unsafe fn x_thread_assert_not_time_sensitive(self: &Self);
    pub unsafe fn x_thread_is_time_sensitive(self: &Self) -> BOOL;
}

impl XAsyncBlock {
    fn get_internal_raw(&self) -> &mut XAsyncInternal {
        assert!(size_of::<XAsyncInternal>() <= self.internal.len());
        unsafe { &mut *(self.internal.as_ptr() as *mut XAsyncInternal) }
    }
    fn get_state_ex(&self, detach: bool) -> (Option<IXAsyncState>, Option<HRESULT>) {
        println!(
            "XAsyncBlock::get_state_ex called with async_block: {:p}, detach: {}, thread id: {:?}",
            &self,
            detach,
            std::thread::current().id()
        );
        let internal = self.get_internal_raw();
        let magic_result = internal.magic_result.fetch_or(0, Ordering::Acquire);
        if (magic_result >> 32) as u32 != XASYNC_INT_MAGIC {
            println!(
                "XAsyncBlock::get_state_ex: magic_result = {:x}, expected {:x}, thread id: {:?} ",
                magic_result,
                (XASYNC_INT_MAGIC as u64) << 32,
                std::thread::current().id()
            );
            return (None, None);
        }

        let result = HRESULT((magic_result & 0xFFFFFFFF) as i32);
        let result = if result == E_PENDING {
            None
        } else {
            Some(result)
        };

        let state_ptr = if detach {
            internal.state.swap(null_mut(), Ordering::AcqRel)
        } else {
            internal.state.load(Ordering::Acquire)
        };
        if state_ptr.is_null() {
            println!(
                "XAsyncBlock::get_state_ex: state_ptr is null, thread id: {:?}",
                std::thread::current().id()
            );
            return (None, result);
        }

        if detach {
            let state = unsafe { IXAsyncState::from_raw(state_ptr) };

            let provider_block = unsafe { &*state.get_local_block() };
            let user_block = unsafe { &*state.get_user_block() };
            let mut state_ptr_2 = provider_block
                .get_internal_raw()
                .state
                .swap(null_mut(), Ordering::AcqRel);
            if state_ptr_2.is_null() {
                state_ptr_2 = user_block
                    .get_internal_raw()
                    .state
                    .swap(null_mut(), Ordering::AcqRel);
            }
            if state_ptr_2 != state_ptr {
                println!(
                    "XAsyncBlock::get_state_ex: state_ptr_2 != state_ptr, {:p} != {:p}, thread id: {:?}",
                    state_ptr_2,
                    state_ptr,
                    std::thread::current().id()
                );
            }
            assert!(state_ptr_2 == state_ptr);
            mem::drop(unsafe { IXAsyncState::from_raw(state_ptr_2) });
            println!(
                "XAsyncBlock::get_state_ex: detached state_ptr = {:p}, thread id: {:?}",
                state_ptr,
                std::thread::current().id()
            );
            (Some(state), result)
        } else {
            let state = unsafe { IXAsyncState::from_raw_borrowed(&state_ptr) };

            println!(
                "XAsyncBlock::get_state_ex: borrowed state_ptr = {:p}, thread id: {:?}",
                state_ptr,
                std::thread::current().id()
            );
            (state.map(|s| s.clone()), result)
        }
    }
    fn get_state(&self) -> (Option<IXAsyncState>, Option<HRESULT>) {
        self.get_state_ex(false)
    }

    fn create_state(
        &self,
        static_: InterfaceRef<'_, IXAsync>,
        context: *mut c_void,
        provider: XAsyncProvider,
    ) -> Option<IXAsyncState> {
        println!(
            "XAsyncBlock::create_state: creating state for async_block {:p}, context: {:p}, provider: {:p}, thread id: {:?}",
            self,
            context,
            provider,
            std::thread::current().id()
        );
        let internal = self.get_internal_raw();
        let mut magic_result = internal.magic_result.fetch_or(0, Ordering::Acquire);
        if (magic_result >> 32) as u32 == XASYNC_INT_MAGIC {
            return None;
        }
        loop {
            if (magic_result >> 32) as u32 != XASYNC_INITIALIZE_MAGIC {
                break;
            }
            magic_result = internal.magic_result.fetch_or(0, Ordering::Acquire);
        }

        if (magic_result >> 32) as u32 == XASYNC_INT_MAGIC {
            return None;
        }

        if internal
            .magic_result
            .compare_exchange(
                magic_result,
                (XASYNC_INITIALIZE_MAGIC as u64) << 32,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_err()
        {
            println!("failed?");
            return None;
        }

        println!(
            "success, creating state for async_block {:p}, context: {:p}, provider: {:p}, thread id: {:?}",
            self,
            context,
            provider,
            std::thread::current().id()
        );
        let queue = if self.queue.is_null() {
            let mut queue: XTaskQueueHandle = std::ptr::null_mut();
            let _ = unsafe { static_.x_task_queue_get_current_process_task_queue(&mut queue) };
            assert_ne!(queue, null_mut());
            unsafe { ITaskQueue::from_raw(queue) }
        } else {
            unsafe { ITaskQueue::from_raw_borrowed(&self.queue).unwrap().clone() }
        };
        let rawq = queue.as_raw();
        let state: IXAsyncState = XAsyncState {
            local_block: *self,
            user_block: self as *const _ as *mut XAsyncBlock,
            provider_data: XAsyncProviderData {
                async_: null_mut() as *mut XAsyncBlock,
                buffer_size: 0,
                buffer: null_mut(),
                context: context,
            },
            provider: provider,
            waiter: Arc::new((Mutex::new(0), Condvar::new())),
            queue: queue,
        }
        .into();

        internal
            .state
            .store(state.clone().into_raw(), Ordering::Release);

        let local_block = unsafe { &mut *state.get_local_block() };
        // libhttpclient demands this do to using the queue
        local_block.queue = rawq;

        let local_internal = local_block.get_internal_raw();
        local_internal.magic_result.store(
            (XASYNC_INT_MAGIC as u64) << 32 | (E_PENDING.0 as u64 & 0xFFFFFFFF),
            Ordering::Release,
        );
        local_internal
            .state
            .store(state.clone().into_raw(), Ordering::Release);

        let provider_data = unsafe { &mut *state.get_provider_data() };
        provider_data.async_ = unsafe { state.get_local_block() };
        // signal no garbage bytes are stored
        internal.magic_result.store(
            (XASYNC_INT_MAGIC as u64) << 32 | (E_PENDING.0 as u64 & 0xFFFFFFFF),
            Ordering::Release,
        );

        Some(state)
    }
}

unsafe extern "system" fn wait_callback(
    _instance: PTP_CALLBACK_INSTANCE,
    context: *mut c_void,
    wait: PTP_WAIT,
    result: TP_WAIT_RESULT,
) {
    if result.0 != WAIT_OBJECT_0 || context.is_null() {
        return;
    }

    let wctx = unsafe { &*(context.cast::<WaitCallbackContext>()) };
    // register the wait again, because it is a one-shot callback
    unsafe { SetThreadpoolWait(wait, Some(wctx.wait_handle), None) };

    // TODO: what to do with the result
    let _hr = unsafe {
        wctx.port
            .submit_callback(&wctx.tracker, wctx.context as *mut c_void, wctx.callback)
    };
}

std::thread_local! {
    static IS_TIME_SENSITIVE: Cell<bool> = Cell::new(false);
}

#[implement(IXAsync)]
pub struct XAsync {
    pub process_queue: Mutex<XTaskQueueHandle>,
    pub runtime: tokio::runtime::Runtime,
}

#[interface("4445B64E-24E7-45DB-98FB-27BD64E66612")]
unsafe trait ITaskQueue: IUnknown {
    unsafe fn get_handle(&self) -> XTaskQueueHandle;
    unsafe fn submit_delayed_callback(
        &self,
        port: XTaskQueuePort,
        delay_ms: u32,
        callback_context: *mut c_void,
        callback: Option<XTaskQueueCallback>,
    ) -> HRESULT;
    unsafe fn dispatch(&self, port: XTaskQueuePort, timeout_in_ms: u32) -> BOOL;
    unsafe fn get_port_handle(&self, port: XTaskQueuePort) -> XTaskQueuePortHandle;
    unsafe fn terminate(
        &self,
        wait: bool,
        callback_context: *mut c_void,
        callback: Option<XTaskQueueTerminatedCallback>,
    );
    unsafe fn register_monitor(
        &self,
        callback_context: *mut c_void,
        callback: Option<XTaskQueueMonitorCallback>,
        token: *mut XTaskQueueRegistrationToken,
    );
    unsafe fn unregister_monitor(&self, token: XTaskQueueRegistrationToken);
    unsafe fn run_monitor(&self, port: XTaskQueuePort);
    unsafe fn register_waiter(
        &self,
        port: XTaskQueuePort,
        wait_handle: HANDLE,
        callback_context: *mut c_void,
        callback: Option<XTaskQueueCallback>,
        token: *mut XTaskQueueRegistrationToken,
    );
    unsafe fn unregister_waiter(&self, token: XTaskQueueRegistrationToken);
}

#[interface("EE5DCC10-6B84-4058-8349-5AA5195FC0F0")]
unsafe trait ITaskQueuePort: IUnknown {
    unsafe fn get_handle(&self) -> XTaskQueuePortHandle;
    unsafe fn submit_callback(
        &self,
        tracker: *const tokio_util::task::TaskTracker,
        callback_context: *mut c_void,
        callback: Option<XTaskQueueCallback>,
    ) -> HRESULT;
    unsafe fn attach(&self, queue: ITaskQueue, port: XTaskQueuePort);
    unsafe fn detach(&self, queue: ITaskQueue);
    unsafe fn dispatch(&self, timeout_in_ms: u32) -> BOOL;
}

struct AtomicVec<T: Sized + Clone> {
    lock: Mutex<()>,
    buffers: [Vec<T>; 2],
    index_and_ref: AtomicU32,
}

impl<T: Sized + Clone> AtomicVec<T> {
    fn add(&mut self, value: T) -> Result<(), HRESULT> {
        let lock = self.lock.lock().map_err(|_| E_FAIL)?;
        let buffer_read_idx = if (self.index_and_ref.load(Ordering::Acquire) & 0x80000000) != 0 {
            1
        } else {
            0
        };
        let buffer_write_idx = 1 - buffer_read_idx;

        self.buffers[buffer_write_idx] = self.buffers[buffer_read_idx].iter().cloned().collect();
        self.buffers[buffer_write_idx].push(value);

        let expected = (buffer_read_idx << 31) as u32;
        let desired = (buffer_write_idx << 31) as u32;

        loop {
            if self
                .index_and_ref
                .compare_exchange_weak(expected, desired, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
            spin_loop();
        }

        self.buffers[buffer_read_idx].clear();
        self.buffers[buffer_read_idx].shrink_to_fit();

        mem::drop(lock);
        Ok(())
    }

    fn remove<F: Fn(&T) -> bool>(&mut self, predicate: F) -> Result<(), HRESULT> {
        let lock = self.lock.lock().map_err(|_| E_FAIL)?;
        let buffer_read_idx = if (self.index_and_ref.load(Ordering::Acquire) & 0x80000000) != 0 {
            1
        } else {
            0
        };
        let buffer_write_idx = 1 - buffer_read_idx;

        self.buffers[buffer_write_idx] = self.buffers[buffer_read_idx]
            .iter()
            .filter(|e| predicate(e))
            .cloned()
            .collect();

        let expected = (buffer_read_idx << 31) as u32;
        let desired = (buffer_write_idx << 31) as u32;

        loop {
            if self
                .index_and_ref
                .compare_exchange_weak(expected, desired, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
            spin_loop();
        }

        self.buffers[buffer_read_idx].clear();
        self.buffers[buffer_read_idx].shrink_to_fit();

        mem::drop(lock);
        Ok(())
    }

    fn visit<F: Fn(&T)>(&self, callback: F) {
        let index_and_ref = self.index_and_ref.fetch_add(1, Ordering::AcqRel) + 1;
        let buffer_idx = if (index_and_ref & 0x80000000) != 0 {
            1
        } else {
            0
        };
        for it in &self.buffers[buffer_idx] {
            callback(it);
        }
        self.index_and_ref.fetch_sub(1, Ordering::AcqRel);
    }

    fn new() -> Self {
        Self {
            lock: Mutex::new(()),
            buffers: [Vec::new(), Vec::new()],
            index_and_ref: AtomicU32::new(0),
        }
    }
}

#[derive(Clone)]
struct MonitorEntry {
    queue: ITaskQueue,
    port: XTaskQueuePort,
}

#[implement(ITaskQueuePort)]
struct TaskQueuePort {
    runtime: tokio::runtime::Runtime,
    monitors: Cell<AtomicVec<MonitorEntry>>,
}

impl TaskQueuePort {
    fn new_thread_pool() -> io::Result<ITaskQueuePort> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        Ok(Self {
            runtime,
            monitors: Cell::new(AtomicVec::new()),
        }
        .into())
    }
    fn new_serialized_thread_pool() -> io::Result<ITaskQueuePort> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()?;
        Ok(Self {
            runtime,
            monitors: Cell::new(AtomicVec::new()),
        }
        .into())
    }
}

impl ITaskQueuePort_Impl for TaskQueuePort_Impl {
    unsafe fn get_handle(&self) -> XTaskQueuePortHandle {
        let unk: InterfaceRef<ITaskQueuePort> = self.as_interface_ref();
        unk.as_raw()
    }

    unsafe fn submit_callback(
        &self,
        tracker: *const tokio_util::task::TaskTracker,
        callback_context: *mut c_void,
        callback: Option<XTaskQueueCallback>,
    ) -> HRESULT {
        unsafe { &*self.monitors.as_ptr() }.visit(|m| unsafe { m.queue.run_monitor(m.port) });
        let tracker = unsafe { &*tracker }.clone();
        let ctx = callback_context as u64;
        println!(
            "TaskQueuePort::submit_callback called with callback_context: {:p}, tracker.is_closed(): {}",
            callback_context,
            tracker.is_closed()
        );
        tracker.clone().spawn_on(
            async move {
                if let Some(cb) = callback.as_ref() {
                    unsafe { cb(ctx as *mut c_void, tracker.is_closed()) };
                }
            },
            self.runtime.handle(),
        );
        S_OK
    }

    unsafe fn dispatch(&self, _timeout_in_ms: u32) -> BOOL {
        println!("TaskQueuePort::dispatch called, but not implemented for thread pool ports");
        BOOL(0)
    }

    unsafe fn attach(&self, queue: ITaskQueue, port: XTaskQueuePort) {
        unsafe { &mut *self.monitors.as_ptr() }
            .add(MonitorEntry { queue, port })
            .unwrap()
    }

    unsafe fn detach(&self, queue: ITaskQueue) {
        unsafe { &mut *self.monitors.as_ptr() }
            .remove(|m| m.queue.as_raw() == queue.as_raw())
            .unwrap()
    }
}

#[implement(ITaskQueuePort)]
struct ImmediateTaskQueuePort {
    monitors: Cell<AtomicVec<MonitorEntry>>,
}

impl ImmediateTaskQueuePort {
    fn new() -> ITaskQueuePort {
        Self {
            monitors: Cell::new(AtomicVec::new()),
        }
        .into()
    }
}

impl ITaskQueuePort_Impl for ImmediateTaskQueuePort_Impl {
    unsafe fn get_handle(&self) -> XTaskQueuePortHandle {
        let unk: InterfaceRef<ITaskQueuePort> = self.as_interface_ref();
        unk.as_raw()
    }

    unsafe fn submit_callback(
        &self,
        tracker: *const tokio_util::task::TaskTracker,
        callback_context: *mut c_void,
        callback: Option<XTaskQueueCallback>,
    ) -> HRESULT {
        unsafe { &*self.monitors.as_ptr() }.visit(|m| unsafe { m.queue.run_monitor(m.port) });
        let tracker = unsafe { &*tracker }.clone();
        let token = tracker.token();
        println!(
            "ImmediateTaskQueuePort::submit_callback called with callback_context: {:p}, tracker.is_closed(): {}",
            callback_context,
            tracker.is_closed()
        );
        callback.map(|f| unsafe { f(callback_context, tracker.is_closed()) });
        mem::drop(token);
        S_OK
    }

    unsafe fn dispatch(&self, _timeout_in_ms: u32) -> BOOL {
        println!(
            "ImmediateTaskQueuePort::dispatch called, but not implemented for immediate ports"
        );
        BOOL(0)
    }

    unsafe fn attach(&self, queue: ITaskQueue, port: XTaskQueuePort) {
        unsafe { &mut *self.monitors.as_ptr() }
            .add(MonitorEntry { queue, port })
            .unwrap()
    }

    unsafe fn detach(&self, queue: ITaskQueue) {
        unsafe { &mut *self.monitors.as_ptr() }
            .remove(|m| m.queue.as_raw() == queue.as_raw())
            .unwrap()
    }
}

struct QueueEntry {
    token: tokio_util::task::task_tracker::TaskTrackerToken,
    callback: Option<XTaskQueueCallback>,
    context: u64,
}

#[implement(ITaskQueuePort)]
struct ManualTaskQueuePort {
    tx: std::sync::mpsc::Sender<QueueEntry>,
    rx: std::sync::mpsc::Receiver<QueueEntry>,
    monitors: Cell<AtomicVec<MonitorEntry>>,
}

impl ManualTaskQueuePort {
    fn new() -> ITaskQueuePort {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            tx,
            rx,
            monitors: Cell::new(AtomicVec::new()),
        }
        .into()
    }
}

impl ITaskQueuePort_Impl for ManualTaskQueuePort_Impl {
    unsafe fn get_handle(&self) -> XTaskQueuePortHandle {
        let unk: InterfaceRef<ITaskQueuePort> = self.as_interface_ref();
        unk.as_raw()
    }

    unsafe fn submit_callback(
        &self,
        tracker: *const tokio_util::task::TaskTracker,
        callback_context: *mut c_void,
        callback: Option<XTaskQueueCallback>,
    ) -> HRESULT {
        unsafe { &*self.monitors.as_ptr() }.visit(|m| unsafe { m.queue.run_monitor(m.port) });
        let tracker = unsafe { &*tracker }.clone();
        println!(
            "ManualTaskQueuePort::submit_callback called with callback_context: {:p}, tracker.is_closed(): {}, thread id: {:?}, handle: {:x}",
            callback_context,
            tracker.is_closed(),
            std::thread::current().id(),
            unsafe { self.get_handle() } as u64,
        );
        // Call monitior somewhere before or after this
        self.tx
            .send(QueueEntry {
                callback,
                context: callback_context as u64,
                token: tracker.token(),
            })
            .map_err(|_| E_FAIL)
            .unwrap();
        S_OK
    }

    unsafe fn dispatch(&self, timeout_in_ms: u32) -> BOOL {
        println!(
            "ManualTaskQueuePort::dispatch called with timeout_in_ms: {} {} {:?}, handle: {:x}",
            timeout_in_ms,
            unsafe { self.get_handle() } as u64,
            std::thread::current().id(),
            unsafe { self.get_handle() } as u64,
        );
        let rv: BOOL = self.rx
            .recv_timeout(std::time::Duration::from_millis(timeout_in_ms as u64))
            .map(|entry| {
                entry.callback.map(|f| unsafe {
                    println!(
                        "ManualTaskQueuePort::dispatch executing callback with context: {:p}, tracker.is_closed(): {}, thread id: {:?}",
                        entry.context as *mut c_void, entry.token.task_tracker().is_closed(),
                        std::thread::current().id(),
                    );
                    f(
                        entry.context as *mut c_void,
                        entry.token.task_tracker().is_closed(),
                    )
                });
            })
            .is_ok().into();
        println!(
            "ManualTaskQueuePort::dispatch returning {} thread id: {:?}, handle: {:x}",
            rv.as_bool(),
            std::thread::current().id(),
            unsafe { self.get_handle() } as u64,
        );
        rv
    }

    unsafe fn attach(&self, queue: ITaskQueue, port: XTaskQueuePort) {
        unsafe { &mut *self.monitors.as_ptr() }
            .add(MonitorEntry { queue, port })
            .unwrap()
    }

    unsafe fn detach(&self, queue: ITaskQueue) {
        unsafe { &mut *self.monitors.as_ptr() }
            .remove(|m| m.queue.as_raw() == queue.as_raw())
            .unwrap()
    }
}

#[implement(ITaskQueue)]
struct TaskQueue {
    work: ITaskQueuePort,
    completion: ITaskQueuePort,
    tracker: tokio_util::task::TaskTracker,
    handle: tokio::runtime::Handle,
    close_token: CancellationToken,
    monitor_handles: Cell<AtomicVec<(XTaskQueueRegistrationToken, XTaskQueueMonitorCallback, u64)>>,
    next_handle: AtomicU64,
    waiter_handles: Arc<
        Mutex<
            Vec<(
                XTaskQueueRegistrationToken,
                *mut winnt::TP_WAIT,
                Box<WaitCallbackContext>,
            )>,
        >,
    >,
    next_waiter_handle: AtomicU64,
}

impl TaskQueue {
    fn new(
        handle: tokio::runtime::Handle,
        work: ITaskQueuePort,
        completion: ITaskQueuePort,
    ) -> ITaskQueue {
        Self {
            work,
            completion,
            tracker: tokio_util::task::TaskTracker::new(),
            handle,
            close_token: CancellationToken::new(),
            monitor_handles: Cell::new(AtomicVec::new()),
            next_handle: AtomicU64::new(0),
            waiter_handles: Arc::new(Mutex::new(Vec::new())),
            next_waiter_handle: AtomicU64::new(0),
        }
        .into()
    }

    fn get_port(&self, port: XTaskQueuePort) -> &ITaskQueuePort {
        match port {
            XTaskQueuePort::Work => &self.work,
            XTaskQueuePort::Completion => &self.completion,
        }
    }
}

struct WaitCallbackContext {
    port: ITaskQueuePort,
    wait_handle: HANDLE,
    callback: Option<XTaskQueueCallback>,
    context: u64,
    tracker: tokio_util::task::TaskTracker,
}

impl ITaskQueue_Impl for TaskQueue_Impl {
    unsafe fn get_handle(&self) -> XTaskQueueHandle {
        let unk: InterfaceRef<ITaskQueue> = self.as_interface_ref();
        unk.as_raw()
    }

    unsafe fn submit_delayed_callback(
        &self,
        port: XTaskQueuePort,
        delay_ms: u32,
        callback_context: *mut c_void,
        callback: Option<XTaskQueueCallback>,
    ) -> HRESULT {
        println!(
            "TaskQueue::submit_delayed_callback called with port: {:?}, delay_ms: {}, callback_context: {:p}, callback: {:?}, thread id: {:?}",
            port,
            delay_ms,
            callback_context,
            callback,
            std::thread::current().id()
        );
        let tracker = self.tracker.clone();
        let oport = self.get_port(port);
        let r = if delay_ms == 0 {
            unsafe { oport.submit_callback(&tracker, callback_context, callback) }
        } else {
            let callback_context = callback_context as u64;
            let oport = oport.clone().into_raw() as u64;
            let cancel_token = self.close_token.clone();
            self.tracker.spawn_on(
                async move {
                    cancel_token
                        .run_until_cancelled(tokio::time::sleep(std::time::Duration::from_millis(
                            delay_ms as u64,
                        )))
                        .await;
                    // TODO what to do with the result?
                    let _hr = unsafe {
                        ITaskQueuePort::from_raw(oport as *mut c_void).submit_callback(
                            &tracker,
                            callback_context as *mut c_void,
                            callback,
                        )
                    };
                },
                &self.handle,
            );
            S_OK
        };
        println!(
            "TaskQueue::submit_delayed_callback returning {:?} for port: {:?}, delay_ms: {}, callback_context: {:p}, callback: {:?} thread id: {:?}",
            r,
            port,
            delay_ms,
            callback_context,
            callback,
            std::thread::current().id()
        );
        r
    }

    unsafe fn dispatch(&self, port: XTaskQueuePort, timeout_in_ms: u32) -> BOOL {
        println!(
            "TaskQueue::dispatch called with port: {:?}, timeout_in_ms: {}",
            port, timeout_in_ms
        );
        unsafe { self.get_port(port).dispatch(timeout_in_ms) }
    }

    unsafe fn get_port_handle(&self, port: XTaskQueuePort) -> XTaskQueuePortHandle {
        unsafe { self.get_port(port).get_handle() }
    }

    unsafe fn terminate(
        &self,
        wait: bool,
        callback_context: *mut c_void,
        callback: Option<XTaskQueueTerminatedCallback>,
    ) {
        println!(
            "TaskQueue::terminate called with wait: {}, callback_context: {:p}, callback: {:?}",
            wait, callback_context, callback
        );
        let callback_context = callback_context as u64;
        let tracker = self.tracker.clone();
        let tq: InterfaceRef<'_, ITaskQueue> = self.as_interface_ref();
        let tq = tq.to_owned().into_raw() as u64;
        let work = self.work.clone().into_raw() as u64;
        let completion = self.work.clone().into_raw() as u64;
        let future = async move {
            tracker.wait().await;
            let tq = unsafe { ITaskQueue::from_raw(tq as *mut c_void) };
            let work = unsafe { ITaskQueuePort::from_raw(work as *mut c_void) };
            let completion = unsafe { ITaskQueuePort::from_raw(completion as *mut c_void) };

            if let Some(cb) = callback.as_ref() {
                unsafe { cb(callback_context as *mut c_void) };
            }
            unsafe { work.detach(tq.clone()) };
            unsafe { completion.detach(tq.clone()) };
        };
        self.tracker.close();
        self.close_token.cancel();
        if wait {
            self.handle.block_on(future);
        } else {
            self.handle.spawn(future);
        }
    }

    unsafe fn register_monitor(
        &self,
        callback_context: *mut c_void,
        callback: Option<XTaskQueueMonitorCallback>,
        token: *mut XTaskQueueRegistrationToken,
    ) {
        let Some(callback) = callback else {
            return;
        };
        let handle = self.next_handle.fetch_add(1, Ordering::SeqCst);
        unsafe { &mut *self.monitor_handles.as_ptr() }
            .add((handle, callback, callback_context as u64))
            .unwrap();
        if !token.is_null() {
            unsafe {
                *token = handle;
            }
        }
    }

    unsafe fn unregister_monitor(&self, token: XTaskQueueRegistrationToken) {
        unsafe { &mut *self.monitor_handles.as_ptr() }
            .remove(|(h, _, _)| *h == token)
            .unwrap();
    }

    unsafe fn register_waiter(
        &self,
        port: XTaskQueuePort,
        wait_handle: HANDLE,
        callback_context: *mut c_void,
        callback: Option<XTaskQueueCallback>,
        token: *mut XTaskQueueRegistrationToken,
    ) {
        let mut waiter_handles = self.waiter_handles.lock().unwrap();

        let oport = self.get_port(port);
        let wctx = Box::new(WaitCallbackContext {
            port: oport.clone(),
            wait_handle,
            callback,
            context: callback_context as u64,
            tracker: self.tracker.clone(),
        });
        let raw = Box::into_raw(wctx);
        let wctx = unsafe { Box::from_raw(raw) };

        let wait =
            unsafe { CreateThreadpoolWait(Some(wait_callback), Some(raw as *mut c_void), None) };
        unsafe { SetThreadpoolWait(wait, Some(wait_handle), None) };
        let handle = self.next_waiter_handle.fetch_add(1, Ordering::SeqCst);
        waiter_handles.push((handle, wait, wctx));
        if !token.is_null() {
            unsafe {
                *token = handle;
            }
        }
    }

    unsafe fn unregister_waiter(&self, token: XTaskQueueRegistrationToken) {
        let mut waiter_handles = self.waiter_handles.lock().unwrap();
        if let Some(pos) = waiter_handles.iter().position(|h| h.0 == token) {
            let (_, waiter, _) = waiter_handles.remove(pos);
            unsafe { SetThreadpoolWait(waiter, None, None) };
            unsafe { WaitForThreadpoolWaitCallbacks(waiter, true) };
            unsafe { CloseThreadpoolWait(waiter) };
        }
    }

    unsafe fn run_monitor(&self, port: XTaskQueuePort) {
        println!(
            "TaskQueue::submit_delayed_callback executing monitor callbacks for port handle: {:x}, thread id: {:?}",
            unsafe { self.get_handle() as u64 },
            std::thread::current().id()
        );
        unsafe { &mut *self.monitor_handles.as_ptr() }.visit(|(_, callback, context)| {
            println!(
                "TaskQueue::submit_delayed_callback executing monitor callback with context: {:p}, port handle: {:x}, thread id: {:?}",
                *context as *mut c_void,
                unsafe { self.get_handle() as u64 },
                std::thread::current().id()
            );
            unsafe { callback(*context as *mut c_void, self.get_handle(), port) };
        });
        println!(
            "TaskQueue::submit_delayed_callback submitting callback, port handle: {:x}, thread id: {:?}",
            unsafe { self.get_handle() as u64 },
            std::thread::current().id()
        );
    }
}

unsafe extern "system" fn x_async_work_callback(context: *mut c_void, cancel: bool) {
    println!(
        "x_async_work_callback called with context: {:?}, cancel: {} {:?}",
        context,
        cancel,
        std::thread::current().id()
    );
    let state = unsafe { IXAsyncState::from_raw(context) };
    let provider_data = unsafe { &*state.get_provider_data() };
    let provider = unsafe { state.get_provider() };
    let hr = unsafe {
        provider(
            if cancel {
                XAsyncOp::Cancel
            } else {
                XAsyncOp::DoWork
            },
            provider_data,
        )
    };
    println!(
        "x_async_work_callback completed with hr: {:?}, context: {:?}, cancel: {} {:?}",
        hr,
        context,
        cancel,
        std::thread::current().id()
    );
    if E_PENDING != hr {
        unsafe {
            xasync::interface()
                .unwrap()
                .x_async_complete(state.get_local_block(), hr, 0)
        };
    }
    mem::drop(state);
}

unsafe extern "system" fn x_async_complete_callback(context: *mut c_void, cancel: bool) {
    println!(
        "x_async_complete_callback called with context: {:?}, cancel: {} {:?}",
        context,
        cancel,
        std::thread::current().id()
    );
    let state = unsafe { IXAsyncState::from_raw(context) };
    let blk = unsafe { &*state.get_local_block() };
    if let Some(blk) = blk.callback {
        unsafe { blk(state.get_user_block()) };
    }
    println!(
        "x_async_complete_callback completed with context: {:?}, cancel: {} {:?}",
        context,
        cancel,
        std::thread::current().id()
    );
    unsafe { state.notify_all() };
    mem::drop(state);
}

struct XsyncContextHelper<T: Sized, F: Fn() -> Result<T, HRESULT>> {
    result: HRESULT,
    canceled: bool,
    payload: Option<T>,
    future: F,
    async_: IXAsync,
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
            let _ = async_context.async_.x_async_schedule(data.async_, 0);
            S_OK
        },
        XAsyncOp::DoWork => {
            match (async_context.future)() {
                Ok(value) => {
                    async_context.result = S_OK;
                    async_context.payload = Some(value);
                }
                Err(hr) => async_context.result = hr,
            };
            unsafe {
                async_context.async_.x_async_complete(
                    data.async_,
                    async_context.result,
                    size_of::<T>(),
                )
            };
            S_OK
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

unsafe fn run_sync<T: Sized, F>(async_: *mut XAsyncBlock, future: F, xasync: IXAsync) -> HRESULT
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
        async_: xasync.clone(),
    }));
    let hr = unsafe {
        xasync.x_async_begin(
            async_,
            async_context.cast(),
            null_mut(),
            c"run_async".as_ptr(),
            Some(run_sync_helper::<T, F>),
        )
    };
    if hr.is_err() {
        unsafe {
            drop(Box::from_raw(async_context));
        }
    }
    hr
}

impl IXAsync_Impl for XAsync_Impl {
    unsafe fn x_async_get_status(&self, async_block: *mut XAsyncBlock, wait: bool) -> HRESULT {
        println!(
            "x_async_get_status called with async_block: {:?}, wait: {}, thread id: {:?}",
            async_block,
            wait,
            std::thread::current().id()
        );
        let blk = unsafe { &mut *async_block };
        match blk.get_state() {
            (Some(state), hr) => {
                // Use state and hr as needed
                if wait {
                    unsafe { state.wait() };

                    println!(
                        "x_async_get_status: wait completed, state: {:?}, hr: {:?}",
                        blk.get_state().0,
                        blk.get_state().1
                    );

                    blk.get_state().1.unwrap()
                } else {
                    println!(
                        "x_async_get_status: non-wait, state: {:?}, hr: {:?}",
                        blk.get_state().0,
                        blk.get_state().1
                    );
                    hr.unwrap_or(E_PENDING)
                }
            }
            (None, Some(hr)) => {
                println!(
                    "x_async_get_status: no state, hr: {:?}, thread id: {:?}",
                    hr,
                    std::thread::current().id()
                );
                hr
            }
            _ => {
                println!(
                    "x_async_get_status: no state and no hr, returning E_FAIL, thread id: {:?}",
                    std::thread::current().id()
                );
                E_FAIL
            }
        }
    }

    unsafe fn x_async_get_result_size(
        &self,
        async_block: *mut XAsyncBlock,
        buffer_size: *mut usize,
    ) -> HRESULT {
        println!(
            "x_async_get_result_size called with async_block: {:?}, buffer_size: {:p}, thread id: {:?}",
            async_block,
            buffer_size,
            std::thread::current().id()
        );
        let blk = unsafe { &mut *async_block };
        let (Some(state), _) = blk.get_state() else {
            return E_FAIL;
        };
        if buffer_size.is_null() {
            return E_FAIL;
        }
        unsafe {
            *buffer_size = state.get_result_size();
        }
        S_OK
    }

    unsafe fn x_async_cancel(&self, async_block: *mut XAsyncBlock) -> HRESULT {
        println!(
            "x_async_cancel called with async_block: {:?}, thread id: {:?}",
            async_block,
            std::thread::current().id()
        );
        let blk = unsafe { &mut *async_block };
        let (Some(state), _) = blk.get_state() else {
            return E_FAIL;
        };
        let provider_data = unsafe { &*state.get_provider_data() };
        let provider = unsafe { state.get_provider() };
        unsafe { provider(XAsyncOp::Cancel, provider_data) }
    }

    unsafe fn x_async_run(
        &self,
        async_block: *mut XAsyncBlock,
        work: Option<XAsyncWork>,
    ) -> HRESULT {
        println!(
            "x_async_run called with async_block: {:?}, work: {:?}, thread id: {:?}",
            async_block,
            work,
            std::thread::current().id()
        );
        let async_inf: InterfaceRef<'_, IXAsync> = self.as_interface_ref();
        unsafe {
            run_sync(
                async_block,
                || {
                    if let Some(work) = work {
                        let hr = work(async_block);
                        if hr.is_err() {
                            return Err(hr);
                        }
                    }
                    Ok(())
                },
                async_inf.to_owned(),
            )
        }
    }

    unsafe fn x_async_begin(
        &self,
        async_block: *mut XAsyncBlock,
        context: *mut c_void,
        _identity: *mut c_void,
        _identity_name: *const c_char,
        provider: Option<XAsyncProvider>,
    ) -> HRESULT {
        println!("x_async_begin prestart {:?}", std::thread::current().id(),);
        let blk = unsafe { &mut *async_block };
        let Some(provider) = provider else {
            return E_FAIL;
        };
        let Some(state) = blk.create_state(self.as_interface_ref(), context, provider) else {
            println!(
                "x_async_begin: failed to create state for async_block: {:?}, context: {:?}, provider: {:?} thread id: {:?}",
                async_block,
                context,
                provider,
                std::thread::current().id(),
            );
            return E_FAIL;
        };
        println!(
            "x_async_begin start with async_block: {:?}, context: {:?}, provider: {:?}, thread id: {:?} {}",
            async_block,
            context,
            provider,
            std::thread::current().id(),
            /* unsafe { ffi::CStr::from_ptr(identity_name) }.to_string_lossy() */ ""
        );

        let provider_data = unsafe { &*state.get_provider_data() };
        println!(
            "x_async_begin: provider_data: async_: {:?}, buffer_size: {}, buffer: {:?}, context: {:?}, thread id: {:?}, queue {}",
            provider_data.async_,
            provider_data.buffer_size,
            provider_data.buffer,
            provider_data.context,
            std::thread::current().id(),
            unsafe { *provider_data.async_ }.queue as u64,
        );
        let hr = unsafe { provider(XAsyncOp::Begin, provider_data) };
        println!(
            "x_async_begin called with async_block: {:?}, context: {:?}, provider: {:?}, hr: {:?}, thread id: {:?} {}",
            async_block,
            context,
            provider,
            hr,
            std::thread::current().id(),
            /* unsafe { ffi::CStr::from_ptr(identity_name) }.to_string_lossy() */ ""
        );
        S_OK
    }

    unsafe fn ___1(&self) {
        todo!()
    }

    unsafe fn x_async_schedule(&self, async_block: *mut XAsyncBlock, delay_in_ms: u32) -> HRESULT {
        println!(
            "x_async_schedule called with async_block: {:?}, delay_in_ms: {}",
            async_block, delay_in_ms
        );
        let blk = unsafe { &mut *async_block };
        let (Some(state), _) = blk.get_state() else {
            return E_FAIL;
        };
        let queue = unsafe { state.get_queue() };
        let context = state.clone().into_raw() as *mut c_void;
        println!("x_async_schedule: context: {:?}", context);
        let _ = unsafe {
            queue.submit_delayed_callback(
                XTaskQueuePort::Work,
                delay_in_ms,
                context,
                Some(x_async_work_callback),
            )
        };
        S_OK
    }

    unsafe fn x_async_complete(
        &self,
        async_block: *mut XAsyncBlock,
        result: HRESULT,
        required_buffer_size: usize,
    ) {
        println!(
            "x_async_complete called with async_block: {:?}, result: {:?}, required_buffer_size: {}, thread id: {:?}",
            async_block,
            result,
            required_buffer_size,
            std::thread::current().id()
        );
        let blk = unsafe { &mut *async_block };
        // required_buffer_size == 0 => cleanup state as no result is expected, otherwise the state is kept until x_async_get_result is called
        let (Some(state), _) = blk.get_state_ex(false) else {
            return;
        };
        if result == E_PENDING {
            return;
        }
        let user_block = unsafe { &*state.get_user_block() };
        if user_block
            .get_internal_raw()
            .magic_result
            .compare_exchange(
                (XASYNC_INT_MAGIC as u64) << 32 | (E_PENDING.0 as u64 & 0xFFFFFFFF),
                (XASYNC_INT_MAGIC as u64) << 32 | (result.0 as u64 & 0xFFFFFFFF),
                Ordering::Release,
                Ordering::Relaxed,
            )
            .is_ok()
        {
            if required_buffer_size == 0 {
                let _ = blk.get_state_ex(true);
            }
            // TODO bug we should wait for complete to run
            unsafe { state.set_result_size(required_buffer_size) };
            let queue = unsafe { state.get_queue() };
            let _ = unsafe {
                queue.submit_delayed_callback(
                    XTaskQueuePort::Completion,
                    0,
                    state.clone().into_raw() as *mut c_void,
                    Some(x_async_complete_callback),
                )
            };
            println!(
                "x_async_complete: submitted completion callback for async_block: {:?}, result: {:?}, required_buffer_size: {}, thread id: {:?}",
                async_block,
                result,
                required_buffer_size,
                std::thread::current().id()
            );
        }
    }

    unsafe fn x_async_get_result(
        &self,
        async_block: *mut XAsyncBlock,
        identity: *const c_void,
        buffer_size: usize,
        buffer: *mut c_void,
        buffer_used: *mut usize,
    ) -> HRESULT {
        println!(
            "x_async_get_result called with async_block: {:?}, identity: {:?}, buffer_size: {}, buffer: {:?}, buffer_used: {:p}, thread id: {:?}",
            async_block,
            identity,
            buffer_size,
            buffer,
            buffer_used,
            std::thread::current().id()
        );
        let blk = unsafe { &mut *async_block };
        let (Some(state), Some(hr)) = blk.get_state_ex(true) else {
            return E_FAIL;
        };
        if hr != S_OK {
            return hr;
        }
        let provider_data = unsafe { &mut *state.get_provider_data() };
        let provider = unsafe { state.get_provider() };
        provider_data.buffer = buffer;
        provider_data.buffer_size = buffer_size;
        let hr = unsafe { provider(XAsyncOp::GetResult, provider_data) };
        println!(
            "x_async_get_result called with async_block: {:?}, identity: {:?}, buffer_size: {}, buffer: {:?}, buffer_used: {:p}, hr: {:?}, thread id: {:?}",
            async_block,
            identity,
            buffer_size,
            buffer,
            buffer_used,
            hr,
            std::thread::current().id()
        );
        hr
    }

    unsafe fn x_task_queue_create(
        &self,
        work_dispatch_mode: XTaskQueueDispatchMode,
        completion_dispatch_mode: XTaskQueueDispatchMode,
        queue: *mut XTaskQueueHandle,
    ) -> HRESULT {
        println!(
            "x_task_queue_create called with work_dispatch_mode: {:?}, completion_dispatch_mode: {:?}",
            work_dispatch_mode, completion_dispatch_mode
        );
        let work: ITaskQueuePort = match work_dispatch_mode {
            XTaskQueueDispatchMode::Manual => ManualTaskQueuePort::new(),
            XTaskQueueDispatchMode::ThreadPool => TaskQueuePort::new_thread_pool().unwrap(),
            XTaskQueueDispatchMode::SerializedThreadPool => {
                TaskQueuePort::new_serialized_thread_pool().unwrap()
            }
            XTaskQueueDispatchMode::Immediate => ImmediateTaskQueuePort::new(),
            _ => {
                todo!(
                    "x_task_queue_create: unsupported completion_dispatch_mode: {:?}",
                    completion_dispatch_mode
                );
            }
        };
        let completion: ITaskQueuePort = match completion_dispatch_mode {
            XTaskQueueDispatchMode::Manual => ManualTaskQueuePort::new(),
            XTaskQueueDispatchMode::ThreadPool => TaskQueuePort::new_thread_pool().unwrap(),
            XTaskQueueDispatchMode::SerializedThreadPool => {
                TaskQueuePort::new_serialized_thread_pool().unwrap()
            }
            XTaskQueueDispatchMode::Immediate => ImmediateTaskQueuePort::new(),
            _ => {
                todo!(
                    "x_task_queue_create: unsupported completion_dispatch_mode: {:?}",
                    completion_dispatch_mode
                );
            }
        };
        let task_queue: ITaskQueue = TaskQueue::new(
            self.runtime.handle().clone(),
            work.clone(),
            completion.clone(),
        );
        unsafe { work.attach(task_queue.clone(), XTaskQueuePort::Work) };
        unsafe { completion.attach(task_queue.clone(), XTaskQueuePort::Completion) };

        unsafe {
            *queue = task_queue.get_handle();
        }
        mem::forget(task_queue);
        S_OK
    }

    unsafe fn x_task_queue_create_composite(
        &self,
        work_port: XTaskQueuePortHandle,
        completion_port: XTaskQueuePortHandle,
        queue: *mut XTaskQueueHandle,
    ) -> HRESULT {
        println!(
            "x_task_queue_create_composite called with work_port: {:?}, completion_port: {:?}, thread id: {:?}",
            work_port,
            completion_port,
            std::thread::current().id()
        );
        let work = unsafe { ITaskQueuePort::from_raw_borrowed(&work_port) };
        let completion = unsafe { ITaskQueuePort::from_raw_borrowed(&completion_port) };
        let (Some(work), Some(completion)) = (work, completion) else {
            unsafe {
                *queue = null_mut();
            }
            println!(
                "x_task_queue_create_composite: failed to create composite queue, work: {:?}, completion: {:?}, thread id: {:?}",
                work,
                completion,
                std::thread::current().id()
            );
            return E_FAIL;
        };
        let task_queue: ITaskQueue = TaskQueue::new(
            self.runtime.handle().clone(),
            work.clone(),
            completion.clone(),
        );
        unsafe { work.attach(task_queue.clone(), XTaskQueuePort::Work) };
        unsafe { completion.attach(task_queue.clone(), XTaskQueuePort::Completion) };
        unsafe {
            *queue = task_queue.get_handle();
        }
        mem::forget(task_queue);
        S_OK
    }

    unsafe fn x_task_queue_get_port(
        &self,
        queue: XTaskQueueHandle,
        port: XTaskQueuePort,
        port_handle: *mut XTaskQueuePortHandle,
    ) -> HRESULT {
        println!("x_task_queue_get_port start {:?}", queue);
        let queue = unsafe { ITaskQueue::from_raw_borrowed(&queue) };
        let Some(queue) = queue else {
            unsafe {
                *port_handle = null_mut();
            }
            println!("x_task_queue_get_port fail");
            return E_FAIL;
        };

        unsafe {
            *port_handle = queue.get_port_handle(port);
        }
        println!(
            "x_task_queue_get_port called with queue: {:?}, port: {:?}, port_handle: {:?}, thread id: {:?}",
            unsafe { queue.get_handle() },
            port,
            unsafe { *port_handle },
            std::thread::current().id()
        );
        S_OK
    }

    unsafe fn x_task_queue_duplicate_handle(
        &self,
        queue_handle: XTaskQueueHandle,
        duplicated_handle: *mut XTaskQueueHandle,
    ) -> HRESULT {
        let handle = unsafe { ITaskQueue::from_raw_borrowed(&queue_handle) };
        let Some(handle) = handle.map(|f| {
            let clone = f.clone();
            mem::forget(clone);
            unsafe { f.get_handle() }
        }) else {
            return E_FAIL;
        };
        unsafe { *duplicated_handle = handle };
        S_OK
    }

    unsafe fn x_task_queue_dispatch(
        &self,
        queue: XTaskQueueHandle,
        port: XTaskQueuePort,
        timeout_in_ms: u32,
    ) -> BOOL {
        println!(
            "x_task_queue_dispatch called with queue: {:?}, port: {:?}, timeout_in_ms: {}, thread id: {:?}",
            queue,
            port,
            timeout_in_ms,
            std::thread::current().id()
        );
        let handle = unsafe { ITaskQueue::from_raw_borrowed(&queue) };
        let ret = handle
            .map(|f| unsafe { f.dispatch(port, timeout_in_ms) })
            .unwrap();
        println!(
            "x_task_queue_dispatch completed with queue: {:?}, port: {:?}, timeout_in_ms: {}, ret: {:?}, thread id: {:?}",
            queue,
            port,
            timeout_in_ms,
            ret,
            std::thread::current().id()
        );
        ret.into()
    }

    unsafe fn x_task_queue_close_handle(&self, queue: XTaskQueueHandle) {
        unsafe {
            ITaskQueue::from_raw(queue);
        }
    }

    unsafe fn x_task_queue_submit_callback(
        &self,
        queue: XTaskQueueHandle,
        port: XTaskQueuePort,
        callback_context: *mut c_void,
        callback: Option<XTaskQueueCallback>,
    ) -> HRESULT {
        unsafe {
            self.x_task_queue_submit_delayed_callback(queue, port, 0, callback_context, callback)
        }
    }

    unsafe fn x_task_queue_submit_delayed_callback(
        &self,
        queue: XTaskQueueHandle,
        port: XTaskQueuePort,
        delay_ms: u32,
        callback_context: *mut c_void,
        callback: Option<XTaskQueueCallback>,
    ) -> HRESULT {
        println!(
            "x_task_queue_submit_delayed_callback called with queue: {:?}, port: {:?}, delay_ms: {}, callback_context: {:p}, callback: {:?}, thread id: {:?}",
            queue,
            port,
            delay_ms,
            callback_context,
            callback,
            std::thread::current().id()
        );
        let handle = unsafe { ITaskQueue::from_raw_borrowed(&queue) };
        handle.map(|f| unsafe {
            f.submit_delayed_callback(port, delay_ms, callback_context, callback)
        });
        println!(
            "x_task_queue_submit_delayed_callback completed with queue: {:?}, port: {:?}, delay_ms: {}, callback_context: {:p}, callback: {:?}, thread id: {:?}",
            queue,
            port,
            delay_ms,
            callback_context,
            callback,
            std::thread::current().id()
        );
        S_OK
    }

    unsafe fn x_task_queue_register_waiter(
        &self,
        queue: XTaskQueueHandle,
        port: XTaskQueuePort,
        wait_handle: HANDLE,
        callback_context: *mut c_void,
        callback: Option<XTaskQueueCallback>,
        token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT {
        println!(
            "x_task_queue_register_waiter called with queue: {:?}, port: {:?}, wait_handle: {:?}, callback_context: {:p}, callback: {:?}",
            queue, port, wait_handle, callback_context, callback
        );
        let handle = unsafe { ITaskQueue::from_raw_borrowed(&queue) };
        handle.map(|f| unsafe {
            f.register_waiter(port, wait_handle, callback_context, callback, token)
        });
        S_OK
    }

    unsafe fn x_task_queue_unregister_waiter(
        &self,
        queue: XTaskQueueHandle,
        token: XTaskQueueRegistrationToken,
    ) {
        println!(
            "x_task_queue_unregister_waiter called with queue: {:?}, token: {:?}",
            queue, token
        );
        let handle = unsafe { ITaskQueue::from_raw_borrowed(&queue) };
        handle.map(|f| unsafe { f.unregister_waiter(token) });
    }

    unsafe fn x_task_queue_terminate(
        &self,
        queue: XTaskQueueHandle,
        wait: bool,
        callback_context: *mut c_void,
        callback: Option<XTaskQueueTerminatedCallback>,
    ) -> HRESULT {
        println!(
            "x_task_queue_terminate called with queue: {:?}, wait: {}, callback_context: {:p}, callback: {:?}",
            queue, wait, callback_context, callback
        );
        let handle = unsafe { ITaskQueue::from_raw_borrowed(&queue) };
        handle.map(|f| unsafe { f.terminate(wait, callback_context, callback) });
        S_OK
    }

    unsafe fn x_task_queue_register_monitor(
        &self,
        queue: XTaskQueueHandle,
        callback_context: *mut c_void,
        callback: Option<XTaskQueueMonitorCallback>,
        token: *mut XTaskQueueRegistrationToken,
    ) -> HRESULT {
        println!(
            "x_task_queue_register_monitor called with queue: {:?}, callback_context: {:p}, callback: {:?}",
            queue, callback_context, callback
        );
        let handle = unsafe { ITaskQueue::from_raw_borrowed(&queue) };
        handle.map(|f| unsafe { f.register_monitor(callback_context, callback, token) });
        S_OK
    }

    unsafe fn x_task_queue_unregister_monitor(
        &self,
        queue: XTaskQueueHandle,
        token: XTaskQueueRegistrationToken,
    ) {
        println!(
            "x_task_queue_unregister_monitor called with queue: {:?}, token: {:?}",
            queue, token
        );
        let handle = unsafe { ITaskQueue::from_raw_borrowed(&queue) };
        handle.map(|f| unsafe { f.unregister_monitor(token) });
    }

    unsafe fn x_task_queue_get_current_process_task_queue(
        &self,
        queue: *mut XTaskQueueHandle,
    ) -> BOOL {
        let a = self.process_queue.lock();
        if a.is_err() {
            return BOOL(0);
        }
        // TODO
        let _hr = unsafe { self.x_task_queue_duplicate_handle(*a.unwrap(), queue) };
        BOOL(1)
    }

    unsafe fn x_task_queue_set_current_process_task_queue(
        &self,
        queue: XTaskQueueHandle,
    ) -> HRESULT {
        let mut lck = self.process_queue.lock().unwrap();
        println!("x_task_queue_set_current_process_task_queue {:?}", queue);
        *lck = queue;
        S_OK
    }

    unsafe fn x_thread_set_time_sensitive(&self, is_time_sensitive_thread: bool) -> HRESULT {
        IS_TIME_SENSITIVE.with(|is_time_sensitive| {
            is_time_sensitive.set(is_time_sensitive_thread);
        });
        S_OK
    }

    unsafe fn x_thread_assert_not_time_sensitive(&self) {
        assert!(!IS_TIME_SENSITIVE.with(|is_time_sensitive| is_time_sensitive.get()));
    }

    unsafe fn x_thread_is_time_sensitive(&self) -> BOOL {
        IS_TIME_SENSITIVE
            .with(|is_time_sensitive| is_time_sensitive.get())
            .into()
    }

    unsafe fn ___2(&self) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use std::{ffi::c_void, ptr::null_mut, sync::Mutex, time::Duration};

    use windows::synchapi::{CreateEventW, SetEvent};
    use windows_core::{HRESULT, Interface};

    use crate::{
        E_FAIL, S_OK,
        threading::{
            ITaskQueuePort, TaskQueuePort, XAsync, XTaskQueueDispatchMode, XTaskQueueHandle,
            XTaskQueuePort, XTaskQueuePortHandle, XTaskQueueRegistrationToken,
        },
        xasync::{self, IXAsync, XAsyncBlock},
    };

    unsafe extern "system" fn callback(ctx: *mut c_void, cancel: bool) {
        println!(
            "Callback called with context: {:?}, cancel: {} {:?}",
            ctx,
            cancel,
            std::thread::current().id()
        );
    }

    #[test]
    fn test_x_async() {
        let xasync: IXAsync = XAsync {
            process_queue: Mutex::new(null_mut()),
            runtime: tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap(),
        }
        .into();
        let mut queue: XTaskQueueHandle = null_mut();
        let mut queue2: XTaskQueueHandle = null_mut();
        let mut port_handle: XTaskQueuePortHandle = null_mut();
        unsafe {
            println!("Creating task queue... {:?}", std::thread::current().id());
            let _ = xasync.x_task_queue_create(
                XTaskQueueDispatchMode::SerializedThreadPool,
                XTaskQueueDispatchMode::Immediate,
                &mut queue,
            );
            println!("Submitting callback to task queue... {}", queue as usize);
            let _ = xasync.x_task_queue_submit_delayed_callback(
                queue,
                XTaskQueuePort::Work,
                0,
                null_mut(),
                Some(callback),
            );
            println!("Submitting delayed callback to task queue...");
            let _ = xasync.x_task_queue_submit_callback(
                queue,
                XTaskQueuePort::Work,
                null_mut(),
                Some(callback),
            );
            let _ = xasync.x_task_queue_get_port(queue, XTaskQueuePort::Work, &mut port_handle);

            let _ = xasync.x_task_queue_create_composite(port_handle, port_handle, &mut queue2);

            let _ = xasync.x_task_queue_terminate(queue, true, null_mut(), None);

            xasync.x_task_queue_close_handle(queue);

            let _ = xasync.x_task_queue_submit_delayed_callback(
                queue2,
                XTaskQueuePort::Work,
                0,
                null_mut(),
                Some(callback),
            );
            let _ = xasync.x_task_queue_submit_delayed_callback(
                queue2,
                XTaskQueuePort::Work,
                1000,
                null_mut(),
                Some(callback),
            );

            let _ = xasync.x_task_queue_terminate(queue2, true, null_mut(), None);

            xasync.x_task_queue_close_handle(queue2);

            // std::thread::sleep(std::time::Duration::from_millis(100));
            println!("Test completed.");
        };
    }

    #[test]
    fn test_x_async2() {
        let port = TaskQueuePort::new_thread_pool().unwrap();

        let handle = unsafe { port.get_handle() };
        let obj = unsafe { ITaskQueuePort::from_raw_borrowed(&handle) };
        let nh = unsafe { obj.unwrap().get_handle() };
        assert_ne!(nh, null_mut());
    }

    #[test]
    fn test_x_async3() {
        let mut async_ = XAsyncBlock {
            callback: None,
            context: null_mut(),
            queue: null_mut(),
            internal: [0; 32],
        };
        let hr = unsafe {
            xasync::run(&mut async_, async {
                return Ok(());
            })
        };

        println!("run returned: {:?}", hr);

        let hr = unsafe { xasync::get_status(&mut async_, true) };
        println!("get_status returned: {:?}", hr);
    }

    #[test]
    fn test_x_async4() {
        let mut async_ = XAsyncBlock {
            callback: None,
            context: null_mut(),
            queue: null_mut(),
            internal: [0; 32],
        };
        let hr = unsafe {
            xasync::run(&mut async_, async {
                return Ok(());
            })
        };

        println!("run returned: {:?}", hr);

        let hr = unsafe { xasync::get_status(&mut async_, true) };
        println!("get_status returned: {:?}", hr);

        let hr = unsafe {
            xasync::run(&mut async_, async {
                return Ok(());
            })
        };

        println!("run returned: {:?}", hr);

        let hr = unsafe { xasync::get_status(&mut async_, true) };
        println!("get_status returned: {:?}", hr);
    }

    #[test]
    fn test_x_async5() {
        let mut async_ = XAsyncBlock {
            callback: None,
            context: null_mut(),
            queue: null_mut(),
            internal: [0; 32],
        };
        let hr = unsafe {
            xasync::run(&mut async_, async {
                return Err::<(), HRESULT>(E_FAIL);
            })
        };
        assert_eq!(hr, S_OK);

        let hr = unsafe { xasync::get_status(&mut async_, true) }.unwrap_err();
        assert_eq!(hr, E_FAIL);

        let mut out = ();
        let hr = unsafe { xasync::get_result(&mut async_, null_mut(), &mut out) }.unwrap_err();
        assert_eq!(hr, E_FAIL);
    }

    #[test]
    fn test_x_async6() {
        let xasync_ = xasync::interface().unwrap();

        let mut queue = null_mut();
        let _ = unsafe {
            xasync_.x_task_queue_create(
                XTaskQueueDispatchMode::Immediate,
                XTaskQueueDispatchMode::Immediate,
                &mut queue,
            )
        };

        let mut async_ = XAsyncBlock {
            callback: None,
            context: null_mut(),
            queue: queue as *mut c_void,
            internal: [0; 32],
        };
        let hr = unsafe {
            xasync::run(&mut async_, async {
                println!("Running async operation...");
                return Err::<(), HRESULT>(E_FAIL);
            })
        };
        assert_eq!(hr, S_OK);

        let hr = unsafe { xasync::get_status(&mut async_, true) }.unwrap_err();
        assert_eq!(hr, E_FAIL);

        let mut out = ();
        let hr = unsafe { xasync::get_result(&mut async_, null_mut(), &mut out) }.unwrap_err();
        assert_eq!(hr, E_FAIL);
    }

    #[test]
    fn test_x_async7() {
        let xasync_ = xasync::interface().unwrap();

        let mut queue = null_mut();
        let _ = unsafe {
            xasync_.x_task_queue_create(
                XTaskQueueDispatchMode::SerializedThreadPool,
                XTaskQueueDispatchMode::SerializedThreadPool,
                &mut queue,
            )
        };

        let mut async_ = XAsyncBlock {
            callback: None,
            context: null_mut(),
            queue: queue as *mut c_void,
            internal: [0; 32],
        };
        let hr = unsafe {
            xasync::run(&mut async_, async {
                println!("Running async operation...");
                return Err::<(), HRESULT>(E_FAIL);
            })
        };
        assert_eq!(hr, S_OK);

        let hr = unsafe { xasync::get_status(&mut async_, true) }.unwrap_err();
        assert_eq!(hr, E_FAIL);

        let mut out = ();
        let hr = unsafe { xasync::get_result(&mut async_, null_mut(), &mut out) }.unwrap_err();
        assert_eq!(hr, E_FAIL);
    }

    unsafe extern "system" fn cbk(_ctx: *mut c_void, _cancel: bool) {
        println!("cbk");
    }

    #[test]
    fn test_x_async8() {
        let xasync_ = xasync::interface().unwrap();

        let mut queue = null_mut();
        let _ = unsafe {
            xasync_.x_task_queue_create(
                XTaskQueueDispatchMode::SerializedThreadPool,
                XTaskQueueDispatchMode::SerializedThreadPool,
                &mut queue,
            )
        };

        let e = unsafe { CreateEventW(None, false, false, None) };

        let mut tkn: XTaskQueueRegistrationToken = 0;
        let _ = unsafe {
            xasync_.x_task_queue_register_waiter(
                queue,
                XTaskQueuePort::Work,
                e,
                null_mut(),
                Some(cbk),
                &mut tkn,
            )
        };

        let _ = unsafe { SetEvent(e) };

        std::thread::sleep(Duration::new(2, 0));

        unsafe { xasync_.x_task_queue_unregister_waiter(queue, tkn) };

        let _ = unsafe { SetEvent(e) };

        let _ = unsafe { xasync_.x_task_queue_terminate(queue, true, null_mut(), None) };
    }
}
