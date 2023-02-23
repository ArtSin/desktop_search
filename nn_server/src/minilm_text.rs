use std::{fs, str::FromStr, time::Instant};

use axum::{http::StatusCode, Json};
use ndarray::{ArrayD, Axis};
use once_cell::sync::OnceCell;
use onnxruntime::{environment::Environment, session::Session, GraphOptimizationLevel};
use serde::Deserialize;
use srx::{Rules, SRX};
use tokenizers::Tokenizer;
use tracing_unwrap::{OptionExt, ResultExt};

use crate::{
    text_processing::{mean_pooling, preprocess_texts, PreprocessedText},
    Embedding, PATH_PREFIX,
};

const BATCH_SIZE: usize = 32;
const MAX_DELAY: u128 = 100;

static MODEL: OnceCell<Session> = OnceCell::new();
static TOKENIZER: OnceCell<Tokenizer> = OnceCell::new();
static SRX_RULES: OnceCell<Rules> = OnceCell::new();

#[derive(Debug, Clone, Deserialize)]
pub struct MiniLMTextRequest {
    text: String,
    max_sentences: u32,
    sentences_per_paragraph: u32,
}

pub fn initialize_model(environment: &Environment) -> anyhow::Result<()> {
    MODEL
        .set(
            environment
                .new_session_builder()?
                .use_cuda(0)?
                .with_graph_optimization_level(GraphOptimizationLevel::All)?
                .with_intra_op_num_threads(1)?
                .with_model_from_file(
                    PATH_PREFIX.to_owned()
                        + "models/paraphrase-multilingual-MiniLM-L12-v2/model.onnx",
                )?,
        )
        .unwrap_or_log();
    TOKENIZER
        .set(
            Tokenizer::from_file(
                PATH_PREFIX.to_owned()
                    + "models/paraphrase-multilingual-MiniLM-L12-v2/tokenizer.json",
            )
            .map_err(|err| anyhow::anyhow!(err))?,
        )
        .unwrap_or_log();
    SRX_RULES
        .set(
            SRX::from_str(&fs::read_to_string(
                PATH_PREFIX.to_owned() + "data/segment.srx",
            )?)?
            .language_rules("ru"),
        )
        .unwrap_or_log();
    Ok(())
}

fn compute_embeddings(paragraphs: Vec<String>) -> anyhow::Result<Vec<ArrayD<f32>>> {
    let start_time = Instant::now();
    let session = MODEL.get().unwrap_or_log();
    let tokenizer = TOKENIZER.get().unwrap_or_log();

    let PreprocessedText {
        input_ids,
        attention_mask,
        type_ids,
    } = preprocess_texts(tokenizer, paragraphs, true).unwrap_or_log();

    let output = session.run(vec![
        input_ids.into(),
        attention_mask.clone().into(),
        type_ids.unwrap_or_log().into(),
    ])?;
    let res: Vec<_> = mean_pooling(output[0].float_array().unwrap_or_log(), attention_mask)
        .outer_iter()
        .map(|x| x.into_owned())
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
    Json(request): Json<MiniLMTextRequest>,
) -> Result<Json<Embedding>, (StatusCode, String)> {
    let text = request.text.replace(['\r', '\n', '\t'], " ");
    let sentences: Vec<_> = SRX_RULES
        .get()
        .unwrap_or_log()
        .split(&text)
        .take(request.max_sentences as usize)
        .collect();
    let paragraphs: Vec<_> = sentences
        .chunks(request.sentences_per_paragraph as usize)
        .map(|x| x.join(""))
        .collect();

    let batch_compute = batched_fn::batched_fn! {
        handler = |batch: Vec<String>| -> Vec<ArrayD<f32>> {
            compute_embeddings(batch).expect_or_log("Can't compute embedding")
        };
        config = {
            max_batch_size: BATCH_SIZE,
            max_delay: MAX_DELAY,
        };
        context = {};
    };
    let paragraphs_embeddings_tasks: Vec<_> = paragraphs
        .into_iter()
        .map(|x| tokio::spawn(async move { batch_compute(x).await }))
        .collect();
    let mut paragraphs_embeddings = Vec::new();
    for x in paragraphs_embeddings_tasks {
        paragraphs_embeddings.push(x.await.unwrap_or_log().map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Batch processing error: {err:#?}"),
            )
        })?);
    }

    let mean_embedding = Embedding::from_unnormalized_array(
        ndarray::stack(
            Axis(0),
            &paragraphs_embeddings
                .iter()
                .map(|x| x.view())
                .collect::<Vec<_>>(),
        )
        .unwrap_or_log()
        .mean_axis(Axis(0))
        .unwrap_or_log(),
    );
    Ok(Json(mean_embedding))
}
