use jormungandr_lib::time::Duration;
use poldercast::{Node, PolicyReport};
use serde::{Deserialize, Serialize};
use slog::Logger;

/// default quarantine duration is 30min
const DEFAULT_QUARANTINE_DURATION: std::time::Duration = std::time::Duration::from_secs(1800);

// default stale duration is 5min
const DEFAULT_STALE_DURATION: std::time::Duration = std::time::Duration::from_secs(300);

/// This is the P2P policy. Right now it is very similar to the default policy
/// defined in `poldercast` crate.
///
#[derive(Debug, Clone)]
pub struct Policy {
    quarantine_duration: std::time::Duration,
    stale_duration: std::time::Duration,
    logger: Logger,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PolicyConfig {
    quarantine_duration: Duration,
    stale_duration: Duration,
}

impl Policy {
    pub fn new(pc: PolicyConfig, logger: Logger) -> Self {
        Self {
            quarantine_duration: pc.quarantine_duration.into(),
            stale_duration: pc.stale_duration.into(),
            logger,
        }
    }
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            quarantine_duration: Duration::from(DEFAULT_QUARANTINE_DURATION),
            stale_duration: Duration::from(DEFAULT_STALE_DURATION),
        }
    }
}

impl poldercast::Policy for Policy {
    fn check(&mut self, node: &mut Node) -> PolicyReport {
        let id = node.id().to_string();
        let logger = self.logger.new(o!("id" => id));

        if node.logs().last_update().elapsed().unwrap() > self.stale_duration {
            debug!(logger, "forgetting about the node (stale)");
            PolicyReport::Forget
        } else {
            match node.logs().quarantined() {
                Some(q) => {
                    if q.elapsed().unwrap() > self.quarantine_duration {
                        node.record_mut().clean_slate();
                        debug!(logger, "lifting quarantine");
                        PolicyReport::LiftQuarantine
                    } else {
                        PolicyReport::None
                    }
                }
                None => {
                    if !node.record().is_clear() {
                        debug!(logger, "moving node to quarantine");
                        PolicyReport::Quarantine
                    } else {
                        debug!(logger, "moving node to quarantine");
                        PolicyReport::None
                    }
                }
            }
        }
    }
}
