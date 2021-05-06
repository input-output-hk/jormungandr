use crate::network::p2p::Address;
use crate::topology::{NodeId, PeerInfo};
use jormungandr_lib::time::Duration;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::{Duration as StdDuration, Instant, SystemTime};
use tracing::instrument;

/// default quarantine duration is 10min
const DEFAULT_QUARANTINE_DURATION: StdDuration = StdDuration::from_secs(10 * 60);

/// default max quarantine is 2 days
const DEFAULT_MAX_QUARANTINE_DURATION: StdDuration = StdDuration::from_secs(2 * 24 * 3600);

/// default number of records is 24_000
const DEFAULT_MAX_NUM_QUARANTINE_RECORDS: usize = 24_000;

#[derive(Debug, Clone)]
struct QuarantineRecord {
    peer_info: PeerInfo,
    quarantine_time: Instant,
}

/// Forgive nodes we demoted after some time
pub struct Quarantine {
    quarantine_duration: StdDuration,
    quarantine_whitelist: HashSet<Address>,
    /// To avoid cycling down nodes back and and forth from quarantine (and as such prevent them
    /// from being evicted from the lru cache), do not quarantine again nodes that were recently
    /// lifted from quarantine.
    ///
    /// A peer is inserted in the grace list after being lifted from quarantine and is removed
    /// from that list after we receive a new gossip about it.
    quarantine_grace: LruCache<NodeId, ()>,
    quarantined_records: LruCache<NodeId, QuarantineRecord>,
}

impl Quarantine {
    pub fn from_config(config: QuarantineConfig) -> Self {
        let max_num_quarantine_records = config
            .max_num_quarantine_records
            .unwrap_or(DEFAULT_MAX_NUM_QUARANTINE_RECORDS);
        Self {
            quarantine_duration: StdDuration::from(config.quarantine_duration),
            quarantine_whitelist: config
                .quarantine_whitelist
                .into_iter()
                .map(|addr| jormungandr_lib::multiaddr::to_tcp_socket_addr(&addr).unwrap())
                .collect(),
            quarantine_grace: LruCache::new(max_num_quarantine_records),
            quarantined_records: LruCache::new(max_num_quarantine_records),
        }
    }

    // Returns whether the node was quarantined or not
    #[instrument(skip(self), level = "trace")]
    pub fn quarantine_node(&mut self, mut node: PeerInfo) -> bool {
        if self.quarantine_whitelist.contains(&node.address) {
            tracing::debug!(
                node = %node.address,
                id=?node.id,
                "quarantine whitelists prevents this node from being quarantined",
            );
            false
        } else if self.quarantine_grace.contains(&node.id) {
            tracing::trace!(node = %node.address, id=?node.id, "not quarantining node in grace list");
            false
        } else {
            tracing::debug!(node = %node.address, id=?node.id, ?self.quarantine_duration, "quarantining node");
            node.quarantined = Some(SystemTime::now().into());
            self.quarantined_records.put(
                node.id.clone(),
                QuarantineRecord {
                    peer_info: node,
                    quarantine_time: Instant::now(),
                },
            );
            true
        }
    }

    pub fn record_new_gossip(&mut self, node: &NodeId) {
        self.quarantine_grace.pop(node);
    }

    pub fn quarantined_nodes(&self) -> Vec<PeerInfo> {
        self.quarantined_records
            .iter()
            .map(|(_, v)| v.peer_info.clone())
            .collect()
    }

    pub fn lift_from_quarantine(&mut self) -> Vec<PeerInfo> {
        let mut res = Vec::new();
        // This is basically a FIFO queue, a lru cache is being used just to
        // avoid keeping another data structure to know if an address was already quarantined
        while let Some((_id, record)) = self.quarantined_records.peek_lru() {
            if record.quarantine_time.elapsed() < self.quarantine_duration {
                break;
            }

            let (id, record) = self.quarantined_records.pop_lru().unwrap();
            self.quarantine_grace.put(id, ());
            res.push(record.peer_info);
        }

        res
    }
}

impl Default for Quarantine {
    fn default() -> Self {
        Self {
            quarantine_duration: DEFAULT_QUARANTINE_DURATION,
            quarantine_whitelist: HashSet::new(),
            quarantine_grace: LruCache::new(DEFAULT_MAX_NUM_QUARANTINE_RECORDS),
            quarantined_records: LruCache::new(DEFAULT_MAX_NUM_QUARANTINE_RECORDS),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct QuarantineConfig {
    quarantine_duration: Duration,
    #[serde(default)]
    max_quarantine: Option<Duration>,
    #[serde(default)]
    max_num_quarantine_records: Option<usize>,
    #[serde(default)]
    quarantine_whitelist: HashSet<multiaddr::Multiaddr>,
}

impl Default for QuarantineConfig {
    fn default() -> Self {
        Self {
            quarantine_duration: Duration::from(DEFAULT_QUARANTINE_DURATION),
            max_quarantine: Some(Duration::from(DEFAULT_MAX_QUARANTINE_DURATION)),
            max_num_quarantine_records: Some(DEFAULT_MAX_NUM_QUARANTINE_RECORDS),
            quarantine_whitelist: HashSet::new(),
        }
    }
}
