use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::process::Command;

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

#[derive(Debug, Serialize, Deserialize)]
struct ErrorResponse {
    error: String,
}

pub struct SemanticDetector {
    python_script_path: String,
}

impl SemanticDetector {
    pub fn new() -> Self {
        let script_path = std::env::var("SEMANTIC_DETECTOR_PATH")
            .expect("SEMANTIC_DETECTOR_PATH environment variable must be set");
        
        Self {
            python_script_path: script_path,
        }
    }

    /// Detect if an advertisement appears in a program similarity
    pub async fn detect_advertisement(
        &self,
        program_text: &str,
        ad_text: &str,
    ) -> Result<SemanticDetectionResult> {
        let total_start = std::time::Instant::now();
        tracing::trace!("Calling Python semantic detector");
        
        // Call Python script with program and ad text as arguments
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

        // Log stderr output (contains debug messages from Python)
        if !output.stderr.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Print each line from stderr
            for line in stderr.lines() {
                tracing::debug!("Python: {}", line);
            }
        }

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Python script failed: {}", stderr);
        }

        let parse_start = std::time::Instant::now();
        let stdout = String::from_utf8(output.stdout)
            .context("Failed to parse Python output as UTF-8")?;

        tracing::trace!("Python output: {}", stdout);

        // Try to parse as error first
        if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&stdout) {
            anyhow::bail!("Python script error: {}", error_response.error);
        }

        // Parse the JSON response
        let result: SemanticDetectionResult = serde_json::from_str(&stdout)
            .context("Failed to parse Python JSON response")?;
        let parse_duration = parse_start.elapsed();
        
        let total_duration = total_start.elapsed();

        tracing::trace!(
            "Semantic detection: match={}, score={}, total={:.2}ms (spawn={:.2}ms, python={:.2}ms, parse={:.2}ms)",
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

impl Default for SemanticDetector {
    fn default() -> Self {
        Self::new()
    }
}
