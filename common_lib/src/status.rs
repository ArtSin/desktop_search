use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexStats {
    pub doc_cnt: u64,
    pub index_size: u64,
}
