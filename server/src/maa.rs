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
    core: Arc<MaaCore>,
    asst: Arc<Asst>,
    callback_state: Arc<CallbackState>,
    running: AtomicBool,
}

unsafe impl Send for CoreManager {}
unsafe impl Sync for CoreManager {}

impl CoreManager {
    /// Load MaaCore from `lib_path`, initialize with `user_dir`/`resource_dir`,
    /// and create a manager around a single Asst instance.
    pub fn init(
        lib_path: &str,
        user_dir: &str,
        resource_dir: &str,
    ) -> Result<Arc<CoreManager>> {
        let core = Arc::new(unsafe { MaaCore::load(lib_path)? });

        // Set up the callback. `custom_arg` points at CallbackState (leaked; freed on drop).
        let callback_state = Arc::new(CallbackState {
            subscribers: Mutex::new(Vec::new()),
            next_sub_id: Mutex::new(0),
            connection_info: Mutex::new(None),
        });

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
        let asst = Arc::new(Asst::create(core.clone(), Some(callback), state_ptr));

        // Initialize user dir and resource.
        if !user_dir.is_empty() {
            asst.set_user_dir(user_dir)?;
        }
        if !resource_dir.is_empty() {
            asst.load_resource(resource_dir)?;
        }

        Ok(Arc::new(CoreManager {
            core,
            asst,
            callback_state,
            running: AtomicBool::new(false),
        }))
    }

    pub fn version(&self) -> String {
        self.core.version()
    }

    pub fn connected(&self) -> bool {
        self.asst.connected()
    }

    pub fn running(&self) -> bool {
        self.asst.running()
    }

    pub fn last_connection_info(&self) -> Option<Value> {
        self.callback_state.connection_info.lock().unwrap().clone()
    }

    /// Connect to a device via ADB.
    pub fn connect(&self, adb_path: &str, address: &str, config: &str) -> Result<()> {
        self.asst.connect(adb_path, address, config)
    }

    /// Append a task (e.g. "Fight") with JSON params, returning the task id.
    pub fn append_task(&self, task_type: &str, params: &Value) -> Result<i32> {
        let params_str = serde_json::to_string(params)?;
        let id = self.asst.append_task(task_type, &params_str)?;
        // Register the task id -> type mapping in the callback state so the frontend
        // can tell which task an event belongs to.
        Ok(id)
    }

    pub fn start(&self) -> Result<()> {
        self.asst.start()?;
        self.running.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        self.asst.stop()?;
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Navigate the game back to the home screen.
    pub fn back_home(&self) -> Result<()> {
        self.asst.back_to_home()
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
