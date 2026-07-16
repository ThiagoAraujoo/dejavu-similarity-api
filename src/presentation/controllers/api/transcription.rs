use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::Json,
};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::application::service::transcription_service::{TranscriptionService, TranscriptionResult};
use crate::utils::noise_removal::NoiseRemovalService;

#[derive(Debug, Clone, Serialize)]
pub struct TranscriptionUploadResponse {
    pub uuid: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TranscriptionStatusUpdate {
    pub uuid: String,
    pub status: String,
    pub progress: f32,
    pub message: String,
    pub result: Option<TranscriptionResult>,
    pub error: Option<String>,
}

pub struct TranscriptionRestState {
    pub transcription_service: TranscriptionService,
    pub noise_removal_service: NoiseRemovalService,
    pub status_tx: broadcast::Sender<TranscriptionStatusUpdate>,
}

pub async fn upload_file(
    State(state): State<Arc<TranscriptionRestState>>,
    mut multipart: Multipart,
) -> Result<Json<TranscriptionUploadResponse>, (StatusCode, String)> {
    let mut token: Option<String> = None;
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut apply_noise_removal = true;
    let mut model_type: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        (StatusCode::BAD_REQUEST, format!("Failed to read multipart field: {}", e))
    })? {
        let name = field.name().unwrap_or("").to_string();
        
        match name.as_str() {
            "token" => {
                token = Some(field.text().await.map_err(|e| {
                    (StatusCode::BAD_REQUEST, format!("Failed to read token: {}", e))
                })?);
            }
            "file" => {
                file_name = field.file_name().map(|s| s.to_string());
                let data = field.bytes().await.map_err(|e| {
                    (StatusCode::BAD_REQUEST, format!("Failed to read file: {}", e))
                })?;
                file_data = Some(data.to_vec());
            }
            "model" => {
                let _model = field.text().await.map_err(|e| {
                    (StatusCode::BAD_REQUEST, format!("Failed to read model: {}", e))
                })?;
            }
            "priority" => {
                let _priority_str = field.text().await.map_err(|e| {
                    (StatusCode::BAD_REQUEST, format!("Failed to read priority: {}", e))
                })?;
            }
            "apply_noise_removal" => {
                let value = field.text().await.map_err(|e| {
                    (StatusCode::BAD_REQUEST, format!("Failed to read apply_noise_removal: {}", e))
                })?;
                apply_noise_removal = value == "true" || value == "1";
            }
            "model_type" => {
                let value = field.text().await.map_err(|e| {
                    (StatusCode::BAD_REQUEST, format!("Failed to read model_type: {}", e))
                })?;
                model_type = Some(value);
            }
            _ => {}
        }
    }

    let expected_token = std::env::var("WEBSOCKET_AUTH_TOKEN")
        .unwrap_or_else(|_| String::new());

    if expected_token.is_empty() {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "Authentication not configured".to_string()));
    }

    let provided_token = token.as_deref().unwrap_or("");
    if provided_token != expected_token {
        return Err((StatusCode::UNAUTHORIZED, "Invalid authentication token".to_string()));
    }

    let file_bytes = file_data.ok_or_else(|| {
        (StatusCode::BAD_REQUEST, "No file provided".to_string())
    })?;

    if file_bytes.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Empty file provided".to_string()));
    }

    let uuid = Uuid::new_v4().to_string();

    tracing::info!(
        "Received similarity upload: uuid={}, size={} bytes",
        uuid,
        file_bytes.len()
    );

    let state_clone = state.clone();
    let uuid_clone = uuid.clone();
    tokio::spawn(async move {
        process_transcription_job(
            state_clone,
            uuid_clone,
            file_bytes,
            file_name,
            apply_noise_removal,
            model_type,
        )
        .await;
    });

    Ok(Json(TranscriptionUploadResponse {
        uuid: uuid.clone(),
        status: "processing".to_string(),
        message: format!("Similarity job {} started. Connect to WebSocket for updates.", uuid),
    }))
}

async fn process_transcription_job(
    state: Arc<TranscriptionRestState>,
    uuid: String,
    file_bytes: Vec<u8>,
    original_file_name: Option<String>,
    apply_noise_removal: bool,
    model_type: Option<String>,
) {
    let job_start_time = std::time::Instant::now();
    tracing::info!("Starting similarity job: {}", uuid);

    let ext = original_file_name.as_deref()
        .and_then(|n| std::path::Path::new(n).extension())
        .and_then(|e| e.to_str())
        .unwrap_or("wav");
    let suffix = format!(".{}", ext);

    let temp_file = match tempfile::Builder::new().suffix(&suffix).tempfile() {
        Ok(file) => file,
        Err(e) => {
            let _ = state.status_tx.send(TranscriptionStatusUpdate {
                uuid: uuid.clone(),
                status: "failed".to_string(),
                progress: 0.0,
                message: "Failed to create temporary file".to_string(),
                result: None,
                error: Some(e.to_string()),
            });
            return;
        }
    };

    if let Err(e) = std::io::Write::write_all(&mut &temp_file, &file_bytes)
        .and_then(|_| std::io::Write::flush(&mut &temp_file))
    {
        let _ = state.status_tx.send(TranscriptionStatusUpdate {
            uuid: uuid.clone(),
            status: "failed".to_string(),
            progress: 0.0,
            message: "Failed to write file".to_string(),
            result: None,
            error: Some(e.to_string()),
        });
        return;
    }

    let temp_path = temp_file.path().to_str().unwrap().to_string();

    tracing::info!("Job {}: {}", uuid, if apply_noise_removal { "Applying noise removal" } else { "Skipping noise removal" });

    let noise_removal_start = std::time::Instant::now();
    let audio_path = if apply_noise_removal {
        match state.noise_removal_service.remove_noise(&temp_path, None).await {
            Ok(cleaned_path) => {
                let noise_removal_duration = noise_removal_start.elapsed();
                tracing::info!("[PERF] Job {}: noise_removal={:.2}s", uuid, noise_removal_duration.as_secs_f64());
                cleaned_path
            },
            Err(e) => {
                tracing::warn!("Noise removal failed, using original: {}", e);
                temp_path.clone()
            }
        }
    } else {
        temp_path.clone()
    };

    let engine_type = model_type.as_deref().unwrap_or("default");
    tracing::info!("Job {}: Starting similarity with engine: {}", uuid, engine_type);

    let transcription_start = std::time::Instant::now();
    match state.transcription_service.transcribe_audio_with_engine(&audio_path, &uuid, model_type).await {
        Ok(result) => {
            let transcription_duration = transcription_start.elapsed();
            let total_duration = job_start_time.elapsed();
            tracing::info!("[PERF] Job {}: similarity={:.2}s, total={:.2}s", 
                uuid, transcription_duration.as_secs_f64(), total_duration.as_secs_f64());
            
            let _ = state.status_tx.send(TranscriptionStatusUpdate {
                uuid: uuid.clone(),
                status: "completed".to_string(),
                progress: 100.0,
                message: "Similarity completed successfully".to_string(),
                result: Some(result),
                error: None,
            });
        }
        Err(e) => {
            let total_duration = job_start_time.elapsed();
            tracing::error!("[PERF] Job {}: failed after {:.2}s", uuid, total_duration.as_secs_f64());
            
            let _ = state.status_tx.send(TranscriptionStatusUpdate {
                uuid: uuid.clone(),
                status: "failed".to_string(),
                progress: 0.0,
                message: "Similarity failed".to_string(),
                result: None,
                error: Some(e.to_string()),
            });
        }
    }

    if audio_path != temp_path {
        let _ = tokio::fs::remove_file(&audio_path).await;
    }
}
