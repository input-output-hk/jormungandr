use crate::time::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Mempool {
    /// time to live in the mempool before being discarded. If the value is not applied
    /// in a block within this duration it will be discarded.
    pub fragment_ttl: Duration,
    /// FragmentLog time to live, it is for information purposes, we log all the fragments
    /// logs in a cache. The log will be discarded at the end of the ttl.
    pub log_ttl: Duration,
    /// interval between 2 garbage collection check of the mempool and the log cache.
    pub garbage_collection_interval: Duration,
}

impl Default for Mempool {
    fn default() -> Self {
        Mempool {
            fragment_ttl: Duration::new(30 * 60, 0),
            log_ttl: Duration::new(3600, 0),
            garbage_collection_interval: Duration::new(3600 / 4, 0),
        }
    }
}
