#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::{path::PathBuf, sync::RwLock, time::Duration};

use common_lib::{ClientSettings, ServerSettings};
use tauri::api::dialog::blocking::FileDialogBuilder;

const SETTINGS_FILE_PATH: &str = "ClientSettings.toml";

struct ClientState {
    client_settings: ClientSettings,
    server_settings: ServerSettings,
    reqwest_client: reqwest::Client,
}

impl ClientState {
    async fn new() -> Self {
        Self {
            client_settings: read_settings_file().await,
            server_settings: ServerSettings::default(),
            reqwest_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
        }
    }
}

async fn read_settings_file() -> ClientSettings {
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
fn get_client_settings(state: tauri::State<'_, RwLock<ClientState>>) -> ClientSettings {
    state.read().unwrap().client_settings.clone()
}

#[tauri::command]
async fn set_client_settings(
    state: tauri::State<'_, RwLock<ClientState>>,
    client_settings: ClientSettings,
) -> Result<(), String> {
    write_settings_file(&client_settings)
        .await
        .map_err(|e| e.to_string())?;
    state.write().unwrap().client_settings = client_settings;
    Ok(())
}

#[tauri::command]
async fn get_server_settings(
    state: tauri::State<'_, RwLock<ClientState>>,
) -> Result<ServerSettings, String> {
    let mut settings_url = state.read().unwrap().client_settings.indexer_url.clone();
    settings_url.set_path("settings");
    let req_builder = state.read().unwrap().reqwest_client.get(settings_url);
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
async fn set_server_settings(
    state: tauri::State<'_, RwLock<ClientState>>,
    server_settings: ServerSettings,
) -> Result<(), String> {
    let mut settings_url = state.read().unwrap().client_settings.indexer_url.clone();
    settings_url.set_path("settings");
    let req_builder = state.read().unwrap().reqwest_client.put(settings_url);
    req_builder
        .json(&server_settings)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    state.write().unwrap().server_settings = server_settings;
    Ok(())
}

#[tauri::command]
async fn pick_folder() -> Option<PathBuf> {
    FileDialogBuilder::new().pick_folder()
}

#[tokio::main]
async fn main() {
    tauri::async_runtime::set(tokio::runtime::Handle::current());
    tauri::Builder::default()
        .manage(RwLock::new(ClientState::new().await))
        .invoke_handler(tauri::generate_handler![
            get_client_settings,
            set_client_settings,
            get_server_settings,
            set_server_settings,
            pick_folder
        ])
        .run(tauri::generate_context!())
        .expect("Error while running tauri application");
}
