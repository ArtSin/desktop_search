use std::sync::Arc;

use async_trait::async_trait;
use common_lib::elasticsearch::{AudioChannelType, FileES, FileMetadata, MultimediaData};
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

use crate::ServerState;

use super::{Metadata, Parser};

#[serde_as]
#[derive(Default, Deserialize)]
pub struct MultimediaMetadata {
    #[serde(rename = "xmpDM:artist")]
    artist: Option<String>,
    #[serde(rename = "xmpDM:album")]
    album: Option<String>,
    #[serde(rename = "xmpDM:genre")]
    genre: Option<String>,
    #[serde(rename = "xmpDM:trackNumber")]
    track_number: Option<String>,
    #[serde(rename = "xmpDM:discNumber")]
    disc_number: Option<String>,
    #[serde(rename = "xmpDM:releaseDate")]
    release_date: Option<String>,
    /// Duration in seconds
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(rename = "xmpDM:duration")]
    duration: Option<f32>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(rename = "xmpDM:audioSampleRate")]
    audio_sample_rate: Option<u32>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(rename = "xmpDM:audioChannelType")]
    audio_channel_type: Option<AudioChannelType>,
}

impl FileMetadata for MultimediaMetadata {
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

pub struct MultimediaParser;

#[async_trait]
impl Parser for MultimediaParser {
    fn is_supported_file(&self, metadata: &Metadata) -> bool {
        metadata.multimedia_data.any_metadata()
    }

    async fn parse(
        &self,
        _state: Arc<ServerState>,
        file: &mut FileES,
        metadata: &mut Metadata,
        _file_bytes: &[u8],
    ) -> anyhow::Result<()> {
        let data = std::mem::take(&mut metadata.multimedia_data);
        file.multimedia_data = MultimediaData {
            artist: data.artist,
            album: data.album,
            genre: data.genre,
            track_number: data.track_number,
            disc_number: data.disc_number,
            release_date: data.release_date,
            duration: data.duration,
            audio_sample_rate: data.audio_sample_rate,
            audio_channel_type: data.audio_channel_type,
        };
        Ok(())
    }
}
