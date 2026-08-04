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
use crate::schedule::{ScheduledTask, Scheduler};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

/// 服务器运行配置（由 main 传入）
#[derive(Clone)]
pub struct ServerConfig {
    pub core_dir: PathBuf,
    pub web_dir: PathBuf,
}

#[derive(Clone)]
pub struct AppState {
    pub manager: Arc<CoreManager>,
    pub config: ServerConfig,
    pub scheduler: Arc<Scheduler>,
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
        .route("/api/tasks", get(list_tasks))
        .route("/api/tasks/clear", post(clear_tasks))
        .route("/api/start", post(start))
        .route("/api/stop", post(stop))
        .route("/api/back-home", post(back_home))
        .route("/api/update/check", get(update_check))
        .route("/api/update", post(update_core_api))
        .route("/api/schedule", get(schedule_list))
        .route("/api/schedule", post(schedule_add))
        .route("/api/schedule/:id", post(schedule_update))
        .route("/api/schedule/:id/delete", post(schedule_delete))
        .route("/api/ws", get(crate::ws::ws_handler))
        .with_state(state)
}

async fn get_version(State(state): State<AppState>) -> Json<ApiResponse<Value>> {
    let version = state.manager.version();
    ApiResponse::success(json!({ "version": version }))
}

async fn get_status(State(state): State<AppState>) -> Json<ApiResponse<Value>> {
    let healthy = state.manager.healthy();
    let conn = state.manager.connected();
    let running = state.manager.running();
    let last_conn = state.manager.last_connection_info();
    ApiResponse::success(json!({
        "healthy": healthy,
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

async fn list_tasks(State(state): State<AppState>) -> Json<ApiResponse<Value>> {
    let tasks = state.manager.list_tasks();
    ApiResponse::success(json!({ "tasks": tasks, "count": tasks.len() }))
}

async fn clear_tasks(State(state): State<AppState>) -> Json<ApiResponse<Value>> {
    state.manager.clear_task_list();
    ApiResponse::success(json!({ "cleared": true }))
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

// ==================== 更新 MaaCore ====================

/// 检查是否有 MaaCore 新版本
async fn update_check(State(state): State<AppState>) -> Json<ApiResponse<Value>> {
    let core_dir = state.config.core_dir.clone();
    match tokio::task::spawn_blocking(move || crate::update::check_update(&core_dir)).await {
        Ok(Ok((local, latest, has_update))) => {
            ApiResponse::success(json!({
                "local_version": local,
                "latest_version": latest,
                "has_update": has_update,
            }))
        }
        Ok(Err(e)) => ApiResponse::error(format!("检查更新失败: {e}")),
        Err(e) => ApiResponse::error(format!("后台任务失败: {e}")),
    }
}

/// 执行 MaaCore 更新
async fn update_core_api(State(state): State<AppState>) -> Json<ApiResponse<Value>> {
    let core_dir = state.config.core_dir.clone();

    // 更新期间避免任务运行
    let _ = state.manager.stop();

    match tokio::task::spawn_blocking(move || crate::update::update_core(&core_dir)).await {
        Ok(Ok(new_version)) => ApiResponse::success(json!({
            "updated": true,
            "new_version": new_version,
            "note": "更新已完成，重启服务后生效",
        })),
        Ok(Err(e)) => ApiResponse::error(format!("更新失败: {e}")),
        Err(e) => ApiResponse::error(format!("后台任务失败: {e}")),
    }
}

// ==================== 定时任务调度 ====================

/// 列出所有定时任务
async fn schedule_list(State(state): State<AppState>) -> Json<ApiResponse<Value>> {
    let list = state.scheduler.list().await;
    ApiResponse::success(json!({ "schedules": list }))
}

/// 添加定时任务
async fn schedule_add(
    State(state): State<AppState>,
    Json(task): Json<ScheduledTask>,
) -> Json<ApiResponse<Value>> {
    match state.scheduler.add(task.clone()).await {
        Ok(()) => ApiResponse::success(json!({ "added": true, "id": task.id })),
        Err(e) => ApiResponse::error(format!("添加定时任务失败: {e}")),
    }
}

/// 更新定时任务（按 id 路径参数）
async fn schedule_update(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(mut task): Json<ScheduledTask>,
) -> Json<ApiResponse<Value>> {
    task.id = id;
    match state.scheduler.update(task.clone()).await {
        Ok(()) => ApiResponse::success(json!({ "updated": true, "id": task.id })),
        Err(e) => ApiResponse::error(format!("更新定时任务失败: {e}")),
    }
}

/// 删除定时任务
async fn schedule_delete(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<ApiResponse<Value>> {
    match state.scheduler.remove(&id).await {
        Ok(()) => ApiResponse::success(json!({ "deleted": true, "id": id })),
        Err(e) => ApiResponse::error(format!("删除定时任务失败: {e}")),
    }
}
