use std::sync::Arc;

use async_trait::async_trait;
use common_lib::elasticsearch::{FileES, ImageData};
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

use crate::{embeddings::get_image_search_image_embedding, ServerState};

use super::{Metadata, Parser};

pub struct ImageParser;

#[serde_as]
#[derive(Default, Deserialize)]
pub struct ImageMetadata {
    /// Width in pixels
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(rename = "tiff:ImageWidth")]
    width: Option<u32>,
    /// Height in pixels
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(rename = "tiff:ImageLength")]
    height: Option<u32>,
}

#[async_trait]
impl Parser for ImageParser {
    fn is_supported_content_type(&self, content_type: &str) -> bool {
        content_type.starts_with("image")
    }

    async fn parse(
        &self,
        state: Arc<ServerState>,
        file: &mut FileES,
        metadata: &Metadata,
    ) -> anyhow::Result<()> {
        tracing::debug!(
            "Calculating image embedding of file: {}",
            file.path.display()
        );

        let reqwest_client = &state.reqwest_client;
        let nnserver_url = state.settings.read().await.other.nnserver_url.clone();
        let embedding =
            get_image_search_image_embedding(reqwest_client, nnserver_url, &file.path).await?;

        file.image_data = ImageData {
            image_embedding: embedding.embedding,
            width: metadata.image_data.width,
            height: metadata.image_data.height,
        };
        Ok(())
    }
}
