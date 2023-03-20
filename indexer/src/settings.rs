use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use common_lib::settings::Settings;
use tracing_unwrap::ResultExt;

use crate::{watcher::start_watcher, ServerState};

const SETTINGS_FILE_PATH: &str = "Settings.toml";

pub async fn read_settings_file() -> Settings {
    match tokio::fs::read_to_string(SETTINGS_FILE_PATH).await {
        Ok(s) => toml::from_str(&s).expect_or_log("Error reading settings"),
        Err(e) => {
            tracing::warn!("Error reading settings file: {}, using defaults", e);
            Default::default()
        }
    }
}

async fn write_settings_file(state: Arc<ServerState>) -> std::io::Result<()> {
    let s = toml::to_string(&*state.settings.read().await).unwrap_or_log();
    tokio::fs::write(SETTINGS_FILE_PATH, s).await?;
    Ok(())
}

/// Get current settings
pub async fn get_settings(State(state): State<Arc<ServerState>>) -> Json<Settings> {
    Json(state.settings.read().await.clone())
}

/// Set settings from JSON
pub async fn put_settings(
    State(state): State<Arc<ServerState>>,
    Json(new_settings): Json<Settings>,
) -> Result<(), (StatusCode, String)> {
    *state.settings.write().await = new_settings;
    start_watcher(Arc::clone(&state)).await;
    write_settings_file(state)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(())
}
