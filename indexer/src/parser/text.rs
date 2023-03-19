use std::sync::Arc;

use async_trait::async_trait;
use common_lib::{
    elasticsearch::{FileES, TextData},
    BatchRequest,
};
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
        metadata: &mut Metadata,
        _file_bytes: &[u8],
    ) -> anyhow::Result<()> {
        file.content = metadata.content.clone();

        tracing::debug!(
            "Calculating text embedding of file: {}",
            file.path.display()
        );

        let text_search_enabled = state.settings.read().await.other.text_search_enabled;
        if text_search_enabled {
            let (max_sentences, sentences_per_paragraph, nnserver_url) = {
                let tmp = state.settings.read().await;
                (
                    tmp.other.max_sentences,
                    tmp.other.sentences_per_paragraph,
                    tmp.other.nnserver_url.clone(),
                )
            };
            let embedding = get_text_search_embedding(
                max_sentences,
                sentences_per_paragraph,
                &state.reqwest_client,
                nnserver_url,
                BatchRequest { batched: true },
                file.content.as_ref().unwrap_or_log(),
                true,
            )
            .await?;

            file.text_data = TextData {
                text_embedding: Some(embedding.embedding),
                summary: embedding.summary,
            };
        }
        Ok(())
    }
}
