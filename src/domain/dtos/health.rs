use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthEntity {
    pub status: String,
    pub timestamp: String,
    pub database: DatabaseStatus,
    pub api: ApiStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseStatus {
    pub status: String,
    pub connection: bool,
    pub response_time_ms: Option<u128>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiStatus {
    pub status: String,
    pub uptime: String,
    pub version: String,
}

impl HealthEntity {
    pub fn new(
        status: String,
        timestamp: String,
        database: DatabaseStatus,
        api: ApiStatus,
    ) -> Self {
        Self {
            status,
            timestamp,
            database,
            api,
        }
    }
}

impl DatabaseStatus {
    pub fn new(status: String, connection: bool, response_time_ms: Option<u128>) -> Self {
        Self {
            status,
            connection,
            response_time_ms,
        }
    }
}

impl ApiStatus {
    pub fn new(status: String, uptime: String, version: String) -> Self {
        Self {
            status,
            uptime,
            version,
        }
    }
}
