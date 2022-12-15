use std::path::PathBuf;

use common_lib::settings::{ClientSettings, ServerSettings};
use tauri::{api::dialog::blocking::FileDialogBuilder, async_runtime::RwLock};

use crate::ClientState;

const SETTINGS_FILE_PATH: &str = "ClientSettings.toml";

pub async fn read_settings_file() -> ClientSettings {
    match tokio::fs::read_to_string(SETTINGS_FILE_PATH).await {
        Ok(s) => toml::from_str(&s).expect("Error reading settings"),
        Err(_) => Default::default(),
    }
}

async fn write_settings_file(client_settings: &ClientSettings) -> std::io::Result<()> {
    let s = toml::to_string(client_settings).unwrap();
    tokio::fs::write(SETTINGS_FILE_PATH, s).await?;
    Ok(())
}

#[tauri::command]
pub async fn get_client_settings(
    state: tauri::State<'_, RwLock<ClientState>>,
) -> Result<ClientSettings, ()> {
    Ok(state.read().await.client_settings.clone())
}

#[tauri::command]
pub async fn set_client_settings(
    state: tauri::State<'_, RwLock<ClientState>>,
    client_settings: ClientSettings,
) -> Result<(), String> {
    write_settings_file(&client_settings)
        .await
        .map_err(|e| e.to_string())?;
    state.write().await.client_settings = client_settings;
    Ok(())
}

#[tauri::command]
pub async fn get_server_settings(
    state: tauri::State<'_, RwLock<ClientState>>,
) -> Result<ServerSettings, String> {
    let mut settings_url = state.read().await.client_settings.indexer_url.clone();
    settings_url.set_path("settings");
    let req_builder = state.read().await.reqwest_client.get(settings_url);
    let server_settings: ServerSettings = req_builder
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    Ok(server_settings)
}

#[tauri::command]
pub async fn set_server_settings(
    state: tauri::State<'_, RwLock<ClientState>>,
    server_settings: ServerSettings,
) -> Result<(), String> {
    let mut settings_url = state.read().await.client_settings.indexer_url.clone();
    settings_url.set_path("settings");
    let req_builder = state.read().await.reqwest_client.put(settings_url);
    req_builder
        .json(&server_settings)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let mut tmp = state.write().await;
    tmp.server_settings = server_settings;
    tmp.update_es();
    Ok(())
}

#[tauri::command]
pub async fn pick_folder() -> Option<PathBuf> {
    FileDialogBuilder::new().pick_folder()
}
