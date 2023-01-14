use std::sync::Arc;

use async_trait::async_trait;
use common_lib::elasticsearch::{FileES, TextData};
use tracing_unwrap::OptionExt;

use crate::{embeddings::get_text_search_embedding, ServerState};

use super::{Metadata, Parser};

pub struct TextParser;

#[async_trait]
impl Parser for TextParser {
    fn is_supported_file(&self, metadata: &Metadata) -> bool {
        metadata.content.is_some()
    }

    async fn parse(
        &self,
        state: Arc<ServerState>,
        file: &mut FileES,
        metadata: &Metadata,
    ) -> anyhow::Result<()> {
        file.content = metadata.content.clone();

        tracing::debug!(
            "Calculating text embedding of file: {}",
            file.path.display()
        );

        let nnserver_url = state.settings.read().await.other.nnserver_url.clone();
        let embedding = get_text_search_embedding(
            &state.reqwest_client,
            nnserver_url,
            file.content.as_ref().unwrap_or_log(),
        )
        .await?;

        file.text_data = TextData {
            text_embedding: Some(embedding.embedding),
        };
        Ok(())
    }
}
