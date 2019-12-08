use crate::blockchain::{Ref, Storage};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub enum ComparisonResult {
    PreferCurrent,
    PreferCandidate,
}

/// chose which of the two Ref is the most interesting to keep as a branch
///
/// i.e. if the two Ref points to the same block date: this allows to make a choice
/// as to which Ref ought to be our preferred choice for a tip. Being two ref
/// on the same block date is to a requirement to call this function as it will still
/// work to make a choice as to which of these two Ref is the right choice.
///
pub fn compare_against(storage: &Storage, current: &Ref, candidate: &Ref) -> ComparisonResult {
    let epoch_stability_depth = current.epoch_ledger_parameters().epoch_stability_depth;

    let rollback_possible =
        check_rollback_up_to(epoch_stability_depth, storage, current, candidate);

    if rollback_possible && current.chain_length() < candidate.chain_length() {
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
