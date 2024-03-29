use std::{net::SocketAddr, path::PathBuf, str::FromStr};

use derive_more::Display;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub indexer_address: SocketAddr,
    pub elasticsearch_url: Url,
    pub tika_url: Url,
    pub nn_server_url: Url,
    pub open_on_start: bool,
    pub exclude_file_regex: String,
    pub watcher_enabled: bool,
    pub debouncer_timeout: f32,
    pub max_file_size: u64,
    pub max_concurrent_files: usize,
    pub elasticsearch_batch_size: usize,
    pub results_per_page: u32,
    pub knn_candidates_multiplier: u32,
    pub nn_server: NNServerSettings,
    pub indexing_directories: Vec<IndexingDirectory>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            indexer_address: "127.0.0.1:11000".parse().unwrap(),
            elasticsearch_url: Url::parse("http://127.0.0.1:9200").unwrap(),
            tika_url: Url::parse("http://127.0.0.1:9998").unwrap(),
            nn_server_url: Url::parse("http://127.0.0.1:10000").unwrap(),
            open_on_start: true,
            indexing_directories: Vec::new(),
            exclude_file_regex: r"[/\\]\.git[/\\]|\.pygtex$|\.pygstyle$|\.aux$|\.bbl$|\.bcf$|\.blg$|\.synctex\.gz$|\.toc$".to_owned(),
            watcher_enabled: true,
            debouncer_timeout: 5.0,
            max_file_size: 50 * 1024 * 1024, // 50 MiB
            max_concurrent_files: 32,
            elasticsearch_batch_size: 100,
            results_per_page: 20,
            knn_candidates_multiplier: 10,
            nn_server: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, Serialize, Deserialize)]
pub enum NNDevice {
    #[display(fmt = "cpu")]
    CPU,
    #[display(fmt = "cuda")]
    CUDA,
}

impl Default for NNDevice {
    fn default() -> Self {
        Self::CPU
    }
}

impl FromStr for NNDevice {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cpu" => Ok(Self::CPU),
            "cuda" => Ok(Self::CUDA),
            _ => Err(anyhow::anyhow!("Unknown device")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NNSettings {
    pub device: NNDevice,
    pub batch_size: usize,
    pub max_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NNServerSettings {
    pub nn_server_address: SocketAddr,
    pub text_search_enabled: bool,
    pub image_search_enabled: bool,
    pub reranking_enabled: bool,
    pub clip_image: NNSettings,
    pub clip_text: NNSettings,
    pub minilm_text: NNSettings,
    pub minilm_rerank: NNSettings,
    pub max_sentences: u32,
    pub window_size: u32,
    pub window_step: u32,
    pub summary_len: u32,
}

impl Default for NNServerSettings {
    fn default() -> Self {
        Self {
            nn_server_address: "127.0.0.1:10000".parse().unwrap(),
            text_search_enabled: true,
            image_search_enabled: true,
            reranking_enabled: true,
            clip_image: NNSettings {
                device: NNDevice::CUDA,
                batch_size: 16,
                max_delay_ms: 100,
            },
            clip_text: NNSettings {
                device: NNDevice::CUDA,
                batch_size: 32,
                max_delay_ms: 100,
            },
            minilm_text: NNSettings {
                device: NNDevice::CUDA,
                batch_size: 32,
                max_delay_ms: 100,
            },
            minilm_rerank: NNSettings {
                device: NNDevice::CUDA,
                batch_size: 8,
                max_delay_ms: 100,
            },
            max_sentences: 100,
            window_size: 100,
            window_step: 75,
            summary_len: 3,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexingDirectory {
    pub path: PathBuf,
    pub exclude: bool,
    pub watch: bool,
}
