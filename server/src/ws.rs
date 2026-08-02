//! WebSocket handler: streams MaaCore events to the frontend in real time.

use crate::api::AppState;
use crate::maa::CoreManager;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures::SinkExt;
use serde_json::json;
use std::sync::Arc;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state.manager))
}

async fn handle_socket(mut socket: WebSocket, manager: Arc<CoreManager>) {
    // Subscribe to core events.
    let rx = manager.subscribe();

    // The C callback fires on a worker thread; move events onto the socket via a
    // std mpsc receiver + tokio select. Simplest approach: spawn a task that
    // drains the mpsc and forwards into a tokio mpsc, then await it here.
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<crate::maa::CoreEvent>(128);
    std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            if event_tx.blocking_send(event).is_err() {
                break;
            }
        }
    });

    // Send a hello message.
    let _ = socket.send(Message::Text(
        json!({ "type": "hello", "version": manager.version() }).to_string().into(),
    )).await;

    loop {
        tokio::select! {
            Some(event) = event_rx.recv() => {
                let payload = json!({
                    "type": "event",
                    "msg": event.msg_name,
                    "msg_id": event.msg,
                    "details": event.details,
                });
                if socket.send(Message::Text(payload.to_string().into())).await.is_err() {
                    break;
                }
            }
            Some(msg) = socket.recv() => {
                match msg {
                    Ok(Message::Close(_)) => break,
                    Ok(Message::Text(text)) => {
                        // Simple client ping/pong keepalive.
                        if text == "ping" {
                            let _ = socket.send(Message::Text("pong".into())).await;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
