use serde::{Deserialize, Serialize};

/// Result returned by all semantic similarity detectors (HTTP service or CLI fallback).
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

/// Generic error shape returned by Python scripts and services.
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}
