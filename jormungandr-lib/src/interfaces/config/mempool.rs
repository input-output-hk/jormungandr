use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct PoolMaxEntries(usize);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct LogMaxEntries(usize);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Mempool {
    /// maximum number of entries in the mempool
    #[serde(default)]
    pub pool_max_entries: PoolMaxEntries,
    /// maximum number of entries in the fragment logs
    #[serde(default)]
    pub log_max_entries: LogMaxEntries,
    /// path to the persistent log of all incoming fragments
    // FIXME: should be a struct like `persistent_log.dir`,
    // as we may want to add more options like rotation policy later
    #[serde(default)]
    pub persistent_log_dir: Option<PathBuf>,
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

impl Default for Mempool {
    fn default() -> Self {
        Mempool {
            pool_max_entries: PoolMaxEntries::default(),
            log_max_entries: LogMaxEntries::default(),
            persistent_log_dir: None,
        }
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
