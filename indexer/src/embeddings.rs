use std::path::Path;

use serde::Deserialize;
use serde_json::json;
use tokio::fs::File;
use url::Url;

#[derive(Deserialize)]
pub struct ImageEmbedding {
    pub embedding: Option<Vec<f32>>,
}

#[derive(Deserialize)]
pub struct TextEmbedding {
    pub embedding: Vec<f32>,
}

pub async fn get_image_search_image_embedding_generic<T: Into<reqwest::Body>>(
    reqwest_client: &reqwest::Client,
    mut nnserver_url: Url,
    image: T,
) -> anyhow::Result<ImageEmbedding> {
    nnserver_url.set_path("clip/image");
    let req_builder = reqwest_client.post(nnserver_url);
    let response = req_builder.body(image).send().await?;
    if response.status().is_client_error() {
        return Ok(ImageEmbedding { embedding: None });
    }
    let embedding = response.json().await?;
    Ok(embedding)
}

pub async fn get_image_search_image_embedding(
    reqwest_client: &reqwest::Client,
    nnserver_url: Url,
    image_path: impl AsRef<Path>,
) -> anyhow::Result<ImageEmbedding> {
    get_image_search_image_embedding_generic(
        reqwest_client,
        nnserver_url,
        File::open(image_path).await?,
    )
    .await
}

pub async fn get_image_search_text_embedding(
    reqwest_client: &reqwest::Client,
    mut nnserver_url: Url,
    text: &str,
) -> anyhow::Result<TextEmbedding> {
    nnserver_url.set_path("clip/text");
    let req_builder = reqwest_client.post(nnserver_url);
    let embedding = req_builder
        .json(&json!({ "text": text }))
        .send()
        .await?
        .json()
        .await?;
    Ok(embedding)
}

pub async fn get_text_search_embedding(
    max_sentences: u32,
    sentences_per_paragraph: u32,
    reqwest_client: &reqwest::Client,
    mut nnserver_url: Url,
    text: &str,
) -> anyhow::Result<TextEmbedding> {
    nnserver_url.set_path("minilm/text");
    let req_builder = reqwest_client.post(nnserver_url);
    let embedding = req_builder
        .json(&json!({
            "text": text,
            "max_sentences": max_sentences,
            "sentences_per_paragraph": sentences_per_paragraph
        }))
        .send()
        .await?
        .json()
        .await?;
    Ok(embedding)
}
