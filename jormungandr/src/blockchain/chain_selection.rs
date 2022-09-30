use crate::blockchain::{Ref, Storage};
use std::time::Duration;

const ALLOWED_TIME_DISCREPANCY: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub enum ComparisonResult {
    PreferCurrent,
    PreferCandidate,
}

/// chose which of the two Ref is the most interesting to keep as a branch
///
/// i.e. if the two Ref points to the same block date: this allows to make a choice
/// as to which Ref ought to be our preferred choice for a tip.
pub fn compare_against(storage: &Storage, current: &Ref, candidate: &Ref) -> ComparisonResult {
    let epoch_stability_depth = current.ledger().settings().epoch_stability_depth;

    let rollback_possible =
        check_rollback_up_to(epoch_stability_depth, storage, current, candidate);

    // returns `true` if the candidate is set in what appears to be in the future
    // relative to this node, with a little buffer to accomodate for small inconsistencies
    // in time
    let in_future = match candidate.elapsed() {
        Err(duration) if duration.duration() > ALLOWED_TIME_DISCREPANCY => {
            tracing::debug!(
                "candidate block {} appear to be in the future by {}s, will not consider it for updating our current tip",
                candidate.header().description(),
                duration.duration().as_secs()
            );
            true
        }
        _ => false,
    };

    if rollback_possible && !in_future && current.chain_length() < candidate.chain_length() {
        ComparisonResult::PreferCandidate
    } else {
        ComparisonResult::PreferCurrent
    }
}

fn check_rollback_up_to(
    _epoch_stability_depth: u32,
    _: &Storage,
    _ref1: &Ref,
    _ref2: &Ref,
) -> bool {
    true
}
