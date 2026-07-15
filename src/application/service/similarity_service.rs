use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::application::service::similarity_result::SemanticDetectionResult;

/// Client for the persistent semantic similarity Python service.
///
/// The service loads the SentenceTransformer model once and serves
/// `POST /detect` requests, avoiding the overhead of spawning a Python
/// process and reloading the model on every comparison.
#[derive(Clone)]
pub struct SimilarityService {
    service_url: String,
    http_client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct DetectRequest<'a> {
    program_text: &'a str,
    ad_text: &'a str,
}

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: String,
}

impl SimilarityService {
    pub fn new() -> Result<Self> {
        let service_url = std::env::var("SIMILARITY_SERVICE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8002".to_string());

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .context("Failed to create similarity service HTTP client")?;

        tracing::info!(
            "Similarity service initialized - Similarity Service URL: {}",
            service_url
        );

        Ok(Self {
            service_url,
            http_client,
        })
    }

    /// Detect if an advertisement appears in a program transcription.
    pub async fn detect_advertisement(
        &self,
        program_text: &str,
        ad_text: &str,
    ) -> Result<SemanticDetectionResult> {
        let total_start = std::time::Instant::now();
        tracing::trace!("Calling similarity service");

        let payload = DetectRequest {
            program_text,
            ad_text,
        };

        let url = format!("{}/detect", self.service_url);
        let request_start = std::time::Instant::now();
        let response = self
            .http_client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .context("Failed to send request to similarity service")?;
        let request_duration = request_start.elapsed();

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Similarity service error {}: {}", status, error_text);
        }

        let parse_start = std::time::Instant::now();
        let body = response
            .text()
            .await
            .context("Failed to read similarity service response")?;

        // Try to parse as error first
        if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&body) {
            anyhow::bail!("Similarity service error: {}", error_response.error);
        }

        let result: SemanticDetectionResult = serde_json::from_str(&body)
            .context("Failed to parse similarity service JSON response")?;
        let parse_duration = parse_start.elapsed();
        let total_duration = total_start.elapsed();

        tracing::trace!(
            "Similarity detection: match={}, score={}, total={:.2}ms (http={:.2}ms, parse={:.2}ms)",
            result.match_found,
            result.score,
            total_duration.as_secs_f64() * 1000.0,
            request_duration.as_secs_f64() * 1000.0,
            parse_duration.as_secs_f64() * 1000.0
        );

        Ok(result)
    }
}

impl Default for SimilarityService {
    fn default() -> Self {
        Self::new().expect("Failed to initialize SimilarityService")
    }
}
