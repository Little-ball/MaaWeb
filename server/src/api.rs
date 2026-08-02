//! HTTP API routes for MaaWeb.
//!
//! Endpoints:
//! - GET  /api/version        -> MaaCore version
//! - GET  /api/status         -> connection + running state
//! - POST /api/connect        -> connect to ADB device
//! - POST /api/disconnect     -> stop tasks (no explicit disconnect in old C API)
//! - POST /api/task           -> append a task
//! - POST /api/start          -> start task queue
//! - POST /api/stop           -> stop task queue
//! - POST /api/back-home      -> navigate to home screen
//! - WS   /api/ws             -> realtime event stream

use crate::maa::CoreManager;
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub manager: Arc<CoreManager>,
}

#[derive(Serialize)]
struct ApiResponse<T> {
    ok: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Json<Self> {
        Json(ApiResponse { ok: true, data: Some(data), error: None })
    }
}

impl ApiResponse<Value> {
    fn error(msg: impl Into<String>) -> Json<Self> {
        Json(ApiResponse { ok: false, data: None, error: Some(msg.into()) })
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/version", get(get_version))
        .route("/api/status", get(get_status))
        .route("/api/connect", post(connect))
        .route("/api/task", post(append_task))
        .route("/api/start", post(start))
        .route("/api/stop", post(stop))
        .route("/api/back-home", post(back_home))
        .route("/api/ws", get(crate::ws::ws_handler))
        .with_state(state)
}

async fn get_version(State(state): State<AppState>) -> Json<ApiResponse<Value>> {
    let version = state.manager.version();
    ApiResponse::success(json!({ "version": version }))
}

async fn get_status(State(state): State<AppState>) -> Json<ApiResponse<Value>> {
    let conn = state.manager.connected();
    let running = state.manager.running();
    let last_conn = state.manager.last_connection_info();
    ApiResponse::success(json!({
        "connected": conn,
        "running": running,
        "last_connection_info": last_conn,
    }))
}

#[derive(Deserialize)]
struct ConnectRequest {
    adb_path: String,
    address: String,
    #[serde(default = "default_config")]
    config: String,
}

fn default_config() -> String {
    "General".to_string()
}

async fn connect(State(state): State<AppState>, Json(req): Json<ConnectRequest>) -> Json<ApiResponse<Value>> {
    match state.manager.connect(&req.adb_path, &req.address, &req.config) {
        Ok(()) => ApiResponse::success(json!({ "connected": true })),
        Err(e) => ApiResponse::error(format!("连接失败: {e}")),
    }
}

#[derive(Deserialize)]
struct TaskRequest {
    task_type: String,
    #[serde(default)]
    params: Value,
}

async fn append_task(
    State(state): State<AppState>,
    Json(req): Json<TaskRequest>,
) -> Json<ApiResponse<Value>> {
    match state.manager.append_task(&req.task_type, &req.params) {
        Ok(id) => ApiResponse::success(json!({ "task_id": id, "type": req.task_type })),
        Err(e) => ApiResponse::error(format!("添加任务失败: {e}")),
    }
}

async fn start(State(state): State<AppState>) -> Json<ApiResponse<Value>> {
    match state.manager.start() {
        Ok(()) => ApiResponse::success(json!({ "started": true })),
        Err(e) => ApiResponse::error(format!("启动失败: {e}")),
    }
}

async fn stop(State(state): State<AppState>) -> Json<ApiResponse<Value>> {
    match state.manager.stop() {
        Ok(()) => ApiResponse::success(json!({ "stopped": true })),
        Err(e) => ApiResponse::error(format!("停止失败: {e}")),
    }
}

async fn back_home(State(state): State<AppState>) -> Json<ApiResponse<Value>> {
    match state.manager.back_home() {
        Ok(()) => ApiResponse::success(json!({ "back_home": true })),
        Err(e) => ApiResponse::error(format!("返回首页失败: {e}")),
    }
}
