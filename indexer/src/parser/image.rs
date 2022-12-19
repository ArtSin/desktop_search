use std::sync::Arc;

use async_trait::async_trait;
use common_lib::{
    elasticsearch::{FileES, ImageData},
    embeddings::get_image_search_image_embedding,
};
use tokio::sync::RwLock;

use crate::ServerState;

use super::{Metadata, Parser};

pub struct ImageParser;

#[async_trait]
impl Parser for ImageParser {
    fn is_supported_content_type(&self, content_type: &str) -> bool {
        content_type.starts_with("image")
    }

    async fn parse(
        &self,
        state: Arc<RwLock<ServerState>>,
        file: &mut FileES,
        metadata: &Metadata,
    ) -> anyhow::Result<()> {
        tracing::debug!(
            "Calculating image embedding of file: {}",
            file.path.display()
        );

        let reqwest_client = &state.read().await.reqwest_client;
        let nnserver_url = state.read().await.settings.other.nnserver_url.clone();
        let embedding =
            get_image_search_image_embedding(reqwest_client, nnserver_url, &file.path).await?;

        file.image_data = Some(ImageData {
            image_embedding: embedding.embedding,
        });
        Ok(())
    }
}
