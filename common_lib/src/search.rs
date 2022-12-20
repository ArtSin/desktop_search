use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::elasticsearch::FileES;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: QueryType,
    pub modified_from: Option<DateTime<Utc>>,
    pub modified_to: Option<DateTime<Utc>>,
    pub size_from: Option<u64>,
    pub size_to: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    Text(TextSearchRequest),
    Image(ImageSearchRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSearchRequest {
    pub query: String,
    pub image_search_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSearchRequest {
    pub image_path: PathBuf,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<FileES>,
}
