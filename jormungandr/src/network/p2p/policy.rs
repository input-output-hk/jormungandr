use jormungandr_lib::time::Duration;
use lru::LruCache;
use poldercast::{Address, Node, PolicyReport};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Duration as StdDuration;
use tracing::{span, Level, Span};

/// default quarantine duration is 10min
const DEFAULT_QUARANTINE_DURATION: StdDuration = StdDuration::from_secs(10 * 60);

/// default max quarantine is 2 days
const DEFAULT_MAX_QUARANTINE_DURATION: StdDuration = StdDuration::from_secs(2 * 24 * 3600);

/// default number of records is 24_000
const DEFAULT_MAX_NUM_QUARANTINE_RECORDS: usize = 24_000;

/// This is the P2P policy. Right now it is very similar to the default policy
/// defined in `poldercast` crate.
///
#[derive(Debug)]
pub struct Policy {
    quarantine_duration: StdDuration,
    max_quarantine: StdDuration,
    records: LruCache<Address, Records>,
    quarantine_whitelist: HashSet<Address>,
    span: Span,
}

pub struct Records {
    /// record the number of time the given node has been quarantined
    /// in known time.
    quarantine: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PolicyConfig {
    quarantine_duration: Duration,
    #[serde(default)]
    max_quarantine: Option<Duration>,
    #[serde(default)]
    max_num_quarantine_records: Option<usize>,
    #[serde(default)]
    quarantine_whitelist: HashSet<Address>,
}

impl Policy {
    pub fn new(pc: PolicyConfig, span: Span) -> Self {
        Self {
            quarantine_duration: pc.quarantine_duration.into(),
            max_quarantine: pc
                .max_quarantine
                .unwrap_or_else(|| DEFAULT_MAX_QUARANTINE_DURATION.into())
                .into(),
            records: LruCache::new(
                pc.max_num_quarantine_records
                    .unwrap_or(DEFAULT_MAX_NUM_QUARANTINE_RECORDS),
            ),
            quarantine_whitelist: pc.quarantine_whitelist,
            span,
        }
    }

    fn quarantine_duration_for(&mut self, id: Address) -> StdDuration {
        if let Some(r) = self.records.get_mut(&id) {
            r.quarantine_for(self.quarantine_duration, self.max_quarantine)
        } else {
            let r = Records::new();
            let t = r.quarantine_for(self.quarantine_duration, self.max_quarantine);
            self.records.put(id, r);
            t
        }
    }

    fn update(&mut self, id: Address) {
        if let Some(r) = self.records.get_mut(&id) {
            r.update();
        } else {
            let r = Records::new();
            self.records.put(id, r);
        }
    }
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            quarantine_duration: Duration::from(DEFAULT_QUARANTINE_DURATION),
            max_quarantine: Some(Duration::from(DEFAULT_MAX_QUARANTINE_DURATION)),
            max_num_quarantine_records: Some(DEFAULT_MAX_NUM_QUARANTINE_RECORDS),
            quarantine_whitelist: HashSet::new(),
        }
    }
}

impl Records {
    fn new() -> Records {
        Self { quarantine: 0 }
    }

    fn update(&mut self) {
        self.quarantine += 1;
    }

    fn quarantine_for(
        &self,
        quarantine_instant: StdDuration,
        max_quarantine: StdDuration,
    ) -> StdDuration {
        std::cmp::max(
            quarantine_instant
                .checked_mul(self.quarantine)
                .unwrap_or(max_quarantine),
            max_quarantine,
        )
    }
}

impl poldercast::Policy for Policy {
    fn check(&mut self, node: &mut Node) -> PolicyReport {
        let id = node.address().to_string();
        let span = span!(parent: &self.span, Level::TRACE, "policy check", id = %id);
        let _enter = span.enter();
        let node_address = node.address();
        // if the node is already quarantined
        if let Some(since) = node.logs().quarantined() {
            let duration = since.elapsed().unwrap();
            let quarantine_duration = self.quarantine_duration_for(node.address().clone());

            if duration < quarantine_duration {
                // the node still need to do some quarantine time
                PolicyReport::None
            } else if node.logs().last_update().elapsed().unwrap() < self.quarantine_duration {
                // the node has been quarantined long enough, check if it has been updated
                // while being quarantined (i.e. the node is still up and advertising itself
                // or others are still gossiping about it.)

                // the fact that this `Policy` does clean the records is a policy choice.
                // one could prefer to keep the record longers for future `check`.
                node.record_mut().clean_slate();
                tracing::debug!("lifting quarantine");
                PolicyReport::LiftQuarantine
            } else {
                // it appears the node was quarantine and is no longer active or gossiped
                // about, so we can forget it
                tracing::debug!("forgetting about the node");
                PolicyReport::Forget
            }
        } else if node.record().is_clear() {
            // if the record is clear, do nothing, leave the Node in the available nodes
            PolicyReport::None
        } else if self.quarantine_whitelist.contains(node_address) {
            // if the node is whitelisted
            tracing::debug!(
                "node is whitelisted, peer_addr: {}",
                node_address.to_string()
            );
            PolicyReport::None
        } else {
            // if the record is not `clear` then we quarantine the block for some time
            tracing::debug!("move node to quarantine");
            self.update(node.address().clone());
            PolicyReport::Quarantine
        }
    }
}
