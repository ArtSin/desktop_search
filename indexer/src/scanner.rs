use std::{cmp::Eq, collections::HashSet, hash::Hash, path::PathBuf};

use chrono::{serde::ts_seconds, DateTime, Utc};
use common_lib::{
    elasticsearch::{
        FileES, ELASTICSEARCH_INDEX, ELASTICSEARCH_MAX_SIZE, ELASTICSEARCH_PIT_KEEP_ALIVE,
    },
    settings::{IndexingDirectory, Settings},
};
use elasticsearch::{Elasticsearch, SearchParts};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tracing_unwrap::{OptionExt, ResultExt};
use walkdir::WalkDir;

/// Struct with file path and data to determine if file has been modified
#[derive(Debug, Clone, Deserialize)]
pub struct FileInfo {
    /// ID of document (in Elasticsearch)
    pub _id: Option<String>,
    /// Absolute path to file
    pub path: PathBuf,
    /// Last modification time
    #[serde(with = "ts_seconds")]
    pub modified: DateTime<Utc>,
    /// Size of file in bytes
    pub size: u64,
    /// Process contents or include only basic metadata
    #[serde(default = "FileInfo::default_process_contents")]
    pub process_contents: bool,
}

impl TryFrom<FileInfo> for FileES {
    type Error = std::io::Error;

    fn try_from(x: FileInfo) -> Result<Self, Self::Error> {
        let hash = x
            .process_contents
            .then(|| {
                tracing::debug!("Calculating hash of file: {}", x.path.display());
                let file = match std::fs::read(&x.path) {
                    Ok(x) => x,
                    Err(e) => {
                        tracing::error!("Error reading file: {}", e);
                        return Err(e);
                    }
                };
                let hash_bytes: [u8; 32] = Sha256::digest(file).into();
                Ok(base16ct::lower::encode_string(&hash_bytes))
            })
            .transpose()?;

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
            multimedia_data: Default::default(),
        })
    }
}

impl FileInfo {
    /// Create file info and check if file contents can be processed with current settings
    fn new(path: PathBuf, modified: DateTime<Utc>, size: u64, settings: &Settings) -> Self {
        Self {
            _id: None,
            path,
            modified,
            size,
            process_contents: size <= settings.max_file_size,
        }
    }

    fn default_process_contents() -> bool {
        true
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

fn file_info_from_path(settings: &Settings, path: PathBuf) -> Option<FileInfo> {
    tracing::debug!("Scanning path: {}", path.display());

    let metadata = match std::fs::metadata(&path) {
        Ok(x) => x,
        Err(e) => {
            tracing::error!("Error getting file metadata: {}", e);
            return None;
        }
    };
    if !metadata.file_type().is_file() {
        return None;
    }

    Some(FileInfo::new(
        path,
        metadata.modified().unwrap_or_log().into(),
        metadata.len(),
        settings,
    ))
}

pub fn process_indexable_files<T, F>(
    settings: &Settings,
    indexing_directories: &[IndexingDirectory],
    process: F,
    allow_errors: bool,
) -> anyhow::Result<Vec<T>>
where
    F: Fn(&Settings, PathBuf) -> Option<T>,
{
    let indexing_directories_hs: HashSet<_> = indexing_directories
        .iter()
        .map(|x| x.path.as_path())
        .collect();
    let exclude_file_regex = Regex::new(&settings.exclude_file_regex)?;

    Ok(indexing_directories
        .iter()
        .filter(|dir| !dir.exclude)
        .flat_map(|dir| {
            WalkDir::new(&dir.path)
                .into_iter()
                .filter_entry(|e| {
                    (e.path() == dir.path || !indexing_directories_hs.contains(e.path()))
                        && (!e.path().is_file()
                            || !exclude_file_regex.is_match(&e.path().to_string_lossy()))
                })
                .filter_map(|entry_res| {
                    let entry = match entry_res {
                        Ok(x) => x,
                        Err(e) => {
                            if allow_errors {
                                tracing::debug!("Error while scanning file system: {}", e);
                            } else {
                                tracing::error!("Error while scanning file system: {}", e);
                            }
                            return None;
                        }
                    };

                    process(settings, entry.into_path())
                })
        })
        .collect())
}

/// Recursively iterates list of directories and returns indexable files.
/// Inaccessible files are skipped
pub fn get_file_system_files_list(settings: &Settings) -> anyhow::Result<Vec<FileInfo>> {
    process_indexable_files(
        settings,
        &settings.indexing_directories,
        file_info_from_path,
        false,
    )
}

pub fn get_file_system_partial_files_list(
    settings: &Settings,
    paths: Vec<PathBuf>,
) -> anyhow::Result<Vec<FileInfo>> {
    process_indexable_files(
        settings,
        &paths
            .iter()
            .map(|path| IndexingDirectory {
                path: path.to_path_buf(),
                exclude: false,
            })
            .collect::<Vec<_>>(),
        file_info_from_path,
        true,
    )
}

/// Returns all files from Elasticsearch index
pub async fn get_elasticsearch_files_list(
    es_client: &Elasticsearch,
    paths: Option<&[PathBuf]>,
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
        let query = match paths {
            Some(paths) => json!({
                "terms": {
                    "path.keyword": paths
                }
            }),
            None => json!({
                "match_all": {}
            }),
        };

        let response: Value = es_client
            .search(SearchParts::None)
            .size(ELASTICSEARCH_MAX_SIZE)
            .track_total_hits(false)
            .body(RequestBody {
                _source: json!({
                    "includes": ["path", "modified", "size"]
                }),
                query,
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
                serde_json::from_value(val).unwrap_or_log()
            })
            .collect();
        files.append(&mut new_files);
        if paths.is_some() {
            break;
        }
    }
    es_client.close_point_in_time().body(pit).send().await?;

    Ok(files)
}
