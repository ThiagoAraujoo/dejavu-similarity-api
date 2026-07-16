use axum::{
    extract::{ws::{Message, WebSocket}, Query, State, WebSocketUpgrade},
    response::{IntoResponse, Response},
    http::StatusCode,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::Deserialize;
use std::sync::Arc;

use crate::presentation::controllers::api::transcription::TranscriptionRestState;

#[derive(Debug, Deserialize)]
pub struct AuthParams {
    pub token: Option<String>,
}

pub async fn status_websocket_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<AuthParams>,
    State(state): State<Arc<TranscriptionRestState>>,
) -> Response {
    let expected_token = std::env::var("WEBSOCKET_AUTH_TOKEN")
        .unwrap_or_else(|_| {
            tracing::warn!("WEBSOCKET_AUTH_TOKEN not set in environment");
            String::new()
        });

    let provided_token = params.token.as_deref().unwrap_or("");

    if expected_token.is_empty() {
        tracing::error!("Transcription authentication token not configured");
        return (StatusCode::INTERNAL_SERVER_ERROR, "Authentication not configured").into_response();
    }

    if provided_token.is_empty() {
        tracing::warn!("WebSocket connection attempt without token");
        return (StatusCode::UNAUTHORIZED, "Authentication token required").into_response();
    }

    if provided_token != expected_token {
        tracing::warn!("WebSocket connection attempt with invalid token");
        return (StatusCode::UNAUTHORIZED, "Invalid authentication token").into_response();
    }

    tracing::info!("Status WebSocket connection authenticated successfully");
    ws.on_upgrade(|socket| handle_status_socket(socket, state))
}

async fn handle_status_socket(socket: WebSocket, state: Arc<TranscriptionRestState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut status_rx = state.status_tx.subscribe();

    tracing::info!("Status WebSocket connection established");

    let send_task = tokio::spawn(async move {
        while let Ok(update) = status_rx.recv().await {
            if let Ok(json) = serde_json::to_string(&update) {
                if let Err(e) = sender.send(Message::Text(json)).await {
                    tracing::error!("Failed to send status update: {}", e);
                    break;
                }
            }
        }
    });

    while let Some(msg) = receiver.next().await {
        if msg.is_err() {
            break;
        }
    }

    send_task.abort();
    tracing::info!("Status WebSocket connection closed");
}
