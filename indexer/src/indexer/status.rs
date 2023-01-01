use std::sync::Arc;

use axum::{
    extract::{
        ws::{self, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
};
use common_lib::{
    elasticsearch::ELASTICSEARCH_INDEX,
    indexer::{IndexStats, IndexingEvent, IndexingWSMessage},
};
use elasticsearch::{indices::IndicesStatsParts, Elasticsearch};
use serde::Serialize;
use serde_json::Value;
use tokio::sync::broadcast;

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

async fn index_stats(es_client: &Elasticsearch) -> Result<IndexStats, elasticsearch::Error> {
    let es_response_body = &get_es_response(es_client).await?["indices"][ELASTICSEARCH_INDEX];

    Ok(IndexStats {
        doc_cnt: es_response_body["total"]["docs"]["count"].as_u64().unwrap(),
        index_size: es_response_body["total"]["store"]["size_in_bytes"]
            .as_u64()
            .unwrap(),
    })
}

pub async fn indexing_status(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ServerState>>,
) -> Response {
    ws.on_upgrade(|socket| indexing_status_ws(socket, state))
}

async fn indexing_status_ws(mut socket: WebSocket, state: Arc<ServerState>) {
    async fn send<T>(socket: &mut WebSocket, message: T) -> bool
    where
        T: Serialize,
        IndexingWSMessage: From<T>,
    {
        let event_json = serde_json::to_string(&IndexingWSMessage::from(message)).unwrap();
        socket.send(ws::Message::Text(event_json)).await.is_ok()
    }
    async fn send_indexing_status(socket: &mut WebSocket, state: &ServerState) -> bool {
        send(socket, state.indexing_status.read().await.clone()).await
    }
    async fn send_index_stats(socket: &mut WebSocket, state: &ServerState) -> bool {
        let stats_message: IndexingWSMessage = match index_stats(&state.es_client).await {
            Ok(res) => res.into(),
            Err(e) => e.to_string().into(),
        };
        send(socket, stats_message).await
    }

    if !send_indexing_status(&mut socket, &state).await {
        return;
    }
    if !send_index_stats(&mut socket, &state).await {
        return;
    }

    let mut rx = state.indexing_events.subscribe();
    loop {
        match rx.recv().await {
            Ok(event) => {
                if let IndexingEvent::Finished = event {
                    if !send_index_stats(&mut socket, &state).await {
                        return;
                    }
                }
                if !send(&mut socket, event).await {
                    return;
                }
            }
            Err(broadcast::error::RecvError::Lagged(_)) => {
                if !send_indexing_status(&mut socket, &state).await {
                    return;
                }
            }
            _ => return,
        }
    }
}
