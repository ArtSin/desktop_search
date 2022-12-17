use common_lib::{elasticsearch::ELASTICSEARCH_INDEX, status::IndexStats, IndexingStatus};
use elasticsearch::{indices::IndicesStatsParts, Elasticsearch};
use serde_json::Value;
use tauri::async_runtime::RwLock;

use crate::ClientState;

async fn get_es_response(es_client: &Elasticsearch) -> Result<Value, elasticsearch::Error> {
    es_client
        .indices()
        .stats(IndicesStatsParts::Metric(&["docs", "store"]))
        .send()
        .await?
        .json::<Value>()
        .await
}

#[tauri::command]
pub async fn get_indexing_status(
    state: tauri::State<'_, RwLock<ClientState>>,
) -> Result<IndexingStatus, String> {
    let mut index_url = state.read().await.client_settings.indexer_url.clone();
    index_url.set_path("index");
    let req_builder = state.read().await.reqwest_client.get(index_url);
    let indexing_status: IndexingStatus = req_builder
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    Ok(indexing_status)
}

#[tauri::command]
pub async fn get_index_stats(
    state: tauri::State<'_, RwLock<ClientState>>,
) -> Result<IndexStats, String> {
    let es_response_body = &get_es_response(&state.read().await.es_client)
        .await
        .map_err(|e| e.to_string())?["indices"][ELASTICSEARCH_INDEX];

    Ok(IndexStats {
        doc_cnt: es_response_body["total"]["docs"]["count"].as_u64().unwrap(),
        index_size: es_response_body["total"]["store"]["size_in_bytes"]
            .as_u64()
            .unwrap(),
    })
}

#[tauri::command]
pub async fn index(state: tauri::State<'_, RwLock<ClientState>>) -> Result<(), String> {
    let mut index_url = state.read().await.client_settings.indexer_url.clone();
    index_url.set_path("index");
    let req_builder = state.read().await.reqwest_client.patch(index_url);
    req_builder
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    Ok(())
}
