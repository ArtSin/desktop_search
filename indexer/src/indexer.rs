use std::{convert::Infallible, error::Error, future::Future, sync::Arc};

use axum::{extract::State, http::StatusCode, Json};
use common_lib::{
    elasticsearch::{FileES, ELASTICSEARCH_BATCH_SIZE, ELASTICSEARCH_INDEX},
    IndexingStatus,
};
use elasticsearch::{http::request::JsonBody, indices::IndicesCreateParts, Elasticsearch};
use serde_json::{json, Value};
use tokio::sync::{
    mpsc::{self, UnboundedReceiver, UnboundedSender},
    RwLock, Semaphore,
};
use tracing_unwrap::ResultExt;

use crate::{
    scanner::{get_elasticsearch_files_list, get_file_system_files_list, FileInfo, FilesDiff},
    ServerState,
};

/// Creates index for storing indexed files, if it doesn't exist
pub async fn create_index(es_client: &Elasticsearch) -> Result<(), elasticsearch::Error> {
    // Check if index exists
    if es_client
        .indices()
        .exists(elasticsearch::indices::IndicesExistsParts::Index(&[
            ELASTICSEARCH_INDEX,
        ]))
        .send()
        .await?
        .status_code()
        == StatusCode::OK
    {
        return Ok(());
    }

    // Create index and set mapping
    es_client
        .indices()
        .create(IndicesCreateParts::Index(ELASTICSEARCH_INDEX))
        .body(json!({
            "mappings": {
                "properties": {
                    "path": {
                        "type": "text"
                    },
                    "modified": {
                        "type": "long"
                    },
                    "size": {
                        "type": "long"
                    },
                    "hash": {
                        "type": "keyword"
                    }
                }
            }
        }))
        .send()
        .await?;
    Ok(())
}

/// Process all files with given function and send results to channel, call function on each error.
/// Processing is parallel with no more than given number of tasks at once
async fn streaming_process<T, F, E, Fut, FE>(
    tx: UnboundedSender<(Value, Value)>,
    files: Vec<T>,
    process: F,
    mut on_err: FE,
) where
    T: Send + 'static,
    F: Fn(T) -> Fut + Send + Sync + Copy + 'static,
    E: Error + Send + 'static,
    Fut: Future<Output = Result<(Value, Value), E>> + Send,
    FE: FnMut(Box<dyn Error>),
{
    const NNSERVER_BATCH_SIZE: usize = 32; // make into setting

    let semaphore = Arc::new(Semaphore::new(NNSERVER_BATCH_SIZE));
    let mut futures = Vec::new();
    for file in files {
        let permit = Arc::clone(&semaphore).acquire_owned().await.unwrap();
        let tx = tx.clone();
        futures.push(tokio::spawn(async move {
            let res = process(file).await;
            drop(permit);
            tx.send(res?).unwrap();
            Ok::<(), E>(())
        }));
    }
    for f in futures {
        if let Err(e) = f.await.unwrap() {
            on_err(Box::new(e));
        }
    }
}

/// Create operation to add new file to index
async fn add_new(file: FileInfo) -> Result<(Value, Value), Infallible> {
    tracing::debug!("Add file: {}", file.path.display());

    let action = json!({"index": {}});
    let file_es: FileES = file.try_into().unwrap_or_log();
    let data = serde_json::to_value(file_es).unwrap_or_log();
    Ok((action, data))
}

/// Create operation to update file in index given old and new file info
async fn update_modified(
    (old_file, new_file): (FileInfo, FileInfo),
) -> Result<(Value, Value), Infallible> {
    tracing::debug!("Update file: {}", new_file.path.display());

    let action = json!({"index": { "_id": old_file._id.unwrap() }});
    let new_file_es: FileES = new_file.try_into().unwrap_or_log();
    let data = serde_json::to_value(new_file_es).unwrap_or_log();
    Ok((action, data))
}

/// Create operation to remove file from index
async fn remove_old(file: FileInfo) -> Result<(Value, Value), Infallible> {
    tracing::debug!("Remove file: {}", file.path.display());

    let action = json!({"delete": { "_id": file._id.unwrap() }});
    Ok((action, Value::Null))
}

/// Accept operations from channel and bulk send them to Elasticsearch
async fn bulk_send(
    es_client: &Elasticsearch,
    mut rx: UnboundedReceiver<(Value, Value)>,
) -> Result<(), elasticsearch::Error> {
    async fn send_queue(
        es_client: &Elasticsearch,
        queue: &mut Vec<JsonBody<Value>>,
    ) -> Result<(), elasticsearch::Error> {
        tracing::debug!("Bulk send {} lines", queue.len());
        let body = std::mem::take(queue);
        es_client
            .bulk(elasticsearch::BulkParts::Index(ELASTICSEARCH_INDEX))
            .body(body)
            .send()
            .await?;
        Ok(())
    }

    let mut queue = Vec::new();
    let mut cnt: usize = 0;
    while let Some((action, data)) = rx.recv().await {
        queue.push(JsonBody::new(action));
        if !data.is_null() {
            queue.push(JsonBody::new(data));
        }
        cnt += 1;

        if cnt >= ELASTICSEARCH_BATCH_SIZE {
            send_queue(es_client, &mut queue).await?;
            cnt = 0;
        }
    }
    send_queue(es_client, &mut queue).await
}

/// Start indexing files
pub async fn index(State(state): State<Arc<RwLock<ServerState>>>) -> (StatusCode, String) {
    if !state.read().await.indexing_status.can_start() {
        return (StatusCode::BAD_REQUEST, "Already indexing".to_owned());
    }

    tokio::spawn(async move {
        // Get files lists from file system and Elasticsearch
        let tmp = Arc::clone(&state);
        let file_system_files_f = tokio::task::spawn_blocking(move || {
            get_file_system_files_list(&tmp.blocking_read().settings.other)
        });

        let tmp = state.read().await;
        let elasticsearch_files_f = get_elasticsearch_files_list(&tmp.es_client);

        let (file_system_files, elasticsearch_files) =
            tokio::join!(file_system_files_f, elasticsearch_files_f);
        drop(tmp);

        let file_system_files = file_system_files.unwrap_or_log();
        let elasticsearch_files = match elasticsearch_files {
            Ok(x) => x,
            Err(e) => {
                tracing::error!("Error reading file info from Elasticsearch: {}", e);
                return;
            }
        };

        // Calculate lists difference
        let diff = FilesDiff::from_vec(elasticsearch_files, file_system_files);

        // Create channel to bulk send operations to Elasticsearch
        let tmp = Arc::clone(&state);
        let (tx, rx) = mpsc::unbounded_channel();
        let bulk_send_f =
            tokio::spawn(async move { bulk_send(&tmp.read().await.es_client, rx).await });

        // Indexing result, which accumulates all errors
        let mut new_indexing_status = IndexingStatus::Finished;
        let mut on_err = |e| {
            tracing::error!("Error while indexing: {}", e);
            new_indexing_status.add_error(e);
        };

        // Process differences and send operations to channel
        streaming_process(tx.clone(), diff.added, add_new, &mut on_err).await;
        streaming_process(tx.clone(), diff.modified, update_modified, &mut on_err).await;
        streaming_process(tx, diff.removed, remove_old, &mut on_err).await;
        if let Err(e) = bulk_send_f.await.unwrap_or_log() {
            on_err(Box::new(e));
        }

        state.write().await.indexing_status = new_indexing_status;
        tracing::info!("Indexing finished");
    });

    (StatusCode::ACCEPTED, String::new())
}

pub async fn indexing_status(
    State(state): State<Arc<RwLock<ServerState>>>,
) -> Json<IndexingStatus> {
    Json(state.read().await.indexing_status.clone())
}
