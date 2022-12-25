use std::path::PathBuf;

use chrono::{serde::ts_seconds, DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

pub const ELASTICSEARCH_INDEX: &str = "files";
pub const ELASTICSEARCH_MAX_SIZE: i64 = 10000;
pub const ELASTICSEARCH_PIT_KEEP_ALIVE: &str = "1m";
pub const ELASTICSEARCH_BATCH_SIZE: usize = 100; // make into setting

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
    /// Fields for image files
    #[serde(flatten)]
    pub image_data: ImageData,
}

impl PartialEq for FileES {
    fn eq(&self, other: &Self) -> bool {
        self._id == other._id
    }
}
impl Eq for FileES {}

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
