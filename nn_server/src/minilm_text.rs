use std::time::Duration;

use axum::{extract::Query, http::StatusCode, Json};
use common_lib::BatchRequest;
use ndarray::{ArrayD, Axis};
use once_cell::sync::OnceCell;
use onnxruntime::{environment::Environment, session::Session, GraphOptimizationLevel};
use serde::{Deserialize, Serialize};
use tokenizers::Tokenizer;
use tokio::sync::mpsc;
use tracing_unwrap::{OptionExt, ResultExt};

use crate::{
    batch_processing::{batch_process, log_processing_function, start_batch_process, Command},
    lexrank::degree_centrality_scores,
    text_processing::{mean_pooling, preprocess_texts, PreprocessedText},
    Embedding, PATH_PREFIX,
};

const BATCH_SIZE: usize = 64;
const MAX_DELAY: Duration = Duration::from_millis(100);
const MAX_CAPACITY: usize = 2 * BATCH_SIZE;
const EMBEDDING_SIZE: usize = 384;

static MODEL: OnceCell<Session> = OnceCell::new();
static TOKENIZER: OnceCell<Tokenizer> = OnceCell::new();
static BATCH_SENDER: OnceCell<mpsc::Sender<Command<String, ArrayD<f32>>>> = OnceCell::new();

#[derive(Debug, Clone, Deserialize)]
pub struct MiniLMTextRequest {
    text: String,
    max_sentences: u32,
    summary_enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SummaryEmbedding {
    #[serde(flatten)]
    embedding: Embedding,
    summary: Vec<String>,
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
    BATCH_SENDER
        .set(start_batch_process(
            BATCH_SIZE,
            MAX_DELAY,
            MAX_CAPACITY,
            |batch| log_processing_function("MiniLM/Text", compute_embeddings, batch),
        ))
        .unwrap_or_log();
    Ok(())
}

fn compute_embeddings(paragraphs: Vec<String>) -> anyhow::Result<Vec<ArrayD<f32>>> {
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
    Ok(res)
}

pub async fn process_request(
    Query(batch_query): Query<BatchRequest>,
    Json(request): Json<MiniLMTextRequest>,
) -> Result<Json<SummaryEmbedding>, (StatusCode, String)> {
    const WINDOW_STEP: usize = 75;
    const WINDOW_SIZE: usize = 100;
    let words: Vec<_> = request.text.split_whitespace().collect();
    let paragraphs: Vec<_> = (0..words.len())
        .step_by(WINDOW_STEP)
        .take(request.max_sentences as usize)
        .map(|i| words[i..(i + WINDOW_SIZE).min(words.len())].join(" "))
        .collect();

    // Spawn tasks for each paragraph
    let paragraphs_embeddings_tasks: Vec<_> = paragraphs
        .iter()
        .cloned()
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
    let mut paragraphs_embeddings = Vec::new();
    for x in paragraphs_embeddings_tasks {
        paragraphs_embeddings.push(x.await.unwrap_or_log());
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

    let summary = if request.summary_enabled {
        const K: usize = 3;

        let norm_paragraphs_embeddings: Vec<_> = paragraphs_embeddings
            .into_iter()
            .map(Embedding::normalize)
            .collect();
        let norm_paragraphs = ndarray::stack(
            Axis(0),
            &norm_paragraphs_embeddings
                .iter()
                .map(|x| x.view())
                .collect::<Vec<_>>(),
        )
        .unwrap_or_log()
        .into_shape((norm_paragraphs_embeddings.len(), EMBEDDING_SIZE))
        .unwrap_or_log();

        let paragraphs_cos_sim = norm_paragraphs.dot(&norm_paragraphs.t()).mapv(|x| x as f64);
        let centrality_scores = degree_centrality_scores(paragraphs_cos_sim).to_vec();
        let mut indices: Vec<usize> = (0..centrality_scores.len()).collect();
        indices.sort_unstable_by(|i, j| {
            centrality_scores[*j]
                .partial_cmp(&centrality_scores[*i])
                .unwrap()
        });

        indices
            .into_iter()
            .take(K)
            .map(|i| paragraphs[i].clone())
            .collect()
    } else {
        Vec::new()
    };

    Ok(Json(SummaryEmbedding {
        embedding: mean_embedding,
        summary,
    }))
}
