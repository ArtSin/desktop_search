#![recursion_limit = "256"]

use std::{sync::Arc, time::Duration};

use axum::{
    error_handling::HandleErrorLayer,
    http::StatusCode,
    routing::{get, post},
    BoxError, Router,
};
use common_lib::{
    indexer::{IndexingEvent, IndexingStatus},
    settings::Settings,
};
use elasticsearch::{http::transport::Transport, Elasticsearch};
use notify::RecommendedWatcher;
use notify_debouncer_mini::Debouncer;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use tokio::{
    signal,
    sync::{broadcast, RwLock},
};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{
    filter::LevelFilter, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};
use tracing_unwrap::ResultExt;

use crate::{
    indexer::create_index::create_index, settings::read_settings_file, watcher::start_watcher,
};

mod actions;
mod embeddings;
mod file_server;
mod indexer;
mod parser;
mod scanner;
mod search;
mod settings;
mod thumbnails;
mod watcher;

pub struct ServerState {
    settings: RwLock<Settings>,
    es_client: Elasticsearch,
    reqwest_client: reqwest_middleware::ClientWithMiddleware,
    indexing_status: RwLock<IndexingStatus>,
    indexing_events: broadcast::Sender<IndexingEvent>,
    watcher_debouncer: RwLock<Option<Debouncer<RecommendedWatcher>>>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env_lossy(),
        )
        .init();

    let settings = read_settings_file().await;

    let es_transport = Transport::single_node(settings.elasticsearch_url.as_str())
        .expect_or_log("Can't create connection to Elasticsearch");
    let es_client = Elasticsearch::new(es_transport);
    create_index(&es_client)
        .await
        .expect_or_log("Can't create Elasticsearch index");

    let address = settings.indexer_address;
    let open_on_start = settings.open_on_start;
    let indexing_events_channel_capacity = 2 * settings.max_concurrent_files;

    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    let reqwest_client = reqwest_middleware::ClientBuilder::new(
        reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_log(),
    )
    .with(RetryTransientMiddleware::new_with_policy(retry_policy))
    .build();

    let server_state = Arc::new(ServerState {
        settings: RwLock::new(settings),
        es_client,
        reqwest_client,
        indexing_status: RwLock::new(IndexingStatus::NotStarted),
        indexing_events: broadcast::channel(indexing_events_channel_capacity).0,
        watcher_debouncer: RwLock::new(None),
    });

    start_watcher(Arc::clone(&server_state)).await;

    let app = Router::new()
        .route(
            "/settings",
            get(settings::get_settings).put(settings::put_settings),
        )
        .route(
            "/index",
            get(indexer::status::indexing_status)
                .patch(indexer::index)
                .delete(indexer::delete_index),
        )
        .route("/search", post(search::search))
        .route("/open_path", post(actions::open_path))
        .route("/pick_file", post(actions::pick_file))
        .route("/pick_folder", post(actions::pick_folder))
        .route("/open_request", post(actions::open_request))
        .route("/save_request", post(actions::save_request))
        .route("/file", get(file_server::get_file))
        .route("/document_content", get(file_server::get_document_content))
        .route(
            "/client_translation",
            get(file_server::get_client_translation),
        )
        .fallback(file_server::get_client_file)
        .with_state(server_state)
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|error: BoxError| async move {
                    if error.is::<tower::timeout::error::Elapsed>() {
                        Ok(StatusCode::REQUEST_TIMEOUT)
                    } else {
                        Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Unhandled internal error: {error}"),
                        ))
                    }
                }))
                .timeout(Duration::MAX)
                .layer(TraceLayer::new_for_http()),
        );
    let url = format!("http://{address}");
    tracing::info!("Listening on {}", url);
    if open_on_start {
        open::that(url).expect_or_log("Can't open server URL");
    }

    axum::Server::bind(&address)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap_or_log();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect_or_log("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect_or_log("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Signal received, starting graceful shutdown");
}
