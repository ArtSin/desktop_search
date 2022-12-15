use serde::{Deserialize, Serialize};

use crate::elasticsearch::FileES;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<FileES>,
}
