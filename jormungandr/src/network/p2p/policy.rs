use jormungandr_lib::time::Duration;
use poldercast::{Node, PolicyReport};
use serde::{Deserialize, Serialize};
use slog::Logger;

/// default quarantine duration is 30min
const DEFAULT_QUARANTINE_DURATION: std::time::Duration = std::time::Duration::from_secs(1800);

/// This is the P2P policy. Right now it is very similar to the default policy
/// defined in `poldercast` crate.
///
#[derive(Debug, Clone)]
pub struct Policy {
    quarantine_duration: std::time::Duration,

    logger: Logger,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PolicyConfig {
    quarantine_duration: Duration,
}

impl Policy {
    pub fn new(pc: PolicyConfig, logger: Logger) -> Self {
        Self {
            quarantine_duration: pc.quarantine_duration.into(),
            logger,
        }
    }
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            quarantine_duration: Duration::from(DEFAULT_QUARANTINE_DURATION),
        }
    }
}

impl poldercast::Policy for Policy {
    fn check(&mut self, node: &mut Node) -> PolicyReport {
        let id = node.id().to_string();
        let logger = self.logger.new(o!("id" => id));

        // if the node is already quarantined
        if let Some(since) = node.logs().quarantined() {
            let duration = since.elapsed().unwrap();

            if duration < self.quarantine_duration {
                // the node still need to do some quarantine time
                PolicyReport::None
            } else if node.logs().last_update().elapsed().unwrap() < self.quarantine_duration {
                // the node has been quarantined long enough, check if it has been updated
                // while being quarantined (i.e. the node is still up and advertising itself
                // or others are still gossiping about it.)

                // the fact that this `Policy` does clean the records is a policy choice.
                // one could prefer to keep the record longers for future `check`.
                node.record_mut().clean_slate();
                debug!(logger, "lifting quarantine");
                PolicyReport::LiftQuarantine
            } else {
                // it appears the node was quarantine and is no longer active or gossiped
                // about, so we can forget it
                debug!(logger, "forgetting about the node");
                PolicyReport::Forget
            }
        } else if node.record().is_clear() {
            // if the record is clear, do nothing, leave the Node in the available nodes
            PolicyReport::None
        } else {
            // if the record is not `clear` then we quarantine the block for some time
            debug!(logger, "move node to quarantine");
            PolicyReport::Quarantine
        }
    }
}
