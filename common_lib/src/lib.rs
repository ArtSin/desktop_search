use std::fmt::Display;

use serde::{Deserialize, Serialize};

pub mod actions;
pub mod elasticsearch;
pub mod search;
pub mod settings;
pub mod status;

#[cfg(not(target_arch = "wasm32"))]
pub mod embeddings;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexingStatus {
    Indexing,
    Finished,
    Error(String),
}

impl Display for IndexingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Indexing => write!(f, "Идёт индексация"),
            Self::Finished => write!(f, "Индексация не идёт"),
            Self::Error(e) => write!(f, "❌ Ошибка индексации: {}", e),
        }
    }
}

impl IndexingStatus {
    pub fn can_start(&self) -> bool {
        *self != Self::Indexing
    }

    pub fn add_error(&mut self, e: anyhow::Error) {
        *self = match self {
            Self::Error(old_e) => Self::Error(format!("{}\n{}", old_e, e)),
            _ => Self::Error(e.to_string()),
        };
    }
}
