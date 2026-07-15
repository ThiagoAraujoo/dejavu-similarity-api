use crate::utils::errors::AppError;
use crate::domain::dtos::{ApiStatus, DatabaseStatus, HealthEntity};
use chrono::Utc;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

#[derive(Clone)]
pub struct HealthUseCase {
    db: Arc<DatabaseConnection>,
    start_time: SystemTime,
}

impl HealthUseCase {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            start_time: SystemTime::now(),
        }
    }

    pub async fn check_health(&self) -> Result<HealthEntity, AppError> {
        let timestamp = Utc::now().to_rfc3339();
        
        // Check database connection
        let database_status = self.check_database_connection().await;
        
        // Check API status
        let api_status = self.check_api_status();
        
        // Overall status
        let overall_status = if database_status.connection {
            "healthy".to_string()
        } else {
            "unhealthy".to_string()
        };

        Ok(HealthEntity::new(
            overall_status,
            timestamp,
            database_status,
            api_status,
        ))
    }

    async fn check_database_connection(&self) -> DatabaseStatus {
        let start = Instant::now();
        
        match self.db.execute(Statement::from_string(
            sea_orm::DatabaseBackend::MySql,
            "SELECT 1".to_string(),
        )).await {
            Ok(_) => {
                let response_time = start.elapsed().as_millis();
                DatabaseStatus::new(
                    "connected".to_string(),
                    true,
                    Some(response_time),
                )
            }
            Err(_) => {
                DatabaseStatus::new(
                    "disconnected".to_string(),
                    false,
                    None,
                )
            }
        }
    }

    fn check_api_status(&self) -> ApiStatus {
        let uptime = match self.start_time.elapsed() {
            Ok(duration) => self.format_duration(duration),
            Err(_) => "unknown".to_string(),
        };

        ApiStatus::new(
            "running".to_string(),
            uptime,
            env!("CARGO_PKG_VERSION").to_string(),
        )
    }

    fn format_duration(&self, duration: Duration) -> String {
        let total_seconds = duration.as_secs();
        let days = total_seconds / 86400;
        let hours = (total_seconds % 86400) / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;

        if days > 0 {
            format!("{}d {}h {}m {}s", days, hours, minutes, seconds)
        } else if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }
}
