use std::sync::Arc;

use async_trait::async_trait;
use common_lib::elasticsearch::FileES;
use serde::Deserialize;
use tokio::{fs::File, sync::RwLock};

use crate::ServerState;

mod image;

const PARSERS: [&(dyn Parser + Send + Sync); 1] = [&image::ImageParser];

#[async_trait]
pub trait Parser {
    fn is_supported_content_type(&self, content_type: &str) -> bool;
    async fn parse(
        &self,
        state: Arc<RwLock<ServerState>>,
        file: &mut FileES,
        metadata: &Metadata,
    ) -> anyhow::Result<()>;
}

#[derive(Deserialize)]
pub struct Metadata {
    #[serde(rename = "Content-Type")]
    content_type: String,
}

async fn get_metadata(
    state: Arc<RwLock<ServerState>>,
    file: &mut FileES,
) -> anyhow::Result<Metadata> {
    let mut tika_meta_url = state.read().await.settings.other.tika_url.clone();
    tika_meta_url.set_path("meta");
    let req_builder = state.read().await.reqwest_client.put(tika_meta_url);
    let metadata = req_builder
        .header("Accept", "application/json")
        .body(File::open(&file.path).await?)
        .send()
        .await?
        .json()
        .await?;
    Ok(metadata)
}

pub async fn parse_file(state: Arc<RwLock<ServerState>>, file: &mut FileES) -> anyhow::Result<()> {
    let metadata = get_metadata(Arc::clone(&state), file).await?;
    for parser in PARSERS {
        if parser.is_supported_content_type(&metadata.content_type) {
            parser.parse(Arc::clone(&state), file, &metadata).await?;
        }
    }
    Ok(())
}
