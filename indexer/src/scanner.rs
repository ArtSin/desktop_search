use std::{cmp::Eq, collections::HashSet, hash::Hash, path::PathBuf};

use chrono::{DateTime, Utc};
use common_lib::{
    elasticsearch::{
        FileES, ELASTICSEARCH_INDEX, ELASTICSEARCH_MAX_SIZE, ELASTICSEARCH_PIT_KEEP_ALIVE,
    },
    settings::Settings,
};
use elasticsearch::{Elasticsearch, SearchParts};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tracing_unwrap::{OptionExt, ResultExt};
use walkdir::WalkDir;

/// Struct with file path and data to determine if file has been modified
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// ID of document (in Elasticsearch)
    pub _id: Option<String>,
    /// Absolute path to file
    pub path: PathBuf,
    /// Last modification time
    pub modified: DateTime<Utc>,
    /// Size of file in bytes
    pub size: u64,
}

impl From<FileES> for FileInfo {
    fn from(x: FileES) -> Self {
        Self {
            _id: x._id,
            path: x.path,
            modified: x.modified,
            size: x.size,
        }
    }
}

impl TryFrom<FileInfo> for FileES {
    type Error = std::io::Error;

    fn try_from(x: FileInfo) -> Result<Self, Self::Error> {
        tracing::debug!("Calculating hash of file: {}", x.path.display());
        let file = match std::fs::read(&x.path) {
            Ok(x) => x,
            Err(e) => {
                tracing::error!("Error reading file: {}", e);
                return Err(e);
            }
        };
        let hash_bytes: [u8; 32] = Sha256::digest(file).into();
        let hash = base16ct::lower::encode_string(&hash_bytes);

        Ok(Self {
            _id: x._id,
            path: x.path,
            modified: x.modified,
            size: x.size,
            hash,
            content_type: String::new(),
            content_type_mime_type: String::new(),
            content_type_mime_essence: String::new(),
            content: None,
            text_data: Default::default(),
            image_data: Default::default(),
            document_data: Default::default(),
        })
    }
}

impl FileInfo {
    /// Can file be indexed with current settings
    fn can_index(&self, settings: &Settings) -> bool {
        self.size <= settings.max_file_size
    }

    /// Checks if file was modified.
    /// Checks last modification time, then size
    fn is_modified(&self, new: &FileInfo) -> bool {
        self.modified.timestamp() != new.modified.timestamp() || self.size != new.size
    }
}

impl PartialEq for FileInfo {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}
impl Eq for FileInfo {}
impl Hash for FileInfo {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path.hash(state)
    }
}

/// Difference between two lists of files
pub struct FilesDiff {
    /// Files present only in new list
    pub added: Vec<FileInfo>,
    /// Files present only in old list
    pub removed: Vec<FileInfo>,
    /// Different files with same paths present in both lists
    pub modified: Vec<(FileInfo, FileInfo)>,
}

impl FilesDiff {
    /// Calculates difference
    pub fn from_vec(old: Vec<FileInfo>, new: Vec<FileInfo>) -> Self {
        let old_hs: HashSet<_> = old.into_iter().collect();
        let new_hs: HashSet<_> = new.into_iter().collect();
        FilesDiff {
            added: new_hs.difference(&old_hs).cloned().collect(),
            removed: old_hs.difference(&new_hs).cloned().collect(),
            modified: old_hs
                .intersection(&new_hs)
                .map(|x| {
                    (
                        old_hs.get(x).unwrap_or_log().clone(),
                        new_hs.get(x).unwrap_or_log().clone(),
                    )
                })
                .filter(|(x, y)| x.is_modified(y))
                .collect(),
        }
    }
}

/// Recursively iterates list of directories and returns indexable files.
/// Inaccessible files are skipped
pub fn get_file_system_files_list(settings: &Settings) -> Vec<FileInfo> {
    settings
        .indexing_directories
        .iter()
        .flat_map(|dir| {
            WalkDir::new(dir).into_iter().filter_map(|entry_res| {
                let entry = match entry_res {
                    Ok(x) => x,
                    Err(e) => {
                        tracing::error!("Error while scanning file system: {}", e);
                        return None;
                    }
                };

                let path = entry.path();
                tracing::debug!("Scanning path: {}", path.display());
                if !entry.file_type().is_file() {
                    return None;
                }

                let metadata = match std::fs::metadata(path) {
                    Ok(x) => x,
                    Err(e) => {
                        tracing::error!("Error getting file metadata: {}", e);
                        return None;
                    }
                };

                let file_info = FileInfo {
                    _id: None,
                    path: path.to_path_buf(),
                    modified: metadata.modified().unwrap_or_log().into(),
                    size: metadata.len(),
                };

                file_info.can_index(settings).then_some(file_info)
            })
        })
        .collect()
}

/// Returns all files from Elasticsearch index
pub async fn get_elasticsearch_files_list(
    es_client: &Elasticsearch,
) -> Result<Vec<FileInfo>, elasticsearch::Error> {
    #[allow(clippy::upper_case_acronyms)]
    #[derive(Serialize, Deserialize)]
    struct PIT {
        id: String,
    }

    #[derive(Serialize)]
    struct RequestBody {
        _source: Value,
        query: Value,
        pit: Value,
        sort: Vec<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        search_after: Option<Vec<Value>>,
    }

    let mut pit: PIT = es_client
        .open_point_in_time(elasticsearch::OpenPointInTimeParts::Index(&[
            ELASTICSEARCH_INDEX,
        ]))
        .keep_alive(ELASTICSEARCH_PIT_KEEP_ALIVE)
        .send()
        .await?
        .json()
        .await?;
    let mut search_after = None;
    let mut files = Vec::new();

    loop {
        let response: Value = es_client
            .search(SearchParts::None)
            .size(ELASTICSEARCH_MAX_SIZE)
            .track_total_hits(false)
            .body(RequestBody {
                _source: json!({
                    "excludes": ["content", "text_embedding", "image_embedding"]
                }),
                query: json!({
                    "match_all": {}
                }),
                pit: json!({
                    "id": pit.id,
                    "keep_alive": ELASTICSEARCH_PIT_KEEP_ALIVE
                }),
                sort: vec![json!({"_shard_doc": "asc"})],
                search_after,
            })
            .send()
            .await?
            .json()
            .await?;

        let hits = response["hits"]["hits"].as_array().unwrap_or_log();
        if hits.is_empty() {
            break;
        }
        pit.id = response["pit_id"].as_str().unwrap_or_log().to_owned();
        search_after = hits.last().unwrap_or_log()["sort"].as_array().cloned();
        let mut new_files: Vec<FileInfo> = hits
            .iter()
            .map(|x| {
                let mut val = x["_source"].to_owned();
                val["_id"] = x["_id"].to_owned();
                serde_json::from_value::<FileES>(val).unwrap_or_log().into()
            })
            .collect();
        files.append(&mut new_files);
    }
    es_client.close_point_in_time().body(pit).send().await?;

    Ok(files)
}
