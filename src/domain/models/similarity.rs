use serde::{Deserialize, Serialize};
use crate::infrastructure::entities::similarity::SimilarityStatus;
use chrono::NaiveDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Similarity {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,
    pub file_queue_id: i32,
    pub song_id: i32,
    pub song_start: f64,
    pub song_end: f64,
    pub score: f64,
    pub status: SimilarityStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityWithRelations {
    pub id: i32,
    pub file_queue_id: i32,
    pub song_id: i32,
    pub song_start: f64,
    pub song_end: f64,
    pub score: f64,
    pub status: SimilarityStatus,
    pub emissora_nome: Option<String>,
    pub song_name: Option<String>,
    pub horario: Option<NaiveDateTime>,
}

impl Similarity {
    pub fn new(file_queue_id: i32, song_id: i32, song_start: f64, song_end: f64, score: f64) -> Self {
        Self {
            id: None,
            file_queue_id,
            song_id,
            song_start,
            song_end,
            score,
            status: SimilarityStatus::Pending,
        }
    }
}
