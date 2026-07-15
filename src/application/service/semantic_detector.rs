use anyhow::{Context, Result};
use std::process::Command;

use crate::application::service::similarity_result::{ErrorResponse, SemanticDetectionResult};
use crate::application::service::similarity_service::SimilarityService;

/// Old CLI-based detector that spawns a Python process for every request.
/// Kept as a fallback when the persistent HTTP similarity service is unavailable.
struct CliSemanticDetector {
    python_script_path: String,
}

impl CliSemanticDetector {
    fn new(script_path: String) -> Self {
        Self {
            python_script_path: script_path,
        }
    }

    async fn detect_advertisement(
        &self,
        program_text: &str,
        ad_text: &str,
    ) -> Result<SemanticDetectionResult> {
        let total_start = std::time::Instant::now();
        tracing::trace!("Calling Python semantic detector (CLI fallback)");

        let spawn_start = std::time::Instant::now();
        let output = tokio::task::spawn_blocking({
            let script_path = self.python_script_path.clone();
            let program = program_text.to_string();
            let ad = ad_text.to_string();

            move || {
                let exec_start = std::time::Instant::now();
                let result = Command::new("python3")
                    .arg(&script_path)
                    .arg(&program)
                    .arg(&ad)
                    .output();
                let exec_duration = exec_start.elapsed();
                (result, exec_duration)
            }
        })
        .await
        .context("Failed to spawn Python process")?;

        let spawn_duration = spawn_start.elapsed();
        let (output_result, python_exec_duration) = output;
        let output = output_result?;

        if !output.stderr.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            for line in stderr.lines() {
                tracing::debug!("Python: {}", line);
            }
        }

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Python script failed: {}", stderr);
        }

        let parse_start = std::time::Instant::now();
        let stdout =
            String::from_utf8(output.stdout).context("Failed to parse Python output as UTF-8")?;

        tracing::trace!("Python output: {}", stdout);

        if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&stdout) {
            anyhow::bail!("Python script error: {}", error_response.error);
        }

        let result: SemanticDetectionResult =
            serde_json::from_str(&stdout).context("Failed to parse Python JSON response")?;
        let parse_duration = parse_start.elapsed();
        let total_duration = total_start.elapsed();

        tracing::trace!(
            "CLI semantic detection: match={}, score={}, total={:.2}ms (spawn={:.2}ms, python={:.2}ms, parse={:.2}ms)",
            result.match_found,
            result.score,
            total_duration.as_secs_f64() * 1000.0,
            spawn_duration.as_secs_f64() * 1000.0,
            python_exec_duration.as_secs_f64() * 1000.0,
            parse_duration.as_secs_f64() * 1000.0
        );

        Ok(result)
    }
}

/// Primary similarity detector.
///
/// Uses a persistent Python HTTP service that keeps the SentenceTransformer model
/// loaded in memory. Falls back to the CLI-based detector if the service is
/// unavailable or if explicitly requested via the `SIMILARITY_SERVICE_URL=cli`
/// environment variable.
pub struct SemanticDetector {
    http_service: Option<SimilarityService>,
    cli_detector: Option<CliSemanticDetector>,
    fallback_enabled: bool,
}

impl SemanticDetector {
    pub fn new() -> Self {
        let service_url = std::env::var("SIMILARITY_SERVICE_URL").unwrap_or_default();
        let force_cli = service_url.eq_ignore_ascii_case("cli");

        let http_service = if force_cli {
            tracing::info!("SIMILARITY_SERVICE_URL=cli; using CLI detector only");
            None
        } else {
            match SimilarityService::new() {
                Ok(service) => {
                    tracing::info!("Using persistent HTTP similarity service");
                    Some(service)
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to initialize similarity HTTP service ({}); will use CLI fallback if available",
                        e
                    );
                    None
                }
            }
        };

        let cli_detector = std::env::var("SEMANTIC_DETECTOR_PATH")
            .ok()
            .map(CliSemanticDetector::new);

        if http_service.is_none() && cli_detector.is_none() {
            tracing::error!(
                "No similarity detector configured. Set SIMILARITY_SERVICE_URL or SEMANTIC_DETECTOR_PATH."
            );
        }

        Self {
            http_service,
            cli_detector,
            fallback_enabled: !force_cli,
        }
    }

    /// Detect if an advertisement appears in a program transcription.
    pub async fn detect_advertisement(
        &self,
        program_text: &str,
        ad_text: &str,
    ) -> Result<SemanticDetectionResult> {
        if let Some(service) = &self.http_service {
            let result = service.detect_advertisement(program_text, ad_text).await;
            match result {
                Ok(detection) => return Ok(detection),
                Err(e) => {
                    if self.cli_detector.is_some() && self.fallback_enabled {
                        tracing::warn!(
                            "Similarity HTTP service request failed ({}); falling back to CLI detector",
                            e
                        );
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        if let Some(cli) = &self.cli_detector {
            return cli.detect_advertisement(program_text, ad_text).await;
        }

        anyhow::bail!("No similarity detector backend available")
    }
}

impl Default for SemanticDetector {
    fn default() -> Self {
        Self::new()
    }
}
