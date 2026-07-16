use axum::{
    Router,
    routing::{get},
};
use std::sync::Arc;

use crate::application::service::semantic_detector::SemanticDetector;
use crate::presentation::controllers::api;
use crate::presentation::controllers::websocket;

pub fn create_routes() -> Router {
    let semantic_detector = Arc::new(SemanticDetector::new());
        
    Router::new()
        .route("/health", get(api::health::check_health))
        .route("/similarity", get(websocket::similarity::similarity_handler))
        .with_state(semantic_detector)
}
