use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::elasticsearch::FileES;

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: QueryType,
    pub modified_from: Option<DateTime<Utc>>,
    pub modified_to: Option<DateTime<Utc>>,
    pub size_from: Option<u64>,
    pub size_to: Option<u64>,

    /// Fields for image files
    pub image_data: ImageSearchRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    Text(TextQuery),
    Image(ImageQuery),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextQuery {
    pub query: String,
    pub image_search_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageQuery {
    pub image_path: PathBuf,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSearchRequest {
    pub width_from: Option<u32>,
    pub width_to: Option<u32>,
    pub height_from: Option<u32>,
    pub height_to: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<FileES>,
}
