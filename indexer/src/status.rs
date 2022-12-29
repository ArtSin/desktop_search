use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use common_lib::{elasticsearch::ELASTICSEARCH_INDEX, status::IndexStats};
use elasticsearch::{indices::IndicesStatsParts, Elasticsearch};
use serde_json::Value;
use tokio::sync::RwLock;

use crate::ServerState;

async fn get_es_response(es_client: &Elasticsearch) -> Result<Value, elasticsearch::Error> {
    es_client
        .indices()
        .stats(IndicesStatsParts::Metric(&["docs", "store"]))
        .send()
        .await?
        .json::<Value>()
        .await
}

// TODO: move into indexer
pub async fn get_index_stats(
    State(state): State<Arc<RwLock<ServerState>>>,
) -> Result<Json<IndexStats>, (StatusCode, String)> {
    let es_response_body = &get_es_response(&state.read().await.es_client)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?["indices"]
        [ELASTICSEARCH_INDEX];

    Ok(Json(IndexStats {
        doc_cnt: es_response_body["total"]["docs"]["count"].as_u64().unwrap(),
        index_size: es_response_body["total"]["store"]["size_in_bytes"]
            .as_u64()
            .unwrap(),
    }))
}
