use std::time::Instant;

use axum::{http::StatusCode, Json};
use once_cell::sync::OnceCell;
use onnxruntime::{environment::Environment, session::Session, GraphOptimizationLevel};
use serde::Deserialize;
use tokenizers::{PaddingParams, Tokenizer, TruncationParams};
use tracing_unwrap::{OptionExt, ResultExt};

use crate::{
    text_processing::{mean_pooling, preprocess_texts, PreprocessedText},
    Embedding, PATH_PREFIX,
};

const BATCH_SIZE: usize = 32;
const MAX_DELAY: u128 = 100;

static MAIN_MODEL: OnceCell<Session> = OnceCell::new();
static DENSE_MODEL: OnceCell<Session> = OnceCell::new();
static TOKENIZER: OnceCell<Tokenizer> = OnceCell::new();

#[derive(Debug, Clone, Deserialize)]
pub struct CLIPTextRequest {
    text: String,
}

pub fn initialize_model(environment: &Environment) -> anyhow::Result<()> {
    MAIN_MODEL
        .set(
            environment
                .new_session_builder()?
                .use_cuda(0)?
                .with_graph_optimization_level(GraphOptimizationLevel::All)?
                .with_intra_op_num_threads(1)?
                .with_model_from_file(
                    PATH_PREFIX.to_owned() + "models/clip-ViT-B-32-multilingual-v1/model.onnx",
                )?,
        )
        .unwrap_or_log();
    DENSE_MODEL
        .set(
            environment
                .new_session_builder()?
                // .use_cuda(0)?
                .with_graph_optimization_level(GraphOptimizationLevel::All)?
                .with_intra_op_num_threads(1)?
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
    Ok(())
}

fn compute_embeddings(texts: Vec<String>) -> anyhow::Result<Vec<Embedding>> {
    let start_time = Instant::now();
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

    let indexing_duration = Instant::now() - start_time;
    tracing::info!(
        "Processed {} requests in {:#?}",
        res.len(),
        indexing_duration
    );
    Ok(res)
}

pub async fn process_request(
    Json(request): Json<CLIPTextRequest>,
) -> Result<Json<Embedding>, (StatusCode, String)> {
    let batch_compute = batched_fn::batched_fn! {
        handler = |batch: Vec<String>| -> Vec<Embedding> {
            compute_embeddings(batch).expect_or_log("Can't compute embedding")
        };
        config = {
            max_batch_size: BATCH_SIZE,
            max_delay: MAX_DELAY,
        };
        context = {};
    };
    batch_compute(request.text).await.map(Json).map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Batch processing error: {err:#?}"),
        )
    })
}
