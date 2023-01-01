use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    error_handling::HandleErrorLayer,
    http::StatusCode,
    routing::{get, post},
    BoxError, Router,
};
use common_lib::indexer::{IndexingEvent, IndexingStatus};
use elasticsearch::{http::transport::Transport, Elasticsearch};
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
    indexer::create_index,
    settings::{read_settings_file, InternalSettings},
};

mod actions;
mod embeddings;
mod file_server;
mod indexer;
mod parser;
mod scanner;
mod search;
mod settings;

pub struct ServerState {
    settings: RwLock<InternalSettings>,
    es_client: Elasticsearch,
    reqwest_client: reqwest::Client,
    indexing_status: RwLock<IndexingStatus>,
    indexing_events: broadcast::Sender<IndexingEvent>,
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

    let address: SocketAddr = settings
        .address
        .parse()
        .expect_or_log("Can't parse address");

    let es_transport = Transport::single_node(settings.other.elasticsearch_url.as_str())
        .expect_or_log("Can't create connection to Elasticsearch");
    let es_client = Elasticsearch::new(es_transport);
    create_index(&es_client)
        .await
        .expect_or_log("Can't create Elasticsearch index");

    let app = Router::new()
        .route(
            "/settings",
            get(settings::get_settings).put(settings::put_settings),
        )
        .route(
            "/index",
            get(indexer::status::indexing_status).patch(indexer::index),
        )
        .route("/search", post(search::search))
        .route("/open_path", post(actions::open_path))
        .route("/pick_file", post(actions::pick_file))
        .route("/pick_folder", post(actions::pick_folder))
        .route("/file", get(file_server::get_file))
        .fallback(file_server::get_client_file)
        .with_state(Arc::new(ServerState {
            settings: RwLock::new(settings),
            es_client,
            reqwest_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
            indexing_status: RwLock::new(IndexingStatus::NotStarted),
            indexing_events: broadcast::channel(64).0, // TODO: capacity from setting
        }))
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|error: BoxError| async move {
                    if error.is::<tower::timeout::error::Elapsed>() {
                        Ok(StatusCode::REQUEST_TIMEOUT)
                    } else {
                        Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Unhandled internal error: {}", error),
                        ))
                    }
                }))
                .timeout(Duration::MAX)
                .layer(TraceLayer::new_for_http()),
        );
    let url = format!("http://{}", address);
    tracing::info!("Listening on {}", url);
    open::that(url).expect_or_log("Can't open server URL"); // make a setting

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
