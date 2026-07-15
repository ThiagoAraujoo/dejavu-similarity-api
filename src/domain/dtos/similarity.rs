use serde::{Deserialize, Serialize};
use validator::Validate;
use crate::core::export::Exportable;
use crate::infrastructure::entities::similarity::SimilarityStatus;

use super::{AdvertisementEntity, FileQueueEntity};

#[derive(Debug, Deserialize, Validate)]
pub struct CreateSimilarityDto {
    #[validate(range(min = 1, message = "file_queue_id is required"))]
    pub file_queue_id: i32,
    #[validate(range(min = 1, message = "song_id is required"))]
    pub song_id: i32,
    pub song_start: f64,
    pub song_end: f64,
    pub score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<SimilarityStatus>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateSimilarityDto {
    #[validate(range(min = 1))]
    pub file_queue_id: Option<i32>,
    #[validate(range(min = 1))]
    pub song_id: Option<i32>,
    pub song_start: Option<f64>,
    pub song_end: Option<f64>,
    pub score: Option<f64>,
    pub status: Option<SimilarityStatus>,
}

#[derive(Debug, Serialize, Clone)]
pub struct SimilarityEntity {
    pub id: i32,
    pub file_queue_id: i32,
    pub song_id: i32,
    pub song_start: f64,
    pub song_end: f64,
    pub score: f64,
    pub status: SimilarityStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emissora_nome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub song_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub horario: Option<chrono::NaiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_queue: Option<FileQueueEntity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub song: Option<AdvertisementEntity>,
}

impl Default for UpdateSimilarityDto {
    fn default() -> Self {
        Self {
            file_queue_id: None,
            song_id: None,
            song_start: None,
            song_end: None,
            score: None,
            status: None,
        }
    }
}

fn score_as_percent(score: f64) -> f64 {
    if score.is_finite() && (0.0..=1.0).contains(&score) {
        score * 100.0
    } else {
        score
    }
}

fn similarity_level(score: f64) -> &'static str {
    if !score.is_finite() {
        return "-";
    }
    if score >= 90.0 {
        "Alto"
    } else if score >= 85.0 {
        "Medio"
    } else if score >= 80.0 {
        "Baixo"
    } else {
        "-"
    }
}

impl Exportable for SimilarityEntity {
    fn headers() -> Vec<String> {
        vec![
            "Codigo".to_string(),
            "Emissora".to_string(),
            "Horario".to_string(),
            "Propaganda".to_string(),
            "Score".to_string(),
            "Similaridade".to_string(),
        ]
    }

    fn to_row(&self) -> Vec<String> {
        let score_percent = score_as_percent(self.score);
        let emissora = self
            .file_queue
            .as_ref()
            .and_then(|f| f.emissora.as_ref())
            .and_then(|e| e.nome.clone())
            .unwrap_or_else(|| "-".to_string());
        let horario = self
            .file_queue
            .as_ref()
            .map(|f| f.horario.format("%d/%m/%Y %H:%M").to_string())
            .unwrap_or_else(|| "-".to_string());
        let propaganda = self
            .song
            .as_ref()
            .and_then(|s| s.song_name.clone())
            .map(|s| s.to_uppercase())
            .unwrap_or_else(|| "-".to_string());
        let score_display = if score_percent.is_finite() {
            format!("{:.2}%", score_percent)
        } else {
            "-".to_string()
        };

        vec![
            self.id.to_string(),
            emissora,
            horario,
            propaganda,
            score_display,
            similarity_level(score_percent).to_string(),
        ]
    }
}
