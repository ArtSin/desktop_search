use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing_unwrap::ResultExt;

use common_lib::settings::ServerSettings;

use crate::ServerState;

const SETTINGS_FILE_PATH: &str = "IndexerSettings.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InternalServerSettings {
    pub address: String,
    #[serde(flatten)]
    pub other: ServerSettings,
}

impl Default for InternalServerSettings {
    fn default() -> Self {
        Self {
            address: "127.0.0.1:11000".to_owned(),
            other: Default::default(),
        }
    }
}

pub async fn read_settings_file() -> InternalServerSettings {
    match tokio::fs::read_to_string(SETTINGS_FILE_PATH).await {
        Ok(s) => toml::from_str(&s).expect_or_log("Error reading settings"),
        Err(e) => {
            tracing::warn!("Error reading settings file: {}, using defaults", e);
            Default::default()
        }
    }
}

async fn write_settings_file(state: Arc<RwLock<ServerState>>) -> std::io::Result<()> {
    let s = toml::to_string(&state.read().await.settings).unwrap();
    tokio::fs::write(SETTINGS_FILE_PATH, s).await?;
    Ok(())
}

/// Get current settings
pub async fn get_settings(State(state): State<Arc<RwLock<ServerState>>>) -> Json<ServerSettings> {
    Json(state.read().await.settings.other.clone())
}

/// Set settings from JSON
pub async fn put_settings(
    State(state): State<Arc<RwLock<ServerState>>>,
    Json(new_settings): Json<ServerSettings>,
) -> Result<(), (StatusCode, String)> {
    {
        let mut state = state.write().await;
        state.settings.other = new_settings;
        state.update_es();
    }
    write_settings_file(state)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(())
}
