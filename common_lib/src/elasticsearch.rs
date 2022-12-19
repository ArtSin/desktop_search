use std::path::PathBuf;

use chrono::{serde::ts_seconds, DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const ELASTICSEARCH_INDEX: &str = "files";
pub const ELASTICSEARCH_MAX_SIZE: i64 = 10000;
pub const ELASTICSEARCH_PIT_KEEP_ALIVE: &str = "1m";
pub const ELASTICSEARCH_BATCH_SIZE: usize = 100; // make into setting

/// File information as stored in Elasticsearch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileES {
    /// ID of document
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
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
    /// Fields for image files
    #[serde(flatten)]
    pub image_data: Option<ImageData>,
}

impl PartialEq for FileES {
    fn eq(&self, other: &Self) -> bool {
        self._id == other._id
    }
}
impl Eq for FileES {}

/// Fields for image files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageData {
    pub image_embedding: Option<Vec<f32>>,
}
