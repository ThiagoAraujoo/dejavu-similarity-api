use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::application::service::semantic_detector::SemanticDetector;

#[derive(Debug, Deserialize)]
pub struct SimilarityRequest {
    pub uuid: String,
    pub programming_id: i32,
    pub programming_transcription: String,
    pub advertisement_id: i32,
    pub advertisement_transcription: String,
}

#[derive(Debug, Serialize)]
pub struct SimilarityResponse {
    pub uuid: String,
    pub programming_id: i32,
    pub programming_transcription: String,
    pub advertisement_id: i32,
    pub advertisement_transcription: String,
    pub match_found: bool,
    pub score: i32,
    pub error: Option<String>,
}

pub async fn analyze_similarity(
    State(detector): State<Arc<SemanticDetector>>,
    Json(request): Json<SimilarityRequest>,
) -> Result<Json<SimilarityResponse>, (StatusCode, String)> {
    tracing::info!(
        "Received similarity analysis request: uuid={}, programming_id={}, advertisement_id={}",
        request.uuid,
        request.programming_id,
        request.advertisement_id
    );
    
    if request.programming_transcription.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "programming_transcription cannot be empty".to_string(),
        ));
    }
    
    if request.advertisement_transcription.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "advertisement_transcription cannot be empty".to_string(),
        ));
    }
    
    match detector.detect_advertisement(
        &request.programming_transcription,
        &request.advertisement_transcription
    ).await {
        Ok(result) => {
            tracing::info!(
                "Similarity analysis complete: uuid={}, match={}, score={}",
                request.uuid,
                result.match_found,
                result.score
            );
            Ok(Json(SimilarityResponse {
                uuid: request.uuid,
                programming_id: request.programming_id,
                programming_transcription: request.programming_transcription,
                advertisement_id: request.advertisement_id,
                advertisement_transcription: request.advertisement_transcription,
                match_found: result.match_found,
                score: result.score,
                error: None,
            }))
        }
        Err(e) => {
            tracing::error!("Similarity analysis failed: uuid={}, error={}", request.uuid, e);
            Ok(Json(SimilarityResponse {
                uuid: request.uuid,
                programming_id: request.programming_id,
                programming_transcription: request.programming_transcription,
                advertisement_id: request.advertisement_id,
                advertisement_transcription: request.advertisement_transcription,
                match_found: false,
                score: 0,
                error: Some(format!("Analysis failed: {}", e)),
            }))
        }
    }
}
