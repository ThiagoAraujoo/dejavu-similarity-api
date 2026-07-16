use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub uuid: String,
    pub transcription: String,
    pub vtt: String,
    pub srt: String,
    pub json_file: String,
    pub tsv: String,
    pub duration_seconds: f64,
    pub language: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WhisperXResponse {
    success: bool,
    output_files: Option<HashMap<String, String>>,
    language: Option<String>,
    error: Option<String>,
}

#[derive(Clone)]
pub struct TranscriptionService {
    whisperx_service_url: String,
    http_client: reqwest::Client,
}

impl TranscriptionService {
    pub fn new() -> Result<Self> {
        let whisperx_service_url = std::env::var("WHISPERX_SERVICE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8001".to_string());

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .context("Failed to create HTTP client")?;

        tracing::info!(
            "Transcription service initialized - WhisperX Service: {}",
            whisperx_service_url
        );

        Ok(Self {
            whisperx_service_url,
            http_client,
        })
    }

    pub async fn transcribe_audio_with_engine(
        &self,
        audio_path: &str,
        uuid: &str,
        _engine_override: Option<String>,
    ) -> Result<TranscriptionResult> {
        tracing::info!("Starting transcription for: {} (uuid: {}) using WhisperX service", audio_path, uuid);
        let start_time = std::time::Instant::now();

        // Prepare request payload
        let mut payload = HashMap::new();
        payload.insert("audio_file", audio_path);
        payload.insert("uuid", uuid);
        payload.insert("output_dir", "/tmp");
        payload.insert("output_format", "all");

        // Call WhisperX service
        let url = format!("{}/transcribe", self.whisperx_service_url);
        let response = self.http_client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .context("Failed to send request to WhisperX service")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            tracing::error!("WhisperX service returned error {}: {}", status, error_text);
            anyhow::bail!("WhisperX service error: {} - {}", status, error_text);
        }

        let whisperx_response: WhisperXResponse = response.json().await
            .context("Failed to parse WhisperX response")?;

        if !whisperx_response.success {
            let error_msg = whisperx_response.error.unwrap_or_else(|| "Unknown error".to_string());
            tracing::error!("WhisperX transcription failed: {}", error_msg);
            anyhow::bail!("WhisperX transcription failed: {}", error_msg);
        }

        let output_files = whisperx_response.output_files
            .context("WhisperX response missing output_files")?;

        // Read all output files
        let txt_file = output_files.get("txt")
            .context("Missing txt file in WhisperX response")?;
        let vtt_file = output_files.get("vtt")
            .context("Missing vtt file in WhisperX response")?;
        let srt_file = output_files.get("srt")
            .context("Missing srt file in WhisperX response")?;
        let json_file = output_files.get("json")
            .context("Missing json file in WhisperX response")?;
        let tsv_file = output_files.get("tsv")
            .context("Missing tsv file in WhisperX response")?;

        let transcription_text = tokio::fs::read_to_string(txt_file)
            .await
            .context("Failed to read transcription text file")?;

        let vtt_content = tokio::fs::read_to_string(vtt_file)
            .await
            .context("Failed to read VTT file")?;

        let srt_content = tokio::fs::read_to_string(srt_file)
            .await
            .context("Failed to read SRT file")?;

        let json_content = tokio::fs::read_to_string(json_file)
            .await
            .context("Failed to read JSON file")?;

        let tsv_content = tokio::fs::read_to_string(tsv_file)
            .await
            .context("Failed to read TSV file")?;

        // Cleanup all output files
        let _ = tokio::fs::remove_file(txt_file).await;
        let _ = tokio::fs::remove_file(vtt_file).await;
        let _ = tokio::fs::remove_file(srt_file).await;
        let _ = tokio::fs::remove_file(json_file).await;
        let _ = tokio::fs::remove_file(tsv_file).await;

        let duration = start_time.elapsed();
        
        tracing::info!(
            "Transcription completed in {:.2}s: {} characters",
            duration.as_secs_f64(),
            transcription_text.len()
        );

        Ok(TranscriptionResult {
            uuid: uuid.to_string(),
            transcription: transcription_text.trim().to_string(),
            vtt: vtt_content,
            srt: srt_content,
            json_file: json_content,
            tsv: tsv_content,
            duration_seconds: duration.as_secs_f64(),
            language: whisperx_response.language,
        })
    }
}