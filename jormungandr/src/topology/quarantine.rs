/// This module handles reports agains other nodes in the topology.
///
/// Its purpose is to be the glue between Poldercast 'promote_peer' and 'demote_peer'
/// functions so that each call to 'demote_peer' is followed after some time (if appropriate)
/// by a call to 'promote_peer', not to ban a node forever from the topology.
///
/// It is also responsible for determining wheter a report is to be accounted for
/// according to the node configs.
use crate::network::p2p::Address;
use crate::topology::{NodeId, Peer, PeerInfo};
use jormungandr_lib::time::Duration;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    time::{Duration as StdDuration, Instant, SystemTime},
};

/// default quarantine duration is 10min
const DEFAULT_QUARANTINE_DURATION: StdDuration = StdDuration::from_secs(10 * 60);

/// default max quarantine is 2 days
const DEFAULT_MAX_QUARANTINE_DURATION: StdDuration = StdDuration::from_secs(2 * 24 * 3600);

/// default number of records is 24_000
const DEFAULT_MAX_NUM_QUARANTINE_RECORDS: usize = 24_000;

#[derive(Debug, Clone)]
struct ReportRecord {
    peer_info: PeerInfo,
    report_time: Instant,
}

pub enum ReportNodeStatus {
    Ok,
    Quarantine,
    SoftReport,
}

/// Forgive nodes we demoted after some time
pub struct ReportRecords {
    /// A report will be lifted after 'report_duration'
    report_duration: StdDuration,
    report_whitelist: HashSet<Address>,
    /// To avoid cycling down nodes back and and forth(and as such prevent them
    /// from being evicted from the lru cache), do not report again nodes that were recently
    /// lifted from a report.
    ///
    /// A peer is inserted in the grace list after the report is lifted and is removed
    /// from that list after we receive a new gossip about it.
    report_grace: LruCache<NodeId, ()>,
    report_records: LruCache<NodeId, ReportRecord>,
}

impl ReportRecords {
    pub fn from_config(config: QuarantineConfig) -> Self {
        let max_num_quarantine_records = config
            .max_num_quarantine_records
            .unwrap_or(DEFAULT_MAX_NUM_QUARANTINE_RECORDS);
        Self {
            report_duration: StdDuration::from(config.quarantine_duration),
            report_whitelist: config
                .quarantine_whitelist
                .into_iter()
                .map(|addr| jormungandr_lib::multiaddr::to_tcp_socket_addr(&addr).unwrap())
                .collect(),
            report_grace: LruCache::new(max_num_quarantine_records),
            report_records: LruCache::new(max_num_quarantine_records),
        }
    }

    /// Returns whether the node has been quarantined or not.
    pub fn report_node(
        &mut self,
        topology: &mut poldercast::Topology,
        node: Peer,
    ) -> ReportNodeStatus {
        if self.report_whitelist.contains(&node.address()) {
            tracing::debug!(
                node = %node.address(),
                id=%node.id(),
                "quarantine whitelists prevents this node from being reported",
            );
            ReportNodeStatus::Ok
        } else if self.report_grace.contains(&node.id()) {
            tracing::trace!(node = %node.address(), id=%node.id(), "not reporting node in grace list");
            ReportNodeStatus::Ok
        } else {
            let mut peer_info = PeerInfo::from(node);
            tracing::debug!(node = %peer_info.address, id=%peer_info.id, ?self.report_duration, "reporting node");
            // If we'll handle report reasons other that a connectivity issue in the future, we may want to
            // demote a peer all the way down to dirty in case of a serious violation.
            topology.remove_peer(peer_info.id.as_ref());

            let mut result = ReportNodeStatus::SoftReport;

            // Not all reports will quarantine a node (which is, put it in the dirty pool). For example,
            // a connectivity issue reported against a trusted peer will only demote it once, thus putting
            // it in the 'pool' pool.
            //
            // Nevertheless, demoted peers are removed from the view layers and no further contact will be
            // initiated by this node until we receive a new gossip about them.
            // For this reason, we keep track of those reports as well so that we will try to contact such
            // nodes again after some time if we haven't heard from them sooner (and avoid network splits).
            if topology.peers().dirty().contains(peer_info.id.as_ref()) {
                peer_info.quarantined = Some(SystemTime::now().into());
                tracing::debug!(node = %peer_info.address, id=%peer_info.id, "node has been quarantined");
                result = ReportNodeStatus::Quarantine;
            }

            self.report_records.put(
                peer_info.id,
                ReportRecord {
                    peer_info,
                    report_time: Instant::now(),
                },
            );

            result
        }
    }

    pub fn reported_nodes(&self) -> Vec<PeerInfo> {
        self.report_records
            .iter()
            .map(|(_, v)| v.peer_info.clone())
            .collect()
    }

    pub fn record_new_gossip(&mut self, node: &NodeId) {
        self.report_grace.pop(node);
    }

    pub fn lift_reports(&mut self) -> Vec<PeerInfo> {
        let mut res = Vec::new();
        // This is basically a FIFO queue, a lru cache is being used just to
        // avoid keeping another data structure to know if an address was already quarantined
        while let Some((_id, record)) = self.report_records.peek_lru() {
            if record.report_time.elapsed() < self.report_duration {
                break;
            }

            let (id, record) = self.report_records.pop_lru().unwrap();
            self.report_grace.put(id, ());
            res.push(record.peer_info);
        }

        res
    }
}

impl Default for ReportRecords {
    fn default() -> Self {
        Self {
            report_duration: DEFAULT_QUARANTINE_DURATION,
            report_whitelist: HashSet::new(),
            report_grace: LruCache::new(DEFAULT_MAX_NUM_QUARANTINE_RECORDS),
            report_records: LruCache::new(DEFAULT_MAX_NUM_QUARANTINE_RECORDS),
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
