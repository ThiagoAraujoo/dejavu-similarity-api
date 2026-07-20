use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SemanticDetectionResult {
    pub match_found: bool,
    pub score: i32,
    pub matched_snippet: String,
    pub overall_similarity: f64,
    pub chunk_similarity: f64,
    pub ad_keywords: Vec<String>,
    pub matched_keywords: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SimilarityRequest {
    program_text: String,
    ad_text: String,
}

pub struct SemanticDetector {
    service_url: String,
    client: reqwest::Client,
}

impl SemanticDetector {
    pub fn new() -> Self {
        let service_url = std::env::var("SIMILARITY_SERVICE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8002/detect".to_string());
        
        Self {
            service_url,
            client: reqwest::Client::new(),
        }
    }

    /// Detect if an advertisement appears in a program transcription
    pub async fn detect_advertisement(
        &self,
        program_text: &str,
        ad_text: &str,
    ) -> Result<SemanticDetectionResult> {
        let total_start = std::time::Instant::now();
        tracing::debug!("Calling similarity service at {}", self.service_url);
        
        let request_body = SimilarityRequest {
            program_text: program_text.to_string(),
            ad_text: ad_text.to_string(),
        };
        
        let http_start = std::time::Instant::now();
        let response = self.client
            .post(&self.service_url)
            .json(&request_body)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .context("Failed to send request to similarity service")?;
        
        let http_duration = http_start.elapsed();
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Similarity service returned error {}: {}", status, error_text);
        }
        
        let parse_start = std::time::Instant::now();
        let result: SemanticDetectionResult = response.json().await
            .context("Failed to parse similarity service response")?;
        let parse_duration = parse_start.elapsed();
        
        let total_duration = total_start.elapsed();

        tracing::debug!(
            "Semantic detection: match={}, score={}, total={:.2}ms (http={:.2}ms, parse={:.2}ms)",
            result.match_found,
            result.score,
            total_duration.as_secs_f64() * 1000.0,
            http_duration.as_secs_f64() * 1000.0,
            parse_duration.as_secs_f64() * 1000.0
        );

        Ok(result)
    }
}

impl Default for SemanticDetector {
    fn default() -> Self {
        Self::new()
    }
}
