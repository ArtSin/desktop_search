use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use uuid::Uuid;

use crate::elasticsearch::FileES;

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub page: u32,
    pub query: QueryType,
    pub path_enabled: bool,
    pub hash_enabled: bool,
    pub modified_from: Option<DateTime<Utc>>,
    pub modified_to: Option<DateTime<Utc>>,
    pub size_from: Option<u64>,
    pub size_to: Option<u64>,

    /// Fields for image files
    pub image_data: ImageSearchRequest,
    /// Fields for document files
    pub document_data: DocumentSearchRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    Text(TextQuery),
    Image(ImageQuery),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextQuery {
    pub query: String,
    pub content_enabled: bool,
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

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSearchRequest {
    pub title_enabled: bool,
    pub creator_enabled: bool,
    pub doc_created_from: Option<DateTime<Utc>>,
    pub doc_created_to: Option<DateTime<Utc>>,
    pub doc_modified_from: Option<DateTime<Utc>>,
    pub doc_modified_to: Option<DateTime<Utc>>,
    pub num_pages_from: Option<u32>,
    pub num_pages_to: Option<u32>,
    pub num_words_from: Option<u32>,
    pub num_words_to: Option<u32>,
    pub num_characters_from: Option<u32>,
    pub num_characters_to: Option<u32>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightedFields {
    pub path: String,
    pub hash: String,
    pub content: Option<String>,
    /// Fields for document files
    pub document_data: DocumentHighlightedFields,
}

/// Fields for document files
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentHighlightedFields {
    pub title: Option<String>,
    pub creator: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub file: FileES,
    pub highlights: HighlightedFields,
    pub id: Uuid,
}

impl PartialEq for SearchResult {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for SearchResult {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PageType {
    First,
    Previous(u32),
    Next(u32),
    Last(u32),
    Current(u32),
    Other(u32),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub pages: Vec<PageType>,
}
