use serde::{Deserialize, Serialize};

pub mod actions;
pub mod elasticsearch;
pub mod indexer;
pub mod search;
pub mod settings;

/// Should the request be batched?
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchRequest {
    #[serde(default = "BatchRequest::default_batched")]
    pub batched: bool,
}

impl BatchRequest {
    fn default_batched() -> bool {
        true
    }
}
