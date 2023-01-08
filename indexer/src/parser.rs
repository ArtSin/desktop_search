use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Local, TimeZone, Utc};
use common_lib::elasticsearch::{DocumentData, FileES};
use mime::Mime;
use serde::{de::Error, Deserialize, Deserializer};
use serde_with::{serde_as, DisplayFromStr};
use tokio::fs::File;

use crate::ServerState;

use self::image::ImageMetadata;

mod image;
mod text;

const PARSERS: [&(dyn Parser + Send + Sync); 2] = [&image::ImageParser, &text::TextParser];

#[async_trait]
pub trait Parser {
    fn is_supported_file(&self, metadata: &Metadata) -> bool;
    async fn parse(
        &self,
        state: Arc<ServerState>,
        file: &mut FileES,
        metadata: &Metadata,
    ) -> anyhow::Result<()>;
}

#[derive(Default, Deserialize)]
pub struct Metadata {
    #[serde(rename = "Content-Type")]
    pub content_type: String,
    #[serde(rename = "X-TIKA:content")]
    pub content: Option<String>,
    /// Fields for image files
    #[serde(flatten)]
    pub image_data: ImageMetadata,
    /// Fields for document files
    #[serde(flatten)]
    pub document_data: DocumentMetadata,
}

#[serde_as]
#[derive(Default, Deserialize)]
pub struct DocumentMetadata {
    #[serde(rename = "dc:title")]
    title: Option<String>,
    #[serde(rename = "dc:creator")]
    creator: Option<String>,
    #[serde(
        rename = "dcterms:created",
        default,
        deserialize_with = "deserialize_datetime_maybe_local"
    )]
    doc_created: Option<DateTime<Utc>>,
    #[serde(
        rename = "dcterms:modified",
        default,
        deserialize_with = "deserialize_datetime_maybe_local"
    )]
    doc_modified: Option<DateTime<Utc>>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(rename = "xmpTPg:NPages")]
    num_pages: Option<u32>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(rename = "meta:word-count")]
    num_words: Option<u32>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(rename = "meta:character-count")]
    num_characters: Option<u32>,
}

async fn get_metadata(state: Arc<ServerState>, file: &mut FileES) -> anyhow::Result<Metadata> {
    if file.size == 0 {
        return Ok(Metadata::default());
    }

    let mut tika_meta_url = state.settings.read().await.other.tika_url.clone();
    tika_meta_url.set_path("rmeta/text");
    let req_builder = state.reqwest_client.put(tika_meta_url);
    let [metadata]: [Metadata; 1] = req_builder
        .header("Accept", "application/json")
        .header("maxEmbeddedResources", "0")
        .body(File::open(&file.path).await?)
        .send()
        .await?
        .json()
        .await?;
    Ok(metadata)
}

pub async fn parse_file(state: Arc<ServerState>, file: &mut FileES) -> anyhow::Result<()> {
    let mut metadata = get_metadata(Arc::clone(&state), file).await?;
    let mut content_type_mime: Mime = metadata.content_type.parse().unwrap();
    if content_type_mime.type_() == mime::TEXT {
        content_type_mime = mime_guess::from_path(&file.path).first_or_octet_stream();
        metadata.content_type = content_type_mime.to_string();
    }

    file.content_type = metadata.content_type.clone();
    file.content_type_mime_type = content_type_mime.type_().to_string();
    file.content_type_mime_essence = content_type_mime.essence_str().to_owned();

    for parser in PARSERS {
        if parser.is_supported_file(&metadata) {
            parser.parse(Arc::clone(&state), file, &metadata).await?;
        }
    }

    file.document_data = DocumentData {
        title: metadata.document_data.title,
        creator: metadata.document_data.creator,
        doc_created: metadata.document_data.doc_created,
        doc_modified: metadata.document_data.doc_modified,
        num_pages: metadata.document_data.num_pages,
        num_words: metadata.document_data.num_words,
        num_characters: metadata.document_data.num_characters,
    };

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
