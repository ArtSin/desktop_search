use std::sync::Arc;

use async_trait::async_trait;
use common_lib::elasticsearch::FileES;

use crate::ServerState;

use super::{Metadata, Parser};

pub struct TextParser;

#[async_trait]
impl Parser for TextParser {
    fn is_supported_file(&self, metadata: &Metadata) -> bool {
        metadata.content.is_some()
    }

    async fn parse(
        &self,
        _state: Arc<ServerState>,
        file: &mut FileES,
        metadata: &Metadata,
    ) -> anyhow::Result<()> {
        file.content = metadata.content.clone();
        Ok(())
    }
}
