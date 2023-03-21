use std::time::Duration;

use axum::{extract::Query, http::StatusCode, Json};
use common_lib::{settings::NNServerSettings, BatchRequest};
use once_cell::sync::OnceCell;
use onnxruntime::{environment::Environment, session::Session, GraphOptimizationLevel};
use serde::Deserialize;
use tokenizers::{PaddingParams, Tokenizer, TruncationParams};
use tokio::sync::mpsc;
use tracing_unwrap::{OptionExt, ResultExt};

use crate::{
    batch_processing::{batch_process, log_processing_function, start_batch_process, Command},
    set_device,
    text_processing::{mean_pooling, preprocess_texts, PreprocessedText},
    Embedding, PATH_PREFIX,
};

static MAIN_MODEL: OnceCell<Session> = OnceCell::new();
static DENSE_MODEL: OnceCell<Session> = OnceCell::new();
static TOKENIZER: OnceCell<Tokenizer> = OnceCell::new();
static BATCH_SENDER: OnceCell<mpsc::Sender<Command<String, Embedding>>> = OnceCell::new();

#[derive(Debug, Clone, Deserialize)]
pub struct CLIPTextRequest {
    text: String,
}

pub fn initialize_model(
    settings: &NNServerSettings,
    environment: &Environment,
) -> anyhow::Result<()> {
    MAIN_MODEL
        .set(
            set_device(environment.new_session_builder()?, settings)?
                .with_graph_optimization_level(GraphOptimizationLevel::All)?
                .with_model_from_file(
                    PATH_PREFIX.to_owned() + "models/clip-ViT-B-32-multilingual-v1/model.onnx",
                )?,
        )
        .unwrap_or_log();
    // Always on CPU
    DENSE_MODEL
        .set(
            environment
                .new_session_builder()?
                .with_graph_optimization_level(GraphOptimizationLevel::All)?
                .with_model_from_file(
                    PATH_PREFIX.to_owned() + "models/clip-ViT-B-32-multilingual-v1/dense.onnx",
                )?,
        )
        .unwrap_or_log();
    TOKENIZER
        .set(
            Tokenizer::from_file(
                PATH_PREFIX.to_owned() + "models/clip-ViT-B-32-multilingual-v1/tokenizer.json",
            )
            .map(|mut x| {
                x.with_padding(Some(PaddingParams::default()));
                x.with_truncation(Some(TruncationParams::default()));
                x
            })
            .map_err(|err| anyhow::anyhow!(err))?,
        )
        .unwrap_or_log();
    BATCH_SENDER
        .set(start_batch_process(
            settings.clip_text_batch_size,
            Duration::from_millis(settings.clip_text_max_delay_ms),
            2 * settings.clip_text_batch_size,
            |batch| log_processing_function("CLIP/Text", compute_embeddings, batch),
        ))
        .unwrap_or_log();
    Ok(())
}

fn compute_embeddings(texts: Vec<String>) -> anyhow::Result<Vec<Embedding>> {
    let session_main = MAIN_MODEL.get().unwrap_or_log();
    let session_dense = DENSE_MODEL.get().unwrap_or_log();
    let tokenizer = TOKENIZER.get().unwrap_or_log();

    let PreprocessedText {
        input_ids,
        attention_mask,
        ..
    } = preprocess_texts(tokenizer, texts, false).unwrap_or_log();

    let output_main = session_main.run(vec![input_ids.into(), attention_mask.clone().into()])?;
    let res_main = mean_pooling(output_main[0].float_array().unwrap_or_log(), attention_mask);
    let output_dense = session_dense.run(vec![res_main.into()])?;

    let res: Vec<_> = output_dense[0]
        .float_array()
        .unwrap_or_log()
        .outer_iter()
        .map(|x| Embedding::from_unnormalized_array(x.into_owned()))
        .collect();
    Ok(res)
}

pub async fn process_request(
    Query(batch_query): Query<BatchRequest>,
    Json(request): Json<CLIPTextRequest>,
) -> Result<Json<Embedding>, (StatusCode, String)> {
    Ok(Json(
        batch_process(
            BATCH_SENDER.get().unwrap_or_log(),
            request.text,
            !batch_query.batched,
        )
        .await,
    ))
}
