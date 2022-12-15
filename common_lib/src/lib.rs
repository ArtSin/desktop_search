use std::error::Error;

use serde::{Deserialize, Serialize};

pub mod elasticsearch;
pub mod search;
pub mod settings;
pub mod status;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexingStatus {
    NotStarted,
    Indexing,
    Finished,
    Error(String),
}

impl IndexingStatus {
    pub fn can_start(&self) -> bool {
        *self != Self::Indexing
    }

    pub fn add_error(&mut self, e: Box<dyn Error>) {
        *self = match self {
            Self::Error(old_e) => Self::Error(format!("{}\n{}", old_e, e)),
            _ => Self::Error(e.to_string()),
        };
    }
}
