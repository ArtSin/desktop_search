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

pub async fn get_image_search_image_embedding(
    reqwest_client: &reqwest::Client,
    mut nnserver_url: Url,
    image_path: impl AsRef<Path>,
) -> anyhow::Result<ImageEmbedding> {
    nnserver_url.set_path("clip/image");
    let req_builder = reqwest_client.post(nnserver_url);
    let response = req_builder
        .body(File::open(image_path).await?)
        .send()
        .await?;
    if response.status().is_client_error() {
        return Ok(ImageEmbedding { embedding: None });
    }
    let embedding = response.json().await?;
    Ok(embedding)
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
    reqwest_client: &reqwest::Client,
    mut nnserver_url: Url,
    text: &str,
) -> anyhow::Result<TextEmbedding> {
    // TODO: make into settings
    const MAX_SENTENCES: u32 = 20;
    const SENTENCES_PER_PARAGRAPH: u32 = 4;

    nnserver_url.set_path("minilm/text");
    let req_builder = reqwest_client.post(nnserver_url);
    let embedding = req_builder
        .json(&json!({
            "text": text,
            "max_sentences": MAX_SENTENCES,
            "sentences_per_paragraph": SENTENCES_PER_PARAGRAPH
        }))
        .send()
        .await?
        .json()
        .await?;
    Ok(embedding)
}
