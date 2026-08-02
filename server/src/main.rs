//! MaaWeb server entry point.
//!
//! MaaWeb is a Web UI controller for MaaAssistantArknights. This server loads
//! MaaCore, exposes a small HTTP API, and streams MaaCore events over
//! WebSocket so a browser frontend can drive tasks.

mod api;
mod maa;
mod maa_core;
mod ws;

use anyhow::Result;
use axum::Router;
use clap::Parser;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

#[derive(Parser, Debug)]
#[command(name = "maaweb-server", version, about = "MaaWeb - Web UI controller for MaaAssistantArknights")]
struct Args {
    /// Path to libMaaCore.so (or .dll)
    #[arg(long, default_value = "core_runtime/libMaaCore.so")]
    core_lib: String,

    /// MaaCore user data directory (configs, cache)
    #[arg(long, default_value = "core_runtime")]
    user_dir: String,

    /// MaaCore resource directory (templates, images)
    #[arg(long, default_value = "core_runtime/resource")]
    resource_dir: String,

    /// Directory containing the built frontend (served at /)
    #[arg(long, default_value = "../web/dist")]
    web_dir: String,

    /// Listen address
    #[arg(long, default_value = "0.0.0.0:8080")]
    bind: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args = Args::parse();

    tracing::info!("Loading MaaCore from {}", args.core_lib);
    let manager = maa::CoreManager::init(&args.core_lib, &args.user_dir, &args.resource_dir);
    if manager.healthy() {
        tracing::info!("MaaCore version: {}", manager.version());
    } else {
        tracing::warn!("MaaCore 未加载，以降级模式运行（WebUI 可用，任务功能不可用）");
    }

    let state = api::AppState { manager };

    let app = Router::new()
        .merge(api::router(state))
        .fallback_service(ServeDir::new(&args.web_dir))
        .layer(CorsLayer::permissive());

    let addr: SocketAddr = args.bind.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("MaaWeb server listening on http://{addr}");

    axum::serve(listener, app).await?;
    Ok(())
}
