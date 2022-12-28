use std::{future::Future, sync::Arc};

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
    parser::parse_file,
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
                    },
                    "content_type": {
                        "type": "keyword"
                    },

                    "image_embedding": {
                        "type": "dense_vector",
                        "dims": 512,
                        "index": true,
                        "similarity": "dot_product"
                    },
                    "width": {
                        "type": "integer"
                    },
                    "height": {
                        "type": "integer"
                    },

                    "title": {
                        "type": "text"
                    },
                    "creator": {
                        "type": "text"
                    },
                    "doc_created": {
                        "type": "long"
                    },
                    "doc_modified": {
                        "type": "long"
                    },
                    "num_pages": {
                        "type": "integer"
                    },
                    "num_words": {
                        "type": "integer"
                    },
                    "num_characters": {
                        "type": "integer"
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
async fn streaming_process<T, F, Fut, FE>(
    state: Arc<RwLock<ServerState>>,
    tx: UnboundedSender<(Value, Value)>,
    files: Vec<T>,
    process: F,
    mut on_err: FE,
) where
    T: Send + 'static,
    F: Fn(Arc<RwLock<ServerState>>, T) -> Fut + Send + Sync + Copy + 'static,
    Fut: Future<Output = anyhow::Result<(Value, Value)>> + Send,
    FE: FnMut(anyhow::Error),
{
    const NNSERVER_BATCH_SIZE: usize = 32; // make into setting

    let semaphore = Arc::new(Semaphore::new(NNSERVER_BATCH_SIZE));
    let mut futures = Vec::new();
    for file in files {
        let permit = Arc::clone(&semaphore).acquire_owned().await.unwrap();
        let state = Arc::clone(&state);
        let tx = tx.clone();
        futures.push(tokio::spawn(async move {
            let res = process(state, file).await;
            drop(permit);
            tx.send(res?).unwrap();
            Ok::<(), _>(())
        }));
    }
    for f in futures {
        if let Err(e) = f.await.unwrap() {
            on_err(e);
        }
    }
}

/// Create operation to add new file to index
async fn add_new(
    state: Arc<RwLock<ServerState>>,
    file: FileInfo,
) -> anyhow::Result<(Value, Value)> {
    tracing::debug!("Add file: {}", file.path.display());

    let action = json!({"index": {}});
    let mut file_es: FileES = file.try_into().unwrap_or_log();
    parse_file(state, &mut file_es).await?;
    let data = serde_json::to_value(file_es).unwrap_or_log();
    Ok((action, data))
}

/// Create operation to update file in index given old and new file info
async fn update_modified(
    state: Arc<RwLock<ServerState>>,
    (old_file, new_file): (FileInfo, FileInfo),
) -> anyhow::Result<(Value, Value)> {
    tracing::debug!("Update file: {}", new_file.path.display());

    let action = json!({"index": { "_id": old_file._id.unwrap() }});
    let mut new_file_es: FileES = new_file.try_into().unwrap_or_log();
    parse_file(state, &mut new_file_es).await?;
    let data = serde_json::to_value(new_file_es).unwrap_or_log();
    Ok((action, data))
}

/// Create operation to remove file from index
async fn remove_old(
    _state: Arc<RwLock<ServerState>>,
    file: FileInfo,
) -> anyhow::Result<(Value, Value)> {
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

    state.write().await.indexing_status = IndexingStatus::Indexing;
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
        streaming_process(
            Arc::clone(&state),
            tx.clone(),
            diff.added,
            add_new,
            &mut on_err,
        )
        .await;
        streaming_process(
            Arc::clone(&state),
            tx.clone(),
            diff.modified,
            update_modified,
            &mut on_err,
        )
        .await;
        streaming_process(
            Arc::clone(&state),
            tx,
            diff.removed,
            remove_old,
            &mut on_err,
        )
        .await;
        if let Err(e) = bulk_send_f.await.unwrap_or_log() {
            on_err(e.into());
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
