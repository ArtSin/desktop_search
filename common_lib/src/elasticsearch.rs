use std::{path::PathBuf, str::FromStr};

use chrono::{
    serde::{ts_seconds, ts_seconds_option},
    DateTime, Utc,
};
use derive_more::Display;
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
    pub hash: Option<String>,
    /// MIME content type
    pub content_type: String,
    /// Type part of content type
    pub content_type_mime_type: String,
    /// Essence part of content type
    pub content_type_mime_essence: String,
    /// Text content
    pub content: Option<String>,
    /// Fields for text files
    #[serde(flatten)]
    pub text_data: TextData,
    /// Fields for image files
    #[serde(flatten)]
    pub image_data: ImageData,
    /// Fields for multimedia files
    #[serde(flatten)]
    pub multimedia_data: MultimediaData,
    /// Fields for document files
    #[serde(flatten)]
    pub document_data: DocumentData,
}

/// Fields for text files
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextData {
    /// MiniLM embedding of text
    pub text_embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, Serialize, Deserialize)]
pub enum ResolutionUnit {
    #[display(fmt = "Inch")]
    Inch,
    #[display(fmt = "cm")]
    Cm,
}

impl FromStr for ResolutionUnit {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Inch" => Ok(Self::Inch),
            "cm" => Ok(Self::Cm),
            _ => Err(anyhow::anyhow!("Unknown resolution unit")),
        }
    }
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
    /// Resolution unit (inches or centimeters)
    pub resolution_unit: Option<ResolutionUnit>,
    /// X resolution in pixels per unit
    pub x_resolution: Option<f32>,
    /// Y resolution in pixels per unit
    pub y_resolution: Option<f32>,
    /// F-number
    pub f_number: Option<f32>,
    /// Focal length of the lens in millimeters
    pub focal_length: Option<f32>,
    /// Exposure time in seconds
    pub exposure_time: Option<f32>,
    /// Did the flash fire?
    pub flash_fired: Option<bool>,
    /// Camera manufacturer
    pub image_make: Option<String>,
    /// Camera model
    pub image_model: Option<String>,
    /// Software/firmware name/version
    pub image_software: Option<String>,
}

impl FileMetadata for ImageData {
    fn any_metadata(&self) -> bool {
        self.width.is_some()
            || self.height.is_some()
            || self.resolution_unit.is_some()
            || self.x_resolution.is_some()
            || self.y_resolution.is_some()
            || self.f_number.is_some()
            || self.focal_length.is_some()
            || self.exposure_time.is_some()
            || self.flash_fired.is_some()
            || self.image_make.is_some()
            || self.image_model.is_some()
            || self.image_software.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, Serialize, Deserialize)]
pub enum AudioChannelType {
    #[display(fmt = "Mono")]
    Mono,
    #[display(fmt = "Stereo")]
    Stereo,
    #[display(fmt = "5.1")]
    _5_1,
    #[display(fmt = "7.1")]
    _7_1,
    #[display(fmt = "16 Channel")]
    _16,
    #[display(fmt = "Other")]
    Other,
}

impl FromStr for AudioChannelType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Mono" => Ok(Self::Mono),
            "Stereo" => Ok(Self::Stereo),
            "5.1" => Ok(Self::_5_1),
            "7.1" => Ok(Self::_7_1),
            "16 Channel" => Ok(Self::_16),
            "Other" => Ok(Self::Other),
            _ => Err(anyhow::anyhow!("Unknown audio channel type")),
        }
    }
}

/// Fields for multimedia files
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MultimediaData {
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub track_number: Option<String>,
    pub disc_number: Option<String>,
    pub release_date: Option<String>,
    /// Duration in seconds
    pub duration: Option<f32>,
    pub audio_sample_rate: Option<u32>,
    pub audio_channel_type: Option<AudioChannelType>,
}

impl FileMetadata for MultimediaData {
    fn any_metadata(&self) -> bool {
        self.artist.is_some()
            || self.album.is_some()
            || self.genre.is_some()
            || self.track_number.is_some()
            || self.disc_number.is_some()
            || self.release_date.is_some()
            || self.duration.is_some()
            || self.audio_sample_rate.is_some()
            || self.audio_channel_type.is_some()
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
