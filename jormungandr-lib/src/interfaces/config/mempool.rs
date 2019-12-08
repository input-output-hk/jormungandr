use crate::time::Duration;
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
    /// time to live in the mempool before being discarded. If the value is not applied
    /// in a block within this duration it will be discarded.
    pub fragment_ttl: Duration,
    /// maximum number of entries in the fragment logs
    #[serde(default)]
    pub log_max_entries: LogMaxEntries,
    /// FragmentLog time to live, it is for information purposes, we log all the fragments
    /// logs in a cache. The log will be discarded at the end of the ttl.
    pub log_ttl: Duration,
    /// interval between 2 garbage collection check of the mempool and the log cache.
    pub garbage_collection_interval: Duration,
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
            fragment_ttl: Duration::new(30 * 60, 0),
            log_max_entries: LogMaxEntries::default(),
            log_ttl: Duration::new(3600, 0),
            garbage_collection_interval: Duration::new(3600 / 4, 0),
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
