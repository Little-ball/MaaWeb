//! MaaCore C API FFI bindings (dynamic loading)
//!
//! Bindings are based on the official `AsstCaller.h` header:
//! https://github.com/MaaAssistantArknights/MaaAssistantArknights/blob/dev-v2/include/AsstCaller.h
//!
//! The library is loaded dynamically via `libloading` so that the server can be
//! built without the MaaCore library present; it is only needed at runtime.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::Arc;

// ---- Type aliases matching the C header ----
pub type AsstBool = u8;
pub type AsstSize = u64;
pub type AsstId = i32;
pub type AsstMsgId = AsstId;
pub type AsstTaskId = AsstId;
pub type AsstAsyncCallId = AsstId;
pub type AsstOptionKey = i32;

pub type AsstHandle = *mut c_void;

pub type AsstApiCallback = unsafe extern "C" fn(
    msg: AsstMsgId,
    details_json: *const c_char,
    custom_arg: *mut c_void,
);

// ---- Message IDs from AsstMsg.h ----
pub mod msg {
    use super::AsstMsgId;

    // Global Info
    pub const INTERNAL_ERROR: AsstMsgId = 0;
    pub const INIT_FAILED: AsstMsgId = 1;
    pub const CONNECTION_INFO: AsstMsgId = 2;
    pub const ALL_TASKS_COMPLETED: AsstMsgId = 3;
    pub const ASYNC_CALL_INFO: AsstMsgId = 4;
    pub const DESTROYED: AsstMsgId = 5;

    // TaskChain Info
    pub const TASK_CHAIN_ERROR: AsstMsgId = 10000;
    pub const TASK_CHAIN_START: AsstMsgId = 10001;
    pub const TASK_CHAIN_COMPLETED: AsstMsgId = 10002;
    pub const TASK_CHAIN_EXTRA_INFO: AsstMsgId = 10003;
    pub const TASK_CHAIN_STOPPED: AsstMsgId = 10004;

    // SubTask Info
    pub const SUB_TASK_ERROR: AsstMsgId = 20000;
    pub const SUB_TASK_START: AsstMsgId = 20001;
    pub const SUB_TASK_COMPLETED: AsstMsgId = 20002;
    pub const SUB_TASK_EXTRA_INFO: AsstMsgId = 20003;
    pub const SUB_TASK_STOPPED: AsstMsgId = 20004;

    // Web Request
    pub const REPORT_REQUEST: AsstMsgId = 30000;

    /// Human-readable name for a message id.
    pub fn name(id: AsstMsgId) -> &'static str {
        match id {
            INTERNAL_ERROR => "InternalError",
            INIT_FAILED => "InitFailed",
            CONNECTION_INFO => "ConnectionInfo",
            ALL_TASKS_COMPLETED => "AllTasksCompleted",
            ASYNC_CALL_INFO => "AsyncCallInfo",
            DESTROYED => "Destroyed",
            TASK_CHAIN_ERROR => "TaskChainError",
            TASK_CHAIN_START => "TaskChainStart",
            TASK_CHAIN_COMPLETED => "TaskChainCompleted",
            TASK_CHAIN_EXTRA_INFO => "TaskChainExtraInfo",
            TASK_CHAIN_STOPPED => "TaskChainStopped",
            SUB_TASK_ERROR => "SubTaskError",
            SUB_TASK_START => "SubTaskStart",
            SUB_TASK_COMPLETED => "SubTaskCompleted",
            SUB_TASK_EXTRA_INFO => "SubTaskExtraInfo",
            SUB_TASK_STOPPED => "SubTaskStopped",
            REPORT_REQUEST => "ReportRequest",
            _ => "Unknown",
        }
    }
}

/// A dynamically-loaded MaaCore library.
pub struct MaaCore {
    _lib: &'static Library,
    // Core lifecycle
    pub set_user_dir: Symbol<'static, unsafe extern "C" fn(*const c_char) -> AsstBool>,
    pub load_resource: Symbol<'static, unsafe extern "C" fn(*const c_char) -> AsstBool>,
    pub set_static_option: Symbol<'static, unsafe extern "C" fn(AsstOptionKey, *const c_char) -> AsstBool>,
    pub create: Symbol<'static, unsafe extern "C" fn() -> AsstHandle>,
    pub create_ex: Symbol<'static, unsafe extern "C" fn(AsstApiCallback, *mut c_void) -> AsstHandle>,
    pub destroy: Symbol<'static, unsafe extern "C" fn(AsstHandle)>,
    pub set_instance_option: Symbol<'static, unsafe extern "C" fn(AsstHandle, AsstOptionKey, *const c_char) -> AsstBool>,
    pub connect: Symbol<'static, unsafe extern "C" fn(AsstHandle, *const c_char, *const c_char, *const c_char) -> AsstBool>,
    pub append_task: Symbol<'static, unsafe extern "C" fn(AsstHandle, *const c_char, *const c_char) -> AsstTaskId>,
    pub set_task_params: Symbol<'static, unsafe extern "C" fn(AsstHandle, AsstTaskId, *const c_char) -> AsstBool>,
    pub start: Symbol<'static, unsafe extern "C" fn(AsstHandle) -> AsstBool>,
    pub stop: Symbol<'static, unsafe extern "C" fn(AsstHandle) -> AsstBool>,
    pub running: Symbol<'static, unsafe extern "C" fn(AsstHandle) -> AsstBool>,
    pub connected: Symbol<'static, unsafe extern "C" fn(AsstHandle) -> AsstBool>,
    pub back_to_home: Symbol<'static, unsafe extern "C" fn(AsstHandle) -> AsstBool>,
    pub get_version: Symbol<'static, unsafe extern "C" fn() -> *const c_char>,
}

impl MaaCore {
    /// Load MaaCore from the given `.so`/`.dll` path.
    ///
    /// # Safety
    /// Loads an arbitrary shared library; the caller must supply a path to a
    /// genuine MaaCore library. The library is intentionally leaked so its
    /// symbols live for the process lifetime (fine for a long-running server).
    pub unsafe fn load(path: &str) -> anyhow::Result<MaaCore> {
        // Leak the library to obtain a 'static reference so Symbol<'static, _>
        // is valid. The library lives for the lifetime of the process.
        let lib: &'static Library = Box::leak(Box::new(Library::new(path)?));

        macro_rules! sym {
            ($name:literal) => {
                lib.get::<_>(concat!($name, "\0").as_bytes())?
            };
        }

        Ok(MaaCore {
            set_user_dir: sym!("AsstSetUserDir"),
            load_resource: sym!("AsstLoadResource"),
            set_static_option: sym!("AsstSetStaticOption"),
            create: sym!("AsstCreate"),
            create_ex: sym!("AsstCreateEx"),
            destroy: sym!("AsstDestroy"),
            set_instance_option: sym!("AsstSetInstanceOption"),
            connect: sym!("AsstConnect"),
            append_task: sym!("AsstAppendTask"),
            set_task_params: sym!("AsstSetTaskParams"),
            start: sym!("AsstStart"),
            stop: sym!("AsstStop"),
            running: sym!("AsstRunning"),
            connected: sym!("AsstConnected"),
            back_to_home: sym!("AsstBackToHome"),
            get_version: sym!("AsstGetVersion"),
            _lib: lib,
        })
    }

    /// The MaaCore version string.
    pub fn version(&self) -> String {
        unsafe {
            let ptr = (self.get_version)();
            if ptr.is_null() {
                "unknown".to_string()
            } else {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        }
    }
}

// ---- Safe wrappers ----

/// A handle to an `Asst` instance, keeping the loaded library alive.
pub struct Asst {
    core: Arc<MaaCore>,
    handle: AsstHandle,
}

unsafe impl Send for Asst {}
unsafe impl Sync for Asst {}

impl Asst {
    pub fn create(core: Arc<MaaCore>, callback: Option<AsstApiCallback>, arg: *mut c_void) -> Self {
        let handle = unsafe {
            match callback {
                Some(cb) => (core.create_ex)(cb, arg),
                None => (core.create)(),
            }
        };
        assert!(!handle.is_null(), "AsstCreate returned null handle");
        Asst { core, handle }
    }

    pub fn set_user_dir(&self, path: &str) -> anyhow::Result<()> {
        let c = CString::new(path)?;
        let ok = unsafe { (self.core.set_user_dir)(c.as_ptr()) };
        anyhow::ensure!(ok != 0, "AsstSetUserDir failed");
        Ok(())
    }

    pub fn load_resource(&self, path: &str) -> anyhow::Result<()> {
        let c = CString::new(path)?;
        let ok = unsafe { (self.core.load_resource)(c.as_ptr()) };
        anyhow::ensure!(ok != 0, "AsstLoadResource failed");
        Ok(())
    }

    pub fn set_instance_option(&self, key: i32, value: &str) -> anyhow::Result<()> {
        let c = CString::new(value)?;
        let ok = unsafe { (self.core.set_instance_option)(self.handle, key, c.as_ptr()) };
        anyhow::ensure!(ok != 0, "AsstSetInstanceOption failed");
        Ok(())
    }

    /// Connect to a device via ADB. Synchronous (deprecated upstream but simplest for a first cut).
    pub fn connect(&self, adb_path: &str, address: &str, config: &str) -> anyhow::Result<()> {
        let adb = CString::new(adb_path)?;
        let addr = CString::new(address)?;
        let cfg = CString::new(config)?;
        let ok = unsafe { (self.core.connect)(self.handle, adb.as_ptr(), addr.as_ptr(), cfg.as_ptr()) };
        anyhow::ensure!(ok != 0, format!("AsstConnect failed for {address}"));
        Ok(())
    }

    pub fn append_task(&self, task_type: &str, params: &str) -> anyhow::Result<AsstTaskId> {
        let t = CString::new(task_type)?;
        let p = CString::new(params)?;
        let id = unsafe { (self.core.append_task)(self.handle, t.as_ptr(), p.as_ptr()) };
        anyhow::ensure!(id >= 0, "AsstAppendTask failed");
        Ok(id)
    }

    pub fn set_task_params(&self, task_id: AsstTaskId, params: &str) -> anyhow::Result<()> {
        let p = CString::new(params)?;
        let ok = unsafe { (self.core.set_task_params)(self.handle, task_id, p.as_ptr()) };
        anyhow::ensure!(ok != 0, "AsstSetTaskParams failed");
        Ok(())
    }

    pub fn start(&self) -> anyhow::Result<()> {
        let ok = unsafe { (self.core.start)(self.handle) };
        anyhow::ensure!(ok != 0, "AsstStart failed");
        Ok(())
    }

    pub fn stop(&self) -> anyhow::Result<()> {
        unsafe { (self.core.stop)(self.handle) };
        Ok(())
    }

    pub fn running(&self) -> bool {
        unsafe { (self.core.running)(self.handle) != 0 }
    }

    pub fn connected(&self) -> bool {
        unsafe { (self.core.connected)(self.handle) != 0 }
    }

    pub fn back_to_home(&self) -> anyhow::Result<()> {
        let ok = unsafe { (self.core.back_to_home)(self.handle) };
        anyhow::ensure!(ok != 0, "AsstBackToHome failed");
        Ok(())
    }
}

impl Drop for Asst {
    fn drop(&mut self) {
        unsafe { (self.core.destroy)(self.handle) };
    }
}
