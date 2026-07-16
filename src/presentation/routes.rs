use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::application::service::semantic_detector::SemanticDetector;
use crate::application::service::transcription_service::TranscriptionService;
use crate::utils::noise_removal::NoiseRemovalService;
use crate::presentation::controllers::api;
use crate::presentation::controllers::websocket;

pub fn create_routes() -> Router {
    let semantic_detector = Arc::new(SemanticDetector::new());

    // Initialize similarity services
    let transcription_service = match TranscriptionService::new() {
        Ok(service) => service,
        Err(e) => {
            tracing::error!("Failed to initialize similarity service: {}", e);
            tracing::warn!("Similarity endpoints will not be available");
            return Router::new()
                .route("/health", get(api::health::check_health))
                .route("/similarity", get(websocket::similarity::similarity_handler))
                .with_state(semantic_detector);
        }
    };

    let noise_removal_service = NoiseRemovalService::new(None, None);
    
    // Create broadcast channel for status updates
    let (status_tx, _) = broadcast::channel(100);
    
    // REST state for file upload
    let rest_state = Arc::new(api::similarity::TranscriptionRestState {
        transcription_service,
        noise_removal_service,
        status_tx,
    });

    Router::new()
        .route("/health", get(api::health::check_health))
        .route("/similarity", get(websocket::similarity::similarity_handler))
        .with_state(semantic_detector)
        // REST endpoint for file upload
        .route("/similarity", post(api::similarity::upload_file))
        .with_state(rest_state.clone())
        // WebSocket for status updates
        .route("/similarity", get(websocket::similarity::status_websocket_handler))
        .with_state(rest_state)
}
