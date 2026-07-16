use axum::{
    extract::{ws::{Message, WebSocket}, Query, State, WebSocketUpgrade},
    response::{IntoResponse, Response},
    http::StatusCode,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};

use crate::application::service::semantic_detector::SemanticDetector;

#[derive(Debug, Deserialize)]
pub struct AuthParams {
    pub token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WebSocketRequest {
    pub uuid: String,
    pub programming_id: i32,
    pub programming_transcription: String,
    pub advertisement_id: i32,
    pub advertisement_transcription: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WebSocketResponse {
    pub uuid: String,
    pub programming_id: i32,
    pub programming_transcription: String,
    pub advertisement_id: i32,
    pub advertisement_transcription: String,
    pub match_found: bool,
    pub score: i32,
    pub error: Option<String>,
}

pub async fn similarity_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<AuthParams>,
    State(detector): State<Arc<SemanticDetector>>,
) -> Response {
    // Validate authentication token
    let expected_token = std::env::var("WEBSOCKET_AUTH_TOKEN")
        .unwrap_or_else(|_| {
            tracing::warn!("WEBSOCKET_AUTH_TOKEN not set in environment");
            String::new()
        });

    let provided_token = params.token.as_deref().unwrap_or("");

    if expected_token.is_empty() {
        tracing::error!("Authentication token not configured in environment");
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

    tracing::info!("WebSocket connection authenticated successfully");
    ws.on_upgrade(|socket| handle_socket(socket, detector))
}

async fn handle_socket(socket: WebSocket, detector: Arc<SemanticDetector>) {
    let max_concurrent = std::env::var("MAX_CONCURRENT_TASKS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| num_cpus::get());

    tracing::info!(
        "WebSocket connection established. Max concurrent tasks: {}",
        max_concurrent
    );

    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<WebSocketResponse>();

    // Spawn task to send responses
    let send_task = tokio::spawn(async move {
        while let Some(response) = rx.recv().await {
            if let Ok(response_json) = serde_json::to_string(&response) {
                if let Err(e) = sender.send(Message::Text(response_json)).await {
                    tracing::error!("Failed to send response: {}", e);
                    break;
                }
            }
        }
    });

    // Process incoming messages
    while let Some(msg) = receiver.next().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                tracing::error!("WebSocket error: {}", e);
                break;
            }
        };

        let text = match msg.to_text() {
            Ok(text) => text,
            Err(_) => continue,
        };

        let request: WebSocketRequest = match serde_json::from_str(text) {
            Ok(req) => req,
            Err(e) => {
                tracing::error!("Failed to parse request: {}. Received text: '{}'", e, text);
                continue;
            }
        };

        let uuid = request.uuid.clone();
        let detector = detector.clone();
        let semaphore = semaphore.clone();
        let tx = tx.clone();

        tokio::spawn(async move {
            let _permit = match semaphore.acquire().await {
                Ok(permit) => permit,
                Err(e) => {
                    tracing::error!("Failed to acquire semaphore: {}", e);
                    return;
                }
            };

            tracing::debug!(
                "Processing request {} (permits available: {})",
                uuid,
                semaphore.available_permits()
            );

            let response = process_request(detector, request).await;

            if let Err(e) = tx.send(response.clone()) {
                tracing::error!("Failed to queue response: {}", e);
            }

            tracing::debug!(
                "Completed request {} (permits available: {})",
                response.uuid,
                semaphore.available_permits() + 1
            );
        });
    }

    drop(tx);
    let _ = send_task.await;
    tracing::info!("WebSocket connection closed");
}

async fn process_request(
    detector: Arc<SemanticDetector>,
    request: WebSocketRequest,
) -> WebSocketResponse {
    let timeout_seconds = std::env::var("TASK_TIMEOUT_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);

    let timeout_duration = std::time::Duration::from_secs(timeout_seconds);

    let result = tokio::time::timeout(
        timeout_duration,
        detector.detect_advertisement(
            &request.programming_transcription,
            &request.advertisement_transcription,
        ),
    )
    .await;

    match result {
        Ok(Ok(detection_result)) => WebSocketResponse {
            uuid: request.uuid,
            programming_id: request.programming_id,
            programming_transcription: request.programming_transcription,
            advertisement_id: request.advertisement_id,
            advertisement_transcription: request.advertisement_transcription,
            match_found: detection_result.match_found,
            score: detection_result.score,
            error: None,
        },
        Ok(Err(e)) => {
            tracing::error!("Detection failed for request {}: {}", request.uuid, e);
            WebSocketResponse {
                uuid: request.uuid,
                programming_id: request.programming_id,
                programming_transcription: request.programming_transcription,
                advertisement_id: request.advertisement_id,
                advertisement_transcription: request.advertisement_transcription,
                match_found: false,
                score: 0,
                error: Some(e.to_string()),
            }
        }
        Err(_) => {
            tracing::error!(
                "Detection timeout for request {} after {} seconds",
                request.uuid,
                timeout_seconds
            );
            WebSocketResponse {
                uuid: request.uuid,
                programming_id: request.programming_id,
                programming_transcription: request.programming_transcription,
                advertisement_id: request.advertisement_id,
                advertisement_transcription: request.advertisement_transcription,
                match_found: false,
                score: 0,
                error: Some(format!("Timeout after {} seconds", timeout_seconds)),
            }
        }
    }
}
