use std::{future::Future, sync::Arc};

use axum::{extract::State, http::StatusCode};
use common_lib::{
    elasticsearch::{FileES, ELASTICSEARCH_INDEX},
    indexer::IndexingEvent,
};
use elasticsearch::{
    http::request::JsonBody,
    indices::{IndicesCreateParts, IndicesExistsParts, IndicesRefreshParts},
    BulkParts, Elasticsearch,
};
use serde_json::{json, Value};
use tokio::sync::{
    mpsc::{self, UnboundedReceiver, UnboundedSender},
    Semaphore,
};
use tracing_unwrap::ResultExt;

use crate::{
    parser::parse_file,
    scanner::{get_elasticsearch_files_list, get_file_system_files_list, FileInfo, FilesDiff},
    ServerState,
};

pub mod status;

/// Creates index for storing indexed files, if it doesn't exist
pub async fn create_index(es_client: &Elasticsearch) -> Result<(), elasticsearch::Error> {
    // Check if index exists
    if es_client
        .indices()
        .exists(IndicesExistsParts::Index(&[ELASTICSEARCH_INDEX]))
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
            "settings": {
                "index": {
                    "analysis": {
                        "filter": {
                            "english_stemmer": {
                                "type": "stemmer",
                                "name": "english"
                            },
                            "russian_stemmer": {
                                "type": "stemmer",
                                "name": "russian"
                            },
                            "english_stop": {
                                "type": "stop",
                                "stopwords": "_english_"
                            },
                            "russian_stop": {
                                "type": "stop",
                                "stopwords": "_russian_"
                            }
                        },
                        "analyzer": {
                            "en_ru_analyzer": {
                                "tokenizer": "standard",
                                "filter": [
                                    "lowercase",
                                    "english_stemmer",
                                    "russian_stemmer",
                                    "english_stop",
                                    "russian_stop"
                                ]
                            },
                            "path_en_ru_analyzer": {
                                "tokenizer": "letter",
                                "filter": [
                                    "lowercase",
                                    "english_stemmer",
                                    "russian_stemmer"
                                ]
                            }
                        }
                    }
                }
            },
            "mappings": {
                "properties": {
                    "path": {
                        "type": "text",
                        "analyzer": "path_en_ru_analyzer"
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
                    "content": {
                        "type": "text",
                        "analyzer": "en_ru_analyzer"
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
                        "type": "text",
                        "analyzer": "en_ru_analyzer"
                    },
                    "creator": {
                        "type": "text",
                        "analyzer": "en_ru_analyzer"
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

/// Update indexing status and send event to channel
async fn on_event(state: Arc<ServerState>, event: IndexingEvent) {
    match &event {
        IndexingEvent::Started => tracing::info!("Indexing started"),
        IndexingEvent::DiffCalculated { .. } => tracing::info!("Difference calculated"),
        IndexingEvent::Error(e) => tracing::error!("Error while indexing: {}", e),
        IndexingEvent::Finished => tracing::info!("Indexing finished"),
        _ => {}
    }
    state
        .indexing_status
        .write()
        .await
        .process_event(event.clone());

    #[allow(unused_must_use)]
    {
        state.indexing_events.send(event);
    }
}

/// Process all files with given function and send results to channel, call function on each event.
/// Processing is parallel with no more than given number of tasks at once
async fn streaming_process<T, F, Fut>(
    state: Arc<ServerState>,
    tx: UnboundedSender<(Value, Value)>,
    files: Vec<T>,
    process: F,
) where
    T: Send + 'static,
    F: Fn(Arc<ServerState>, T) -> Fut + Send + Sync + Copy + 'static,
    Fut: Future<Output = anyhow::Result<(Value, Value)>> + Send,
{
    let semaphore = Arc::new(Semaphore::new(
        state.settings.read().await.other.nnserver_batch_size,
    ));
    let mut futures = Vec::new();
    for file in files {
        let permit = Arc::clone(&semaphore).acquire_owned().await.unwrap();
        let state = Arc::clone(&state);
        let tx = tx.clone();
        futures.push(tokio::spawn(async move {
            let res = process(Arc::clone(&state), file).await;
            tx.send(res?).unwrap();
            on_event(state, IndexingEvent::FileProcessed).await;
            drop(permit);
            Ok::<(), anyhow::Error>(())
        }));
    }
    for f in futures {
        if let Err(e) = f.await.unwrap() {
            on_event(Arc::clone(&state), IndexingEvent::Error(e.to_string())).await;
        }
    }
}

/// Create operation to add new file to index
async fn add_new(state: Arc<ServerState>, file: FileInfo) -> anyhow::Result<(Value, Value)> {
    tracing::debug!("Add file: {}", file.path.display());

    let action = json!({"index": {}});
    let mut file_es: FileES = file.try_into().unwrap_or_log();
    parse_file(state, &mut file_es).await?;
    let data = serde_json::to_value(file_es).unwrap_or_log();
    Ok((action, data))
}

/// Create operation to update file in index given old and new file info
async fn update_modified(
    state: Arc<ServerState>,
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
async fn remove_old(_state: Arc<ServerState>, file: FileInfo) -> anyhow::Result<(Value, Value)> {
    tracing::debug!("Remove file: {}", file.path.display());

    let action = json!({"delete": { "_id": file._id.unwrap() }});
    Ok((action, Value::Null))
}

/// Accept operations from channel and bulk send them to Elasticsearch
async fn bulk_send(
    state: Arc<ServerState>,
    mut rx: UnboundedReceiver<(Value, Value)>,
) -> Result<(), elasticsearch::Error> {
    async fn send_queue(
        es_client: &Elasticsearch,
        queue: &mut Vec<JsonBody<Value>>,
    ) -> Result<(), elasticsearch::Error> {
        tracing::debug!("Bulk send {} lines", queue.len());
        let body = std::mem::take(queue);
        es_client
            .bulk(BulkParts::Index(ELASTICSEARCH_INDEX))
            .body(body)
            .send()
            .await?;
        Ok(())
    }

    let mut queue = Vec::new();
    let mut cnt: usize = 0;
    let batch_size = state.settings.read().await.other.elasticsearch_batch_size;
    while let Some((action, data)) = rx.recv().await {
        queue.push(JsonBody::new(action));
        if !data.is_null() {
            queue.push(JsonBody::new(data));
        }
        cnt += 1;

        if cnt >= batch_size {
            send_queue(&state.es_client, &mut queue).await?;
            on_event(Arc::clone(&state), IndexingEvent::FilesSent(cnt)).await;
            cnt = 0;
        }
    }
    send_queue(&state.es_client, &mut queue).await?;
    on_event(state, IndexingEvent::FilesSent(cnt)).await;
    Ok(())
}

/// Start indexing files
pub async fn index(State(state): State<Arc<ServerState>>) -> (StatusCode, String) {
    if !state.indexing_status.read().await.can_start() {
        return (StatusCode::BAD_REQUEST, "Already indexing".to_owned());
    }

    on_event(Arc::clone(&state), IndexingEvent::Started).await;

    tokio::spawn(async move {
        // Get files lists from file system and Elasticsearch
        let tmp = Arc::clone(&state);
        let file_system_files_f = tokio::task::spawn_blocking(move || {
            get_file_system_files_list(&tmp.settings.blocking_read().other)
        });

        let elasticsearch_files_f = get_elasticsearch_files_list(&state.es_client);

        let (file_system_files, elasticsearch_files) =
            tokio::join!(file_system_files_f, elasticsearch_files_f);

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
        on_event(
            Arc::clone(&state),
            IndexingEvent::DiffCalculated {
                to_add: diff.added.len(),
                to_remove: diff.removed.len(),
                to_update: diff.modified.len(),
            },
        )
        .await;

        // Create channel to bulk send operations to Elasticsearch
        let tmp = Arc::clone(&state);
        let (tx, rx) = mpsc::unbounded_channel();
        let bulk_send_f = tokio::spawn(async move { bulk_send(tmp, rx).await });

        // Process differences and send operations to channel
        streaming_process(Arc::clone(&state), tx.clone(), diff.added, add_new).await;
        streaming_process(
            Arc::clone(&state),
            tx.clone(),
            diff.modified,
            update_modified,
        )
        .await;
        streaming_process(Arc::clone(&state), tx, diff.removed, remove_old).await;
        if let Err(e) = bulk_send_f.await.unwrap_or_log() {
            on_event(Arc::clone(&state), IndexingEvent::Error(e.to_string())).await;
        }

        // Finish indexing
        if let Err(e) = state
            .es_client
            .indices()
            .refresh(IndicesRefreshParts::Index(&[ELASTICSEARCH_INDEX]))
            .send()
            .await
        {
            on_event(Arc::clone(&state), IndexingEvent::Error(e.to_string())).await;
        }
        on_event(Arc::clone(&state), IndexingEvent::Finished).await;
    });

    (StatusCode::ACCEPTED, String::new())
}
