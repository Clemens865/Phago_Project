//! WebSocket handler for real-time colony events.

use crate::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use tokio::sync::broadcast;

/// WebSocket upgrade handler for /ws/events.
pub async fn events_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle a WebSocket connection.
async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.subscribe();

    // Send initial snapshot
    {
        let snapshot = state.snapshot().await;
        let msg = serde_json::json!({
            "type": "snapshot",
            "data": snapshot
        });
        if let Ok(json) = serde_json::to_string(&msg) {
            let _ = socket.send(Message::Text(json.into())).await;
        }
    }

    // Stream events as they arrive
    loop {
        tokio::select! {
            // Receive events from the broadcast channel
            result = rx.recv() => {
                match result {
                    Ok(event) => {
                        let msg = serde_json::json!({
                            "type": "event",
                            "data": event
                        });
                        if let Ok(json) = serde_json::to_string(&msg) {
                            if socket.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        // Client is too slow, skip some events
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
            // Handle incoming messages (e.g., ping/pong, commands)
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        // Handle commands from client
                        if let Ok(cmd) = serde_json::from_str::<ClientCommand>(&text) {
                            match cmd {
                                ClientCommand::Tick { count } => {
                                    state.run(count.unwrap_or(1)).await;
                                }
                                ClientCommand::Snapshot => {
                                    let snapshot = state.snapshot().await;
                                    let msg = serde_json::json!({
                                        "type": "snapshot",
                                        "data": snapshot
                                    });
                                    if let Ok(json) = serde_json::to_string(&msg) {
                                        let _ = socket.send(Message::Text(json.into())).await;
                                    }
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}

/// Commands that can be sent over WebSocket.
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "cmd")]
enum ClientCommand {
    #[serde(rename = "tick")]
    Tick { count: Option<u64> },
    #[serde(rename = "snapshot")]
    Snapshot,
}
