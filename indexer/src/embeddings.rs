use std::path::Path;

use common_lib::BatchRequest;
use serde::Deserialize;
use serde_json::json;
use url::Url;

#[derive(Deserialize)]
pub struct ImageEmbedding {
    pub embedding: Option<Vec<f32>>,
}

#[derive(Deserialize)]
pub struct TextEmbedding {
    pub embedding: Vec<f32>,
}

#[derive(Deserialize)]
pub struct SummaryTextEmbedding {
    pub embedding: Vec<f32>,
    pub summary: Vec<String>,
}

#[derive(Deserialize)]
pub struct Scores {
    pub scores: Vec<f32>,
}

pub async fn get_image_search_image_embedding_generic<T: Into<reqwest::Body>>(
    reqwest_client: &reqwest_middleware::ClientWithMiddleware,
    mut nn_server_url: Url,
    batch_request: BatchRequest,
    image: T,
) -> anyhow::Result<ImageEmbedding> {
    nn_server_url.set_path("clip/image");
    let req_builder = reqwest_client.post(nn_server_url).query(&batch_request);
    let response = req_builder.body(image).send().await?;
    if response.status().is_client_error() {
        return Ok(ImageEmbedding { embedding: None });
    }
    let embedding = response.json().await?;
    Ok(embedding)
}

pub async fn get_image_search_image_embedding(
    reqwest_client: &reqwest_middleware::ClientWithMiddleware,
    nn_server_url: Url,
    batch_request: BatchRequest,
    image_path: impl AsRef<Path>,
) -> anyhow::Result<ImageEmbedding> {
    let file = tokio::fs::read(image_path).await?;
    get_image_search_image_embedding_generic(reqwest_client, nn_server_url, batch_request, file)
        .await
}

pub async fn get_image_search_text_embedding(
    reqwest_client: &reqwest_middleware::ClientWithMiddleware,
    mut nn_server_url: Url,
    batch_request: BatchRequest,
    text: &str,
) -> anyhow::Result<TextEmbedding> {
    nn_server_url.set_path("clip/text");
    let req_builder = reqwest_client.post(nn_server_url).query(&batch_request);
    let embedding = req_builder
        .json(&json!({ "text": text }))
        .send()
        .await?
        .json()
        .await?;
    Ok(embedding)
}

pub async fn get_text_search_embedding(
    reqwest_client: &reqwest_middleware::ClientWithMiddleware,
    mut nn_server_url: Url,
    batch_request: BatchRequest,
    text: &str,
    summary_enabled: bool,
) -> anyhow::Result<SummaryTextEmbedding> {
    nn_server_url.set_path("minilm/text");
    let req_builder = reqwest_client.post(nn_server_url).query(&batch_request);
    let embedding = req_builder
        .json(&json!({
            "text": text,
            "summary_enabled": summary_enabled,
        }))
        .send()
        .await?
        .json()
        .await?;
    Ok(embedding)
}

pub async fn get_rerank_scores(
    reqwest_client: &reqwest_middleware::ClientWithMiddleware,
    mut nn_server_url: Url,
    batch_request: BatchRequest,
    queries: Vec<String>,
    paragraphs: Vec<String>,
) -> anyhow::Result<Scores> {
    nn_server_url.set_path("minilm/rerank");
    let req_builder = reqwest_client.post(nn_server_url).query(&batch_request);
    let embedding = req_builder
        .json(&json!({
            "queries": queries,
            "paragraphs": paragraphs,
        }))
        .send()
        .await?
        .json()
        .await?;
    Ok(embedding)
}
