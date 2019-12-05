use crate::time::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PoolMaxEntries(usize);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Mempool {
    /// maximum number of entries in the mempool
    #[serde(default)]
    pub pool_max_entries: PoolMaxEntries,
    /// time to live in the mempool before being discarded. If the value is not applied
    /// in a block within this duration it will be discarded.
    pub fragment_ttl: Duration,
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

impl Default for Mempool {
    fn default() -> Self {
        Mempool {
            pool_max_entries: PoolMaxEntries::default(),
            fragment_ttl: Duration::new(30 * 60, 0),
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
