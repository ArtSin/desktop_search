use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub elasticsearch_url: Url,
    pub tika_url: Url,
    pub nnserver_url: Url,
    pub indexing_directories: Vec<PathBuf>,
    pub max_file_size: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            elasticsearch_url: Url::parse("http://127.0.0.1:9200").unwrap(),
            tika_url: Url::parse("http://127.0.0.1:9998").unwrap(),
            nnserver_url: Url::parse("http://127.0.0.1:10000").unwrap(),
            indexing_directories: Vec::new(),
            max_file_size: 50 * 1024 * 1024, // 50 MiB
        }
    }
}
