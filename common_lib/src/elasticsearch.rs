use std::path::PathBuf;

use chrono::{
    serde::{ts_seconds, ts_seconds_option},
    DateTime, Utc,
};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

pub const ELASTICSEARCH_INDEX: &str = "files";
pub const ELASTICSEARCH_MAX_SIZE: i64 = 10000;
pub const ELASTICSEARCH_PIT_KEEP_ALIVE: &str = "1m";

pub trait FileMetadata {
    fn any_metadata(&self) -> bool;
}

/// File information as stored in Elasticsearch
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileES {
    /// ID of document
    pub _id: Option<String>,
    /// Absolute path to file
    pub path: PathBuf,
    /// Last modification time
    #[serde(with = "ts_seconds")]
    pub modified: DateTime<Utc>,
    /// Size of file in bytes
    pub size: u64,
    /// Base16 representation of SHA-256 hash of file
    pub hash: String,
    /// MIME content type
    pub content_type: String,
    /// Type part of content type
    pub content_type_mime_type: String,
    /// Essence part of content type
    pub content_type_mime_essence: String,
    /// Text content
    pub content: Option<String>,
    /// Fields for image files
    #[serde(flatten)]
    pub image_data: ImageData,
    /// Fields for document files
    #[serde(flatten)]
    pub document_data: DocumentData,
}

/// Fields for image files
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImageData {
    /// CLIP embedding of image
    pub image_embedding: Option<Vec<f32>>,
    /// Width in pixels
    pub width: Option<u32>,
    /// Height in pixels
    pub height: Option<u32>,
}

impl FileMetadata for ImageData {
    fn any_metadata(&self) -> bool {
        self.width.is_some() || self.height.is_some()
    }
}

/// Fields for document files
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocumentData {
    pub title: Option<String>,
    pub creator: Option<String>,
    #[serde(default, with = "ts_seconds_option")]
    pub doc_created: Option<DateTime<Utc>>,
    #[serde(default, with = "ts_seconds_option")]
    pub doc_modified: Option<DateTime<Utc>>,
    pub num_pages: Option<u32>,
    pub num_words: Option<u32>,
    pub num_characters: Option<u32>,
}

impl FileMetadata for DocumentData {
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
