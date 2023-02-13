use std::{fmt::Display, mem::take, time::Duration};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexingEvent {
    Started,
    DiffFailed(String),
    DiffCalculated {
        to_add: usize,
        to_remove: usize,
        to_update: usize,
    },
    FileProcessed,
    FilesSent(usize),
    Error(String),
    Finished(Duration),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexingStatusData {
    pub to_add: usize,
    pub to_remove: usize,
    pub to_update: usize,
    pub processed: usize,
    pub sent: usize,
    pub duration: Option<Duration>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexingStatus {
    NotStarted,
    DiffFailed(String),
    CalculatingDiff,
    Indexing(IndexingStatusData),
    Finished(IndexingStatusData),
}

impl Display for IndexingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotStarted | Self::Finished(_) => writeln!(f, "индексация не идёт"),
            Self::DiffFailed(e) => writeln!(
                f,
                "не удалось вычислить разность между файловой системой и индексом: {e}"
            ),
            Self::CalculatingDiff => {
                writeln!(f, "вычисление разности между файловой системой и индексом")
            }
            Self::Indexing(_) => writeln!(f, "идёт индексация"),
        }
    }
}

impl IndexingStatus {
    pub fn can_start(&self) -> bool {
        !matches!(self, Self::CalculatingDiff | Self::Indexing(_))
    }

    pub fn process_event(&mut self, event: IndexingEvent) {
        match event {
            IndexingEvent::Started => *self = Self::CalculatingDiff,
            IndexingEvent::DiffFailed(e) => *self = Self::DiffFailed(e),
            IndexingEvent::DiffCalculated {
                to_add,
                to_remove,
                to_update,
            } => {
                *self = Self::Indexing(IndexingStatusData {
                    to_add,
                    to_remove,
                    to_update,
                    ..Default::default()
                })
            }
            IndexingEvent::FileProcessed => match self {
                Self::Indexing(data) => {
                    data.processed += 1;
                }
                _ => unreachable!(),
            },
            IndexingEvent::FilesSent(cnt) => match self {
                Self::Indexing(data) => {
                    data.sent += cnt;
                }
                _ => unreachable!(),
            },
            IndexingEvent::Error(e) => match self {
                Self::Indexing(data) => {
                    data.errors.push(e);
                }
                _ => unreachable!(),
            },
            IndexingEvent::Finished(duration) => {
                *self = match self {
                    Self::Indexing(data) => {
                        let mut tmp = take(data);
                        tmp.duration = Some(duration);
                        Self::Finished(tmp)
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexStats {
    pub doc_cnt: u64,
    pub index_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexingWSMessage {
    IndexingStatus(IndexingStatus),
    IndexingEvent(IndexingEvent),
    IndexStats(IndexStats),
    Error(String),
}

impl From<IndexingStatus> for IndexingWSMessage {
    fn from(value: IndexingStatus) -> Self {
        Self::IndexingStatus(value)
    }
}
impl From<IndexingEvent> for IndexingWSMessage {
    fn from(value: IndexingEvent) -> Self {
        Self::IndexingEvent(value)
    }
}
impl From<IndexStats> for IndexingWSMessage {
    fn from(value: IndexStats) -> Self {
        Self::IndexStats(value)
    }
}
impl From<String> for IndexingWSMessage {
    fn from(value: String) -> Self {
        Self::Error(value)
    }
}
