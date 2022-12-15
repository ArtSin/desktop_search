use common_lib::{elasticsearch::ELASTICSEARCH_INDEX, status::IndexStats};
use elasticsearch::{indices::IndicesStatsParts, Elasticsearch};
use serde_json::Value;
use tauri::async_runtime::RwLock;

use crate::ClientState;

async fn get_es_response(es_client: &Elasticsearch) -> Result<Value, elasticsearch::Error> {
    es_client
        .indices()
        .stats(IndicesStatsParts::Metric(&["docs"]))
        .send()
        .await?
        .json::<Value>()
        .await
}

#[tauri::command]
pub async fn get_index_stats(
    state: tauri::State<'_, RwLock<ClientState>>,
) -> Result<IndexStats, String> {
    let es_response_body = get_es_response(&state.read().await.es_client)
        .await
        .map_err(|e| e.to_string())?;

    Ok(IndexStats {
        doc_cnt: es_response_body["indices"][ELASTICSEARCH_INDEX]["total"]["docs"]["count"]
            .as_u64()
            .unwrap(),
    })
}
