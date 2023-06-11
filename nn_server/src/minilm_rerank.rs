use axum::{extract::Query, http::StatusCode, Json};
use common_lib::{settings::NNServerSettings, BatchRequest};
use once_cell::sync::OnceCell;
use onnxruntime::{environment::Environment, session::Session, GraphOptimizationLevel};
use serde::{Deserialize, Serialize};
use tokenizers::{
    PaddingDirection, PaddingParams, PaddingStrategy, Tokenizer, TruncationDirection,
    TruncationParams, TruncationStrategy,
};
use tokio::sync::mpsc;
use tracing_unwrap::{OptionExt, ResultExt};

use crate::{
    batch_processing::{batch_process, log_processing_function, start_batch_process, Command},
    set_device,
    text_processing::{preprocess_texts, PreprocessedText},
    PATH_PREFIX,
};

static MODEL: OnceCell<Session> = OnceCell::new();
static TOKENIZER: OnceCell<Tokenizer> = OnceCell::new();
static BATCH_SENDER: OnceCell<mpsc::Sender<Command<(String, String), f32>>> = OnceCell::new();

#[derive(Debug, Clone, Deserialize)]
pub struct MiniLMRerankRequest {
    queries: Vec<String>,
    paragraphs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Scores {
    scores: Vec<f32>,
}

pub fn initialize_model(
    settings: &NNServerSettings,
    environment: &Environment,
) -> anyhow::Result<()> {
    MODEL
        .set(
            set_device(environment.new_session_builder()?, &settings.minilm_rerank)?
                .with_graph_optimization_level(GraphOptimizationLevel::All)?
                .with_model_from_file(
                    PATH_PREFIX.to_owned() + "models/mMiniLM-L6-v2-mmarco-v2/model.onnx",
                )?,
        )
        .unwrap_or_log();
    TOKENIZER
        .set(
            Tokenizer::from_file(
                PATH_PREFIX.to_owned() + "models/mMiniLM-L6-v2-mmarco-v2/tokenizer.json",
            )
            .map(|mut x| {
                x.with_truncation(Some(TruncationParams {
                    max_length: 256,
                    strategy: TruncationStrategy::default(),
                    stride: 0,
                    direction: TruncationDirection::default(),
                }));
                x.with_padding(Some(PaddingParams {
                    strategy: PaddingStrategy::BatchLongest,
                    direction: PaddingDirection::Right,
                    pad_to_multiple_of: None,
                    pad_id: 1,
                    pad_type_id: 0,
                    pad_token: "<pad>".to_owned(),
                }));
                x
            })
            .map_err(|err| anyhow::anyhow!(err))?,
        )
        .unwrap_or_log();
    BATCH_SENDER
        .set(start_batch_process(&settings.minilm_rerank, |batch| {
            log_processing_function("MiniLM/Rerank", compute_embeddings, batch)
        }))
        .unwrap_or_log();
    Ok(())
}

fn compute_embeddings(queries_paragraphs: Vec<(String, String)>) -> anyhow::Result<Vec<f32>> {
    let session = MODEL.get().unwrap_or_log();
    let tokenizer = TOKENIZER.get().unwrap_or_log();

    let PreprocessedText {
        input_ids,
        attention_mask,
        ..
    } = preprocess_texts(tokenizer, queries_paragraphs, false).unwrap_or_log();

    let output = session.run(vec![input_ids.into(), attention_mask.into()])?;
    let res: Vec<_> = (*output[0].float_array().unwrap_or_log())
        .to_owned()
        .into_iter()
        .collect();
    Ok(res)
}

pub async fn process_request(
    Query(batch_query): Query<BatchRequest>,
    Json(request): Json<MiniLMRerankRequest>,
) -> Result<Json<Scores>, (StatusCode, String)> {
    // Spawn tasks for each pair
    let tasks: Vec<_> = request
        .queries
        .into_iter()
        .zip(request.paragraphs)
        .map(|x| {
            tokio::spawn(async move {
                batch_process(BATCH_SENDER.get().unwrap_or_log(), x, false).await
            })
        })
        .collect();
    // Send flush command if needed
    if !batch_query.batched {
        BATCH_SENDER
            .get()
            .unwrap_or_log()
            .send(Command::Flush)
            .await
            .expect_or_log("Error sending to batch processing channel");
    }
    // Wait for all tasks to finish
    let mut scores = Vec::new();
    for x in tasks {
        scores.push(x.await.unwrap_or_log());
    }

    Ok(Json(Scores { scores }))
}
