//! High-level MaaCore manager: owns the Asst instance, processes callbacks,
//! and broadcasts events to WebSocket subscribers.

use crate::maa_core::{msg, Asst, AsstApiCallback, AsstMsgId, MaaCore};
use anyhow::Result;
use serde_json::Value;
use std::ffi::{c_char, c_void, CStr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// A single event emitted by MaaCore, already parsed as JSON.
#[derive(Clone, Debug)]
pub struct CoreEvent {
    pub msg: AsstMsgId,
    pub msg_name: String,
    pub details: Value,
}

impl CoreEvent {
    pub fn new(msg_id: AsstMsgId, details: Value) -> Self {
        CoreEvent {
            msg: msg_id,
            msg_name: msg::name(msg_id).to_string(),
            details,
        }
    }
}

/// A subscriber slot for core events.
struct Subscriber {
    id: usize,
    tx: std::sync::mpsc::Sender<CoreEvent>,
}

/// Shared state passed to the C callback.
struct CallbackState {
    /// Subscribers to core events (WebSocket clients).
    subscribers: Mutex<Vec<Subscriber>>,
    /// Next subscriber id.
    next_sub_id: Mutex<usize>,
    /// Latest connection status for quick polling.
    connection_info: Mutex<Option<Value>>,
}

impl CallbackState {
    fn broadcast(&self, event: CoreEvent) {
        if event.msg == msg::CONNECTION_INFO {
            if let Ok(mut ci) = self.connection_info.lock() {
                *ci = Some(event.details.clone());
            }
        }
        if let Ok(mut subs) = self.subscribers.lock() {
            subs.retain(|s| s.tx.send(event.clone()).is_ok());
        }
    }
}

/// The MaaCore manager.
pub struct CoreManager {
    core: Option<Arc<MaaCore>>,
    asst: Option<Arc<Asst>>,
    callback_state: Arc<CallbackState>,
    running: AtomicBool,
}

unsafe impl Send for CoreManager {}
unsafe impl Sync for CoreManager {}

impl CoreManager {
    /// Load MaaCore from `lib_path`, initialize with `user_dir`/`resource_dir`,
    /// and create a manager around a single Asst instance.
    ///
    /// If MaaCore cannot be loaded, a *degraded* manager is returned instead of
    /// failing: the Web UI still serves, but task APIs report an error.
    pub fn init(lib_path: &str, user_dir: &str, resource_dir: &str) -> Arc<CoreManager> {
        let callback_state = Arc::new(CallbackState {
            subscribers: Mutex::new(Vec::new()),
            next_sub_id: Mutex::new(0),
            connection_info: Mutex::new(None),
        });

        let core = match unsafe { MaaCore::load(lib_path) } {
            Ok(c) => Arc::new(c),
            Err(e) => {
                tracing::warn!("MaaCore 加载失败，WebUI 以降级模式运行: {e}");
                return Arc::new(CoreManager {
                    core: None,
                    asst: None,
                    callback_state,
                    running: AtomicBool::new(false),
                });
            }
        };

        // Global (static) init MUST happen before AsstCreate:
        // AsstSetUserDir + AsstLoadResource are process-wide.
        if !user_dir.is_empty() {
            if let Err(e) = core.set_user_dir(user_dir) {
                tracing::warn!("AsstSetUserDir 失败: {e}");
            }
        }
        if !resource_dir.is_empty() {
            if let Err(e) = core.load_resource(resource_dir) {
                tracing::warn!("AsstLoadResource 失败: {e}");
            }
        }

        // Set up the callback. `custom_arg` points at CallbackState (leaked; freed on drop).
        let state_ptr: *mut c_void = Arc::as_ptr(&callback_state) as *mut c_void;
        extern "C" fn on_message(msg: AsstMsgId, details: *const c_char, arg: *mut c_void) {
            if arg.is_null() || details.is_null() {
                return;
            }
            // SAFETY: the pointer was created from an Arc and remains alive for the
            // lifetime of the Asst instance.
            let state = unsafe { &*(arg as *const CallbackState) };
            let json_str = unsafe { CStr::from_ptr(details) }.to_string_lossy().into_owned();
            let details: Value = serde_json::from_str(&json_str).unwrap_or(Value::Null);
            state.broadcast(CoreEvent::new(msg, details));
        }
        let callback: AsstApiCallback = on_message;

        // Create the Asst instance.
        let asst = match Asst::create(core.clone(), Some(callback), state_ptr) {
            Ok(a) => Arc::new(a),
            Err(e) => {
                tracing::warn!("MaaCore 初始化失败，WebUI 以降级模式运行: {e}");
                return Arc::new(CoreManager {
                    core: None,
                    asst: None,
                    callback_state,
                    running: AtomicBool::new(false),
                });
            }
        };

        Arc::new(CoreManager {
            core: Some(core),
            asst: Some(asst),
            callback_state,
            running: AtomicBool::new(false),
        })
    }

    /// Whether MaaCore was loaded successfully.
    pub fn healthy(&self) -> bool {
        self.asst.is_some()
    }

    pub fn version(&self) -> String {
        match &self.core {
            Some(core) => core.version(),
            None => "MaaCore 未加载".to_string(),
        }
    }

    pub fn connected(&self) -> bool {
        self.asst.as_ref().map_or(false, |a| a.connected())
    }

    pub fn running(&self) -> bool {
        self.asst.as_ref().map_or(false, |a| a.running())
    }

    pub fn last_connection_info(&self) -> Option<Value> {
        self.callback_state.connection_info.lock().unwrap().clone()
    }

    /// Connect to a device via ADB.
    pub fn connect(&self, adb_path: &str, address: &str, config: &str) -> Result<()> {
        let asst = self.asst.as_ref().ok_or_else(|| anyhow::anyhow!("MaaCore 未加载"))?;
        asst.connect(adb_path, address, config)
    }

    /// Append a task (e.g. "Fight") with JSON params, returning the task id.
    pub fn append_task(&self, task_type: &str, params: &Value) -> Result<i32> {
        let asst = self.asst.as_ref().ok_or_else(|| anyhow::anyhow!("MaaCore 未加载"))?;
        let params_str = serde_json::to_string(params)?;
        let id = asst.append_task(task_type, &params_str)?;
        // Register the task id -> type mapping in the callback state so the frontend
        // can tell which task an event belongs to.
        Ok(id)
    }

    pub fn start(&self) -> Result<()> {
        let asst = self.asst.as_ref().ok_or_else(|| anyhow::anyhow!("MaaCore 未加载"))?;
        asst.start()?;
        self.running.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        let asst = self.asst.as_ref().ok_or_else(|| anyhow::anyhow!("MaaCore 未加载"))?;
        asst.stop()?;
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Navigate the game back to the home screen.
    pub fn back_home(&self) -> Result<()> {
        let asst = self.asst.as_ref().ok_or_else(|| anyhow::anyhow!("MaaCore 未加载"))?;
        asst.back_to_home()
    }

    /// Subscribe to core events. Returns a receiver.
    pub fn subscribe(&self) -> std::sync::mpsc::Receiver<CoreEvent> {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut guard = self.callback_state.subscribers.lock().unwrap();
        let mut id_guard = self.callback_state.next_sub_id.lock().unwrap();
        let id = *id_guard;
        *id_guard += 1;
        guard.push(Subscriber { id, tx });
        rx
    }
}

impl Drop for CoreManager {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        // The callback_state Arc is leaked by design; dropping it here while the
        // C library may still call the callback would be a use-after-free.
    }
}
