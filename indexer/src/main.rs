use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
    time::Duration,
};

use axum::{error_handling::HandleErrorLayer, http::StatusCode, routing::get, BoxError, Router};
use elasticsearch::{http::transport::Transport, Elasticsearch};
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{
    filter::LevelFilter, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt,
    EnvFilter,
};
use tracing_unwrap::ResultExt;

use crate::settings::{read_settings_file, InternalServerSettings};

mod settings;

pub struct ServerState {
    settings: InternalServerSettings,
    es_client: Elasticsearch,
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

    let app = Router::new()
        .route(
            "/settings",
            get(settings::get_settings).put(settings::put_settings),
        )
        .with_state(Arc::new(RwLock::new(ServerState {
            settings,
            es_client,
        })))
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
                .timeout(Duration::from_secs(10))
                .layer(TraceLayer::new_for_http()),
        );
    tracing::info!("Listening on http://{}", address);

    axum::Server::bind(&address)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
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
