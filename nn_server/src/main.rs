use std::{net::SocketAddr, time::Duration};

use axum::{
    error_handling::HandleErrorLayer, extract::DefaultBodyLimit, http::StatusCode, routing::post,
    BoxError, Router,
};
use clap::Parser;
use ndarray::{Array, ArrayD, Dimension};
use onnxruntime::{environment::Environment, LoggingLevel};
use serde::Serialize;
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{
    filter::LevelFilter, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};
use tracing_unwrap::ResultExt;

mod batch_processing;
mod clip_image;
mod clip_text;
mod lexrank;
mod minilm_rerank;
mod minilm_text;
mod text_processing;

const PATH_PREFIX: &str = "nn_server/";

#[derive(Debug, Clone, Serialize)]
pub struct Embedding {
    pub embedding: Vec<f32>,
}

impl Embedding {
    pub fn normalize<D: Dimension>(arr: Array<f32, D>) -> Array<f32, D> {
        const NORMALIZE_EPS: f32 = 1e-12;

        let norm = arr.mapv(|x| x.powi(2)).sum().sqrt().max(NORMALIZE_EPS);
        arr / norm
    }

    pub fn from_unnormalized_array(embedding: ArrayD<f32>) -> Self {
        Self {
            embedding: Embedding::normalize(embedding).into_iter().collect(),
        }
    }
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value_t = String::from("127.0.0.1:10000"))]
    address: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env_lossy(),
        )
        .init();

    let address: SocketAddr = args.address.parse().expect_or_log("Can't parse address");

    initialize_models().expect_or_log("Can't initialize models");

    let app = Router::new()
        .route("/clip/image", post(clip_image::process_request))
        .route("/clip/text", post(clip_text::process_request))
        .route("/minilm/text", post(minilm_text::process_request))
        .route("/minilm/rerank", post(minilm_rerank::process_request))
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024))
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
                .timeout(Duration::from_secs(30))
                .layer(TraceLayer::new_for_http()),
        );
    let url = format!("http://{address}");
    tracing::info!("Listening on {}", url);

    axum::Server::bind(&address)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap_or_log();
}

fn initialize_models() -> anyhow::Result<()> {
    let environment = Environment::builder()
        .with_name("nn_server_env")
        .with_log_level(LoggingLevel::Warning)
        .build()?;
    clip_image::initialize_model(&environment)?;
    clip_text::initialize_model(&environment)?;
    minilm_text::initialize_model(&environment)?;
    minilm_rerank::initialize_model(&environment)?;
    Ok(())
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
