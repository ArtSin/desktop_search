use std::sync::Arc;

use async_trait::async_trait;
use common_lib::{
    elasticsearch::{FileES, ImageData, ResolutionUnit},
    BatchRequest,
};
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

use crate::{
    embeddings::{get_image_search_image_embedding_generic, ImageEmbedding},
    thumbnails::get_thumbnail,
    ServerState,
};

use super::{Metadata, Parser};

#[serde_as]
#[derive(Default, Deserialize)]
pub struct ImageMetadata {
    /// Width in pixels
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(rename = "tiff:ImageWidth")]
    width: Option<u32>,
    /// Height in pixels
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(rename = "tiff:ImageLength")]
    height: Option<u32>,
    /// Resolution unit (inches or centimeters)
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(rename = "tiff:ResolutionUnit")]
    resolution_unit: Option<ResolutionUnit>,
    /// X resolution in pixels per unit
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(rename = "tiff:XResolution")]
    x_resolution: Option<f32>,
    /// Y resolution in pixels per unit
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(rename = "tiff:YResolution")]
    y_resolution: Option<f32>,
    /// F-number
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(rename = "exif:FNumber")]
    f_number: Option<f32>,
    /// Focal length of the lens in millimeters
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(rename = "exif:FocalLength")]
    focal_length: Option<f32>,
    /// Exposure time in seconds
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(rename = "exif:ExposureTime")]
    exposure_time: Option<f32>,
    /// Did the flash fire?
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(rename = "exif:Flash")]
    flash_fired: Option<bool>,
    /// Camera manufacturer
    #[serde(rename = "tiff:Make")]
    image_make: Option<String>,
    /// Camera model
    #[serde(rename = "tiff:Model")]
    image_model: Option<String>,
    /// Software/firmware name/version
    #[serde(rename = "tiff:Software")]
    image_software: Option<String>,
}

pub struct ImageParser;

#[async_trait]
impl Parser for ImageParser {
    fn is_supported_file(&self, metadata: &Metadata) -> bool {
        metadata.content_type.starts_with("image")
            || metadata.content_type.starts_with("audio")
            || metadata.content_type.starts_with("video")
    }

    async fn parse(
        &self,
        state: Arc<ServerState>,
        file: &mut FileES,
        metadata: &mut Metadata,
        file_bytes: &[u8],
    ) -> anyhow::Result<()> {
        tracing::debug!(
            "Calculating image embedding of file: {}",
            file.path.display()
        );

        let image_search_enabled = state.settings.read().await.other.image_search_enabled;
        let embedding = if image_search_enabled {
            let nnserver_url = state.settings.read().await.other.nnserver_url.clone();
            if metadata.content_type.starts_with("image") {
                get_image_search_image_embedding_generic(
                    &state.reqwest_client,
                    nnserver_url,
                    BatchRequest { batched: true },
                    file_bytes.to_vec(),
                )
                .await?
            } else {
                // Try to get thumbnail for audio/video files, ignore errors
                match get_thumbnail(&file.path.to_string_lossy(), &None).await {
                    Ok(thumbnail) => {
                        match get_image_search_image_embedding_generic(
                            &state.reqwest_client,
                            nnserver_url,
                            BatchRequest { batched: true },
                            thumbnail.0,
                        )
                        .await
                        {
                            Ok(res) => res,
                            Err(err) => {
                                tracing::debug!(
                                    "Error calculating embedding of thumbnail: {}",
                                    err
                                );
                                ImageEmbedding { embedding: None }
                            }
                        }
                    }
                    Err(err) => {
                        tracing::debug!("Error getting thumbnail of file: {}", err);
                        ImageEmbedding { embedding: None }
                    }
                }
            }
        } else {
            ImageEmbedding { embedding: None }
        };

        let data = std::mem::take(&mut metadata.image_data);
        file.image_data = ImageData {
            image_embedding: embedding.embedding,
            width: data.width,
            height: data.height,
            resolution_unit: data.resolution_unit,
            x_resolution: data.x_resolution,
            y_resolution: data.y_resolution,
            f_number: data.f_number,
            focal_length: data.focal_length,
            exposure_time: data.exposure_time,
            flash_fired: data.flash_fired,
            image_make: data.image_make,
            image_model: data.image_model,
            image_software: data.image_software,
        };
        Ok(())
    }
}
