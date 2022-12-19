#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::time::Duration;

use common_lib::settings::{ClientSettings, ServerSettings};
use elasticsearch::{http::transport::Transport, Elasticsearch};
use settings::read_settings_file;
use tauri::async_runtime::RwLock;

mod assets;
mod search;
mod settings;
mod status;

pub struct ClientState {
    client_settings: ClientSettings,
    server_settings: ServerSettings,
    es_client: Elasticsearch,
    reqwest_client: reqwest::Client,
}

impl ClientState {
    async fn new() -> Self {
        let server_settings = ServerSettings::default();
        let es_transport = Transport::single_node(server_settings.elasticsearch_url.as_str())
            .expect("Can't create connection to Elasticsearch");
        Self {
            client_settings: read_settings_file().await,
            server_settings,
            es_client: Elasticsearch::new(es_transport),
            reqwest_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
        }
    }

    fn update_es(&mut self) {
        let es_transport = Transport::single_node(self.server_settings.elasticsearch_url.as_str())
            .expect("Can't create connection to Elasticsearch");
        self.es_client = Elasticsearch::new(es_transport);
    }
}

#[tokio::main]
async fn main() {
    tauri::async_runtime::set(tokio::runtime::Handle::current());
    tauri::Builder::default()
        .manage(RwLock::new(ClientState::new().await))
        .register_uri_scheme_protocol("localfile", assets::get_local_file)
        .invoke_handler(tauri::generate_handler![
            search::search,
            status::get_indexing_status,
            status::get_index_stats,
            status::index,
            settings::get_client_settings,
            settings::set_client_settings,
            settings::get_server_settings,
            settings::set_server_settings,
            settings::pick_folder
        ])
        .run(tauri::generate_context!())
        .expect("Error while running tauri application");
}
