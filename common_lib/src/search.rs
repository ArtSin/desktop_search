use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use uuid::Uuid;

use crate::elasticsearch::{AudioChannelType, FileES, ResolutionUnit};

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub page: u32,
    pub query: QueryType,
    pub content_type: Option<Vec<ContentTypeRequestItem>>,
    pub path_enabled: bool,
    pub hash_enabled: bool,
    pub modified_from: Option<DateTime<Utc>>,
    pub modified_to: Option<DateTime<Utc>>,
    pub size_from: Option<u64>,
    pub size_to: Option<u64>,

    /// Fields for image files
    pub image_data: ImageSearchRequest,
    /// Fields for multimedia files
    pub multimedia_data: MultimediaSearchRequest,
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
    pub text_search_enabled: bool,
    pub image_search_enabled: bool,
    pub text_search_pages: u32,
    pub image_search_pages: u32,
    pub query_coeff: f64,
    pub text_search_coeff: f64,
    pub image_search_coeff: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageQuery {
    pub image_path: PathBuf,
    pub image_search_pages: u32,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSearchRequest {
    pub image_make_enabled: bool,
    pub image_model_enabled: bool,
    pub image_software_enabled: bool,
    pub width_from: Option<u32>,
    pub width_to: Option<u32>,
    pub height_from: Option<u32>,
    pub height_to: Option<u32>,
    pub resolution_unit: ResolutionUnit,
    pub x_resolution_from: Option<f32>,
    pub x_resolution_to: Option<f32>,
    pub y_resolution_from: Option<f32>,
    pub y_resolution_to: Option<f32>,
    pub f_number_from: Option<f32>,
    pub f_number_to: Option<f32>,
    pub focal_length_from: Option<f32>,
    pub focal_length_to: Option<f32>,
    pub exposure_time_from: Option<f32>,
    pub exposure_time_to: Option<f32>,
    pub flash_fired: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultimediaSearchRequest {
    pub artist_enabled: bool,
    pub album_enabled: bool,
    pub genre_enabled: bool,
    pub track_number_enabled: bool,
    pub disc_number_enabled: bool,
    pub release_date_enabled: bool,
    pub duration_min_from: Option<f32>,
    pub duration_min_to: Option<f32>,
    pub audio_sample_rate_from: Option<u32>,
    pub audio_sample_rate_to: Option<u32>,
    pub audio_channel_type: Option<AudioChannelType>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentTypeRequestItem {
    IncludeType {
        type_: String,
    },
    IncludeSubtypes {
        subtypes: Vec<String>,
    },
    ExcludeType {
        type_: String,
    },
    ExcludeSubtypes {
        type_: String,
        subtypes: Vec<String>,
    },
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightedFields {
    pub path: String,
    pub hash: Option<String>,
    pub content: Option<String>,
    /// Fields for image files
    pub image_data: ImageHighlightedFields,
    /// Fields for multimedia files
    pub multimedia_data: MultimediaHighlightedFields,
    /// Fields for document files
    pub document_data: DocumentHighlightedFields,
}

/// Fields for image files
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageHighlightedFields {
    pub image_make: Option<String>,
    pub image_model: Option<String>,
    pub image_software: Option<String>,
}

/// Fields for multimedia files
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultimediaHighlightedFields {
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub track_number: Option<String>,
    pub disc_number: Option<String>,
    pub release_date: Option<String>,
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
    pub suggestion: Option<(String, String)>,
}
