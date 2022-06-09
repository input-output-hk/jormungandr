use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct PoolMaxEntries(usize);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct LogMaxEntries(usize);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PersistentLog {
    pub dir: PathBuf,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Mempool {
    /// maximum number of entries in the mempool
    #[serde(default)]
    pub pool_max_entries: PoolMaxEntries,
    /// maximum number of entries in the fragment logs
    #[serde(default)]
    pub log_max_entries: LogMaxEntries,
    /// path to the persistent log of all incoming fragments
    #[serde(default)]
    pub persistent_log: Option<PersistentLog>,
}

impl Default for PoolMaxEntries {
    fn default() -> Self {
        PoolMaxEntries(10_000)
    }
}

impl Default for LogMaxEntries {
    fn default() -> Self {
        LogMaxEntries(100_000)
    }
}

impl From<usize> for PoolMaxEntries {
    fn from(s: usize) -> Self {
        PoolMaxEntries(s)
    }
}

impl From<PoolMaxEntries> for usize {
    fn from(s: PoolMaxEntries) -> Self {
        s.0
    }
}

impl From<usize> for LogMaxEntries {
    fn from(s: usize) -> Self {
        LogMaxEntries(s)
    }
}

impl From<LogMaxEntries> for usize {
    fn from(s: LogMaxEntries) -> Self {
        s.0
    }
}
