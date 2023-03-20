use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Local, TimeZone, Utc};
use common_lib::elasticsearch::FileES;
use mime::Mime;
use serde::{de::Error, Deserialize, Deserializer};

use crate::ServerState;

use self::{document::DocumentMetadata, image::ImageMetadata, multimedia::MultimediaMetadata};

mod document;
mod image;
mod multimedia;
mod text;

const PARSERS: [&(dyn Parser + Send + Sync); 4] = [
    &text::TextParser,
    &image::ImageParser,
    &multimedia::MultimediaParser,
    &document::DocumentParser,
];

#[async_trait]
pub trait Parser {
    fn is_supported_file(&self, metadata: &Metadata) -> bool;
    async fn parse(
        &self,
        state: Arc<ServerState>,
        file: &mut FileES,
        metadata: &mut Metadata,
        file_bytes: &[u8],
    ) -> anyhow::Result<()>;
}

#[derive(Deserialize)]
pub struct Metadata {
    #[serde(rename = "Content-Type")]
    pub content_type: String,
    #[serde(rename = "X-TIKA:content")]
    pub content: Option<String>,
    /// Fields for image files
    #[serde(flatten)]
    pub image_data: ImageMetadata,
    /// Fields for multimedia files
    #[serde(flatten)]
    pub multimedia_data: MultimediaMetadata,
    /// Fields for document files
    #[serde(flatten)]
    pub document_data: DocumentMetadata,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            content_type: "application/octet-stream".to_owned(),
            content: Default::default(),
            image_data: Default::default(),
            multimedia_data: Default::default(),
            document_data: Default::default(),
        }
    }
}

async fn get_metadata_and_bytes(
    state: Arc<ServerState>,
    file: &mut FileES,
) -> anyhow::Result<(Metadata, Vec<u8>)> {
    if file.size == 0 {
        return Ok((Metadata::default(), Vec::new()));
    }

    let mut tika_meta_url = state.settings.read().await.tika_url.clone();
    tika_meta_url.set_path("rmeta/text");
    let req_builder = state.reqwest_client.put(tika_meta_url);
    let file = tokio::fs::read(&file.path).await?;
    let [metadata]: [Metadata; 1] = req_builder
        .header("Accept", "application/json")
        .header("maxEmbeddedResources", "0")
        .body(file.clone())
        .send()
        .await?
        .json()
        .await?;
    Ok((metadata, file))
}

pub async fn parse_file(state: Arc<ServerState>, file: &mut FileES) -> anyhow::Result<()> {
    let (mut metadata, file_bytes) = get_metadata_and_bytes(Arc::clone(&state), file).await?;
    let mut content_type_mime: Mime = metadata.content_type.parse()?;
    if content_type_mime.type_() == mime::TEXT {
        let new_mime = mime_guess::from_path(&file.path).first_or_octet_stream();
        if new_mime.type_() == mime::TEXT {
            content_type_mime = new_mime;
            metadata.content_type = content_type_mime.to_string();
        }
    }

    file.content_type = metadata.content_type.clone();
    file.content_type_mime_type = content_type_mime.type_().to_string();
    file.content_type_mime_essence = content_type_mime.essence_str().to_owned();

    for parser in PARSERS {
        if parser.is_supported_file(&metadata) {
            parser
                .parse(Arc::clone(&state), file, &mut metadata, &file_bytes)
                .await?;
        }
    }

    Ok(())
}

/// Deserialize Option<DateTime> from string with given time zone, or local if not given
pub fn deserialize_datetime_maybe_local<'de, D>(
    deserializer: D,
) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::<String>::deserialize(deserializer)? {
        Some(s) => match s.parse::<DateTime<Utc>>() {
            Ok(x) => Ok(Some(x)),
            Err(_) => Local
                .datetime_from_str(&s, "%Y-%m-%dT%H:%M:%S%.f")
                .map(|x| Some(x.into()))
                .map_err(D::Error::custom),
        },
        None => Ok(None),
    }
}
