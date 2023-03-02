use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use common_lib::elasticsearch::{DocumentData, FileES, FileMetadata};
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

use crate::ServerState;

use super::{deserialize_datetime_maybe_local, Metadata, Parser};

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

impl FileMetadata for DocumentMetadata {
    fn any_metadata(&self) -> bool {
        self.title.is_some()
            || self.creator.is_some()
            || self.doc_created.is_some()
            || self.doc_modified.is_some()
            || self.num_pages.is_some()
            || self.num_words.is_some()
            || self.num_characters.is_some()
    }
}

pub struct DocumentParser;

#[async_trait]
impl Parser for DocumentParser {
    fn is_supported_file(&self, metadata: &Metadata) -> bool {
        metadata.document_data.any_metadata()
    }

    async fn parse(
        &self,
        _state: Arc<ServerState>,
        file: &mut FileES,
        metadata: &mut Metadata,
        _file_bytes: &[u8],
    ) -> anyhow::Result<()> {
        let data = std::mem::take(&mut metadata.document_data);
        file.document_data = DocumentData {
            title: data.title,
            creator: data.creator,
            doc_created: data.doc_created,
            doc_modified: data.doc_modified,
            num_pages: data.num_pages,
            num_words: data.num_words,
            num_characters: data.num_characters,
        };
        Ok(())
    }
}
