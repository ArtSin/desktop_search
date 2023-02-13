use std::{path::PathBuf, sync::Arc, time::Duration};

use common_lib::{
    elasticsearch::ELASTICSEARCH_MAX_SIZE, indexer::IndexingStatus, settings::IndexingDirectory,
};
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult};
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tracing_unwrap::{OptionExt, ResultExt};

use crate::{indexer::indexing_process, scanner::process_indexable_files, ServerState};

pub async fn start_watcher(state: Arc<ServerState>) {
    tracing::info!("Starting watcher");

    let (tx, rx) = mpsc::unbounded_channel();
    let tmp = Arc::clone(&state);
    tokio::spawn(async { event_handler(tmp, rx).await });

    let mut debouncer = new_debouncer(
        Duration::from_secs_f32(state.settings.read().await.other.debouncer_timeout),
        None,
        move |e| {
            tx.send(e).unwrap_or_log();
        },
    )
    .expect_or_log("Can't start file system watcher");

    for path in process_indexable_files(
        &state.settings.read().await.other,
        &state.settings.read().await.other.indexing_directories,
        |_, path| Some(path),
        true,
        false,
    )
    .expect_or_log("Can't add paths to watcher")
    {
        if let Err(e) = debouncer
            .watcher()
            .watch(&path, RecursiveMode::NonRecursive)
        {
            tracing::warn!("Can't add path to watcher: {}", e);
        }
    }

    *state.watcher_debouncer.write().await = Some(debouncer);
}

async fn event_handler(
    state: Arc<ServerState>,
    mut watcher_rx: UnboundedReceiver<DebounceEventResult>,
) {
    let mut indexing_status = state.indexing_status.read().await.clone();
    let mut indexing_rx = state.indexing_events.subscribe();
    let mut paths = Some(Vec::new());

    let process_paths = |indexing_status: &IndexingStatus, paths: &mut Option<Vec<PathBuf>>| {
        if indexing_status.can_start() && (paths.is_none() || !paths.as_ref().unwrap().is_empty()) {
            let state_tmp = Arc::clone(&state);
            let paths_tmp = std::mem::replace(paths, Some(Vec::new()));
            tokio::spawn(async move {
                match paths_tmp {
                    Some(mut x) => {
                        x.sort_unstable();
                        x.dedup();

                        {
                            let mut tmp = state_tmp.watcher_debouncer.write().await;
                            let debouncer = tmp.as_mut().unwrap_or_log();
                            for path in process_indexable_files(
                                &state_tmp.settings.read().await.other,
                                &x.iter()
                                    .map(|path| IndexingDirectory {
                                        path: path.to_path_buf(),
                                        exclude: false,
                                        watch: true,
                                    })
                                    .collect::<Vec<_>>(),
                                |_, path| Some(path),
                                true,
                                true,
                            )
                            .expect_or_log("Can't add paths to watcher")
                            {
                                if let Err(e) = debouncer
                                    .watcher()
                                    .watch(&path, RecursiveMode::NonRecursive)
                                {
                                    tracing::warn!("Can't add path to watcher: {}", e);
                                }
                            }
                        }

                        indexing_process(Arc::clone(&state_tmp), Some(x)).await;
                    }
                    None => indexing_process(state_tmp, None).await,
                }
            });
        }
    };

    loop {
        tokio::select! {
            indexing_event = indexing_rx.recv() => {
                match indexing_event {
                    Ok(e) => indexing_status.process_event(e),
                    Err(_) => break,
                }
                process_paths(&indexing_status, &mut paths);
            },
            watch_event = watcher_rx.recv() => {
                match watch_event {
                    Some(e) => {
                        let mut curr_paths = match e {
                            Ok(x) => (x.len() <= ELASTICSEARCH_MAX_SIZE as usize).then(|| {
                                x.into_iter()
                                    .map(|event| event.path)
                                    .collect()
                            }),
                            Err(e) => {
                                tracing::warn!("File system watcher errors: {:#?}", e);
                                continue;
                            }
                        };

                        match (&mut paths, &mut curr_paths) {
                            (Some(x), Some(y)) => x.append(y),
                            (None, _) => {},
                            (_, None) => paths = None,
                        }
                        process_paths(&indexing_status, &mut paths);
                    }
                    None => break,
                }
            },
        }
    }
}
